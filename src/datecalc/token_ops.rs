use super::token::Token;
use crate::datecalc::{date_to_sod, duration_to_date};
use crate::entry::Entry;
use crate::prelude::*;

use time::ext::NumericalDuration;
use time::macros::format_description;
use time::{OffsetDateTime, Time, UtcDateTime, UtcOffset};

impl Token {
    /// Check token precedence and if it's left associative.
    pub fn prec_and_assoc(&self) -> (u8, bool) {
        use Token::*;

        match self {
            Add(true) | Sub(true) => (10, false),
            FuzzyEq => (9, true),
            Not => (8, false),
            At => (7, true),
            Mul | Div | Mod => (6, true),
            Add(false) | Sub(false) => (5, true),
            Less(false) | LessEq(false) | Greater(false) | GreaterEq(false) => (4, true),
            Less(true) | LessEq(true) | Greater(true) | GreaterEq(true) => (4, false),
            Eq | NotEq => (3, true),
            And => (2, true),
            Or => (1, true),
            If | Else => (0, true),
            String(_) | Reference(_) | LParen | RParen | Bool(_) | Regex(_) | Duration(_)
            | Date(_) | Func(_) => panic!("Token {:?} is not operator", self),
        }
    }

    /// If operator can be used in unary form, return it with mode flag and new assoc flag.
    pub fn to_unary(&self, prec: u8) -> (Self, bool, u8) {
        use Token::*;

        match self {
            Add(_) => (Add(true), false, 10),
            Sub(_) => (Sub(true), false, 10),
            Not => (Not, false, prec),
            Less(_) => (Less(true), false, prec),
            LessEq(_) => (LessEq(true), false, prec),
            Greater(_) => (Greater(true), false, prec),
            GreaterEq(_) => (GreaterEq(true), false, prec),
            _ => (self.clone(), true, prec),
        }
    }

    /// Produce a sum of durations or durations and dates, error for
    /// incompatible arguments.
    pub fn sum(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Duration(lhs), Duration(rhs)) => Ok(Duration(lhs + rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Date(lhs + *rhs as i64)),
            (Duration(lhs), Date(rhs)) => Ok(Date(*lhs as i64 + rhs)),
            _ => {
                bail!(
                    "Unsupported '+' operator arguments ({} and {})",
                    self.ttype(),
                    rhs.ttype()
                )
            }
        }
    }

    /// Produce a substraction of durations and dates.
    pub fn sub(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Duration(lhs), Duration(rhs)) => Ok(Duration(lhs - rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Date(lhs - *rhs as i64)),
            (Date(lhs), Date(rhs)) => Ok(Duration((lhs - rhs) as f64)),
            (Duration(_), Date(_)) => bail!("Unable substract date from span"),
            _ => bail!(
                "Unsupported '-' operator arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Produce a multiplication of two duration values.
    pub fn mul(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Duration(lhs), Duration(rhs)) => Ok(Duration(lhs * rhs)),
            _ => bail!(
                "Unsupported '*' operator arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Produce a multiplication of two duration values.
    /// On booleans, produce 'or'.
    pub fn div(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Duration(lhs), Duration(rhs)) => {
                if rhs.abs() == 0.0 {
                    bail!("Division '/' by zero");
                }
                Ok(Duration(lhs / rhs))
            }
            _ => bail!(
                "Unsupported '/' operator arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Find the division remainder.
    pub fn modulo(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (self, rhs) {
            (Duration(lhs), Duration(rhs)) => Ok(Duration(lhs % rhs)),
            _ => bail!("Unsupported '%' operator arguments"),
        }
    }

    /// Produce a negative result of a value.
    pub fn neg(self) -> Result<Self> {
        match &self {
            Self::Duration(duration) => Ok(Self::Duration(-duration)),
            Self::Date(_) => bail!("Date can't be negative"),
            _ => bail!("Unary '-' applied to the wrong token ({})", self.ttype()),
        }
    }

    /// Apply time to the date.
    pub fn at(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        match (self, rhs) {
            (Self::Date(lhs), Self::Date(rhs)) => {
                let ltime = (lhs + ts.offset().whole_seconds() as i64) % 86400;
                let rtime = (rhs + ts.offset().whole_seconds() as i64) % 86400;
                Ok(Self::Date(lhs - ltime + rtime))
            }
            (Self::Duration(lhs), Self::Date(rhs)) => {
                let rtime = (rhs + ts.offset().whole_seconds() as i64) % 86400;
                let with_offset = ts
                    .saturating_add((lhs as i64).seconds())
                    .replace_time(Time::MIDNIGHT)
                    .saturating_add(rtime.seconds());
                Ok(Self::Date(with_offset.unix_timestamp()))
            }
            _ => bail!("'@' can only be applied to spans and dates"),
        }
    }

    /// Check if expression is boolean 'true' - otherwise produce 'else' value.
    /// Other operators may interpret 'else' as 'false', but 'else' used on itself
    /// always selects the right branch.
    pub fn r#if(self, rhs: Self) -> Self {
        match rhs {
            Self::Bool(true) => self,
            _ => Self::Else,
        }
    }

    /// Check if expression is 'else' produced by 'if' - otherwise return right argument.
    pub fn r#else(self, rhs: Self) -> Self {
        match self {
            Self::Else => rhs,
            _ => self,
        }
    }

    /// Perform logical AND.
    pub fn and(self, rhs: Self) -> Self {
        match (&self, &rhs) {
            (Self::Bool(false) | Self::Else, _) => self,
            _ => rhs,
        }
    }

    /// Perform logical OR.
    ///
    /// Only 'false' booleans are treated as 'false' - that allows to use
    /// 'or' as coalesce operator for possibly missing values.
    pub fn or(self, rhs: Self) -> Self {
        match (&self, &rhs) {
            (Self::Bool(false) | Self::Else, _) => rhs,
            _ => self,
        }
    }

    /// Check if two are exactly the same.
    pub fn eq(self, rhs: Self, entry: &Entry) -> Result<Self> {
        match (&self, &rhs) {
            (Self::Bool(lhs), Self::Bool(rhs)) => Ok(Self::Bool(lhs == rhs)),
            (Self::Date(_lhs), Self::Bool(rhs)) => Ok(Self::Bool(*rhs)),
            (Self::Bool(lhs), Self::Date(_rhs)) => Ok(Self::Bool(*lhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs == rhs)),
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs == rhs)),
            (Self::Reference(lhs), rhs) => Ok(Self::Bool(lhs.eq(rhs, entry)?)),
            (lhs, Self::Reference(rhs)) => Ok(Self::Bool(rhs.eq(lhs, entry)?)),
            _ => bail!(
                "'eq' ('==') was used on incompatible values ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Check if two values are not equal.
    pub fn not_eq(self, rhs: Self, entry: &Entry) -> Result<Self> {
        match (&self, &rhs) {
            (Self::Bool(lhs), Self::Bool(rhs)) => Ok(Self::Bool(lhs != rhs)),
            (Self::Date(_lhs), Self::Bool(rhs)) => Ok(Self::Bool(!rhs)),
            (Self::Bool(lhs), Self::Date(_rhs)) => Ok(Self::Bool(!lhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs != rhs)),
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs != rhs)),
            (Self::Reference(lhs), rhs) => Ok(Self::Bool(!lhs.eq(rhs, entry)?)),
            (lhs, Self::Reference(rhs)) => Ok(Self::Bool(!rhs.eq(lhs, entry)?)),
            _ => bail!(
                "'eq' ('!=') was used on incompatible values ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Peform loose comparison.
    pub fn fuzzy_eq(&self, rhs: &Self, issue: &Entry, ts: OffsetDateTime) -> Result<Self> {
        use Token::*;

        match (self, rhs) {
            (Date(lhs), Date(rhs)) => Ok(Bool(date_to_sod(ts, *lhs) == date_to_sod(ts, *rhs))),
            (Date(lhs), Duration(rhs)) => Ok(Bool(
                date_to_sod(ts, *lhs) == date_to_sod(ts, duration_to_date(*rhs, ts)),
            )),
            (Duration(lhs), Date(rhs)) => Ok(Bool(
                date_to_sod(ts, duration_to_date(*lhs, ts)) == date_to_sod(ts, *rhs),
            )),
            (Duration(lhs), Duration(rhs)) => Ok(Bool(
                date_to_sod(ts, duration_to_date(*lhs, ts))
                    == date_to_sod(ts, duration_to_date(*rhs, ts)),
            )),
            (Bool(_), Date(_) | Duration(_)) => Ok(Bool(false)),
            (Bool(lhs), Bool(rhs)) => Ok(Bool(*lhs == *rhs)),
            (Date(_lhs), Bool(rhs)) => Ok(Bool(*rhs)),
            (String(lhs), String(rhs)) => Ok(Bool(lhs.contains(&**rhs))),
            (Reference(lhs), token) => Ok(Bool(lhs.fuzzy_eq(token, issue)?)),
            _ => bail!(
                "':' was used on incompatible values ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Perform logical NOT.
    pub fn not(self) -> Result<Self> {
        match self {
            Self::Bool(val) => Ok(Self::Bool(!val)),
            Self::Date(_) => Ok(Self::Bool(false)),
            _ => bail!(
                "'not' ('!') got incompatible argument ({}), can only be applied to boolean",
                self.ttype()
            ),
        }
    }

    /// Perform greater comparison.
    pub fn greater(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Date(lhs), Date(rhs)) => Ok(Bool(lhs > rhs)),
            (Duration(lhs), Duration(rhs)) => Ok(Bool(lhs > rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Bool(*lhs > duration_to_date(*rhs, ts))),
            (Duration(lhs), Date(rhs)) => Ok(Bool(duration_to_date(*lhs, ts) > *rhs)),

            (Bool(false) | Else, Duration(_) | Date(_)) => Ok(Bool(false)),
            (Duration(_) | Date(_), Bool(false) | Else) => Ok(Bool(false)),

            _ => bail!(
                "'>' operator got incompatibe arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Perform greater comparison.
    pub fn greater_eq(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Date(lhs), Date(rhs)) => Ok(Bool(lhs >= rhs)),
            (Duration(lhs), Duration(rhs)) => Ok(Bool(lhs >= rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Bool(*lhs >= duration_to_date(*rhs, ts))),
            (Duration(lhs), Date(rhs)) => Ok(Bool(duration_to_date(*lhs, ts) >= *rhs)),

            (Bool(false) | Else, Duration(_) | Date(_)) => Ok(Bool(false)),
            (Duration(_) | Date(_), Bool(false) | Else) => Ok(Bool(false)),

            _ => bail!(
                "'>=' operator got incompatibe arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Perform greater comparison.
    pub fn less(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Date(lhs), Date(rhs)) => Ok(Bool(lhs < rhs)),
            (Duration(lhs), Duration(rhs)) => Ok(Bool(lhs < rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Bool(*lhs < duration_to_date(*rhs, ts))),
            (Duration(lhs), Date(rhs)) => Ok(Bool(duration_to_date(*lhs, ts) < *rhs)),

            (Bool(false) | Else, Duration(_) | Date(_)) => Ok(Bool(false)),
            (Duration(_) | Date(_), Bool(false) | Else) => Ok(Bool(false)),

            _ => bail!(
                "'<' operator got incompatibe arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Perform greater comparison.
    pub fn less_eq(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        use Token::*;

        match (&self, &rhs) {
            (Date(lhs), Date(rhs)) => Ok(Bool(lhs <= rhs)),
            (Duration(lhs), Duration(rhs)) => Ok(Bool(lhs <= rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Bool(*lhs <= duration_to_date(*rhs, ts))),
            (Duration(lhs), Date(rhs)) => Ok(Bool(duration_to_date(*lhs, ts) <= *rhs)),

            (Bool(false) | Else, Duration(_) | Date(_)) => Ok(Bool(false)),
            (Duration(_) | Date(_), Bool(false) | Else) => Ok(Bool(false)),

            _ => bail!(
                "'<=' operator got incompatibe arguments ({} and {})",
                self.ttype(),
                rhs.ttype()
            ),
        }
    }

    /// Produce type name of the token.
    pub fn ttype(&self) -> &'static str {
        use Token::*;
        match self {
            Date(_) => "date",
            Duration(_) => "number",
            Bool(_) => "boolean",
            String(_) => "string",
            Reference(_) => "reference",
            Regex(_) => "regex",
            _ => "operator",
        }
    }

    /// Produce token string representation.
    pub fn to_string(&self) -> Result<String> {
        use Token::*;
        Ok(match self {
            Date(d) => {
                let utc = UtcDateTime::from_unix_timestamp(*d)?;
                let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
                utc.to_offset(UtcOffset::current_local_offset()?);
                utc.format(format)?
            }
            Duration(v) => format!("{:.0}", v.round()),
            Bool(v) => v.to_string(),
            String(v) => v.to_string(),
            _ => format!("{:?}", self),
        })
    }
}
