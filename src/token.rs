use std::num::ParseIntError;
use std::rc::Rc;

use logos::{Lexer, Logos};
use thiserror::Error;
use time::ext::NumericalDuration;
use time::macros::format_description;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcDateTime, UtcOffset, Weekday};

use crate::issue::{FieldRef, Issue};
use crate::prelude::*;

/// Max date value supported by time-rs.
pub const SOMEDAY: i64 = 253402300799 - 86400;

/// Parsed token types.
///
/// When operator token is added, it needs to be acounted in four places:
/// * Precedence and associativity: [Token::prec_and_assoc].
/// * Unary conversion: [Token::to_unary].
/// * Date exp lexing.
/// * Date exp evaluation.
#[derive(Clone, Debug, Logos)]
#[logos(skip r"[ \t\n\f]+", extras = OffsetDateTime, error = LexerError)]
pub enum Token {
    #[regex(r"\d+(\.\d+)?", parse_no_suffix_span)]
    #[regex(r"\d+(\.\d+)?s", |l| parse_suffix_span(l, 1, 1.))]
    #[regex(r"\d+(\.\d+)?sec", |l| parse_suffix_span(l, 3, 1.))]
    #[regex(r"\d+(\.\d+)?m", |l| parse_suffix_span(l, 1, 60.))]
    #[regex(r"\d+(\.\d+)?min", |l| parse_suffix_span(l, 3, 60.))]
    #[regex(r"\d+(\.\d+)?h", |l| parse_suffix_span(l, 1, 3_600.))]
    #[regex(r"\d+(\.\d+)?hrs", |l| parse_suffix_span(l, 3, 3_600.))]
    #[regex(r"\d+(\.\d+)?[Dd]", |l| parse_suffix_span(l, 1, 86_400.))]
    #[regex(r"\d+(\.\d+)?[Ww]", |l| parse_suffix_span(l, 1, 604_800.))]
    #[regex(r"\d+(\.\d+)?M", |l| parse_suffix_span(l, 1, 2_592_000.))]
    #[regex(r"\d+(\.\d+)?[Mm][Oo]", |l| parse_suffix_span(l, 2, 2_592_000.))]
    #[regex(r"\d+(\.\d+)?[Yy]", |l| parse_suffix_span(l, 1, 31_536_000.))]
    #[regex(r"\d+(\.\d+)?k", |l| parse_suffix_span(l, 1, 1_000.))]
    #[regex(r"\d+(\.\d+)?mil", |l| parse_suffix_span(l, 3, 1_000_000.))]
    Duration(f64),

    #[regex(r"(?i)\d+(st|nd|rd|th)", parse_st_nd_rd_th)]
    #[regex(r"\d{1,2}:\d{2}", parse_24h)]
    #[regex(r"\d{1,2}:\d{2}:\d{2}", parse_24h_sec)]
    #[regex(r"\d{1,2}:\d{2}[AaPp][Mm]", parse_12h)]
    #[regex(r"\d{1,2}:\d{2}:\d{2}[AaPp][Mm]", parse_12h_sec)]
    #[regex(r"\d{2}-\d{2}", parse_short_date)]
    #[regex(r"\d{4,}-\d{3}", parse_ordinal)]
    #[regex(r"\d{4,}-\d{2}-\d{2}", parse_full_date)]
    #[regex(r"\d{4,}-\d{2}-\d{2}T\d{2}:\d{2}", parse_date_time)]
    #[regex(r"\d{4,}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", parse_date_time_sec)]
    #[regex(r"(?i)now", |lex| lex.extras.unix_timestamp())]
    #[regex(r"(?i)someday", |_| SOMEDAY)]
    #[regex(r"(?i)(sod|today)", |lex| lex.extras.replace_time(Time::MIDNIGHT).unix_timestamp())]
    #[regex(r"(?i)tomorrow", |lex| relative_sod(lex, 1))]
    #[regex(r"(?i)yesterday", |lex| relative_sod(lex, -1))]
    #[regex(r"(?i)mon(day)?", |lex| relative_weekday(lex, Weekday::Monday))]
    #[regex(r"(?i)tue(sday)?", |lex| relative_weekday(lex, Weekday::Tuesday))]
    #[regex(r"(?i)wed(nesday)?", |lex| relative_weekday(lex, Weekday::Wednesday))]
    #[regex(r"(?i)thu(rsday)?", |lex| relative_weekday(lex, Weekday::Thursday))]
    #[regex(r"(?i)fri(day)?", |lex| relative_weekday(lex, Weekday::Friday))]
    #[regex(r"(?i)sat(urday)?", |lex| relative_weekday(lex, Weekday::Saturday))]
    #[regex(r"(?i)sun(day)?", |lex| relative_weekday(lex, Weekday::Sunday))]
    #[regex(r"(?i)jan(uary)?", |lex| relative_month(lex, Month::January))]
    #[regex(r"(?i)feb(ruary)?", |lex| relative_month(lex, Month::February))]
    #[regex(r"(?i)mar(ch)?", |lex| relative_month(lex, Month::March))]
    #[regex(r"(?i)apr(il)?", |lex| relative_month(lex, Month::April))]
    #[regex(r"(?i)may", |lex| relative_month(lex, Month::May))]
    #[regex(r"(?i)june?", |lex| relative_month(lex, Month::June))]
    #[regex(r"(?i)july?", |lex| relative_month(lex, Month::July))]
    #[regex(r"(?i)aug(ust)?", |lex| relative_month(lex, Month::August))]
    #[regex(r"(?i)sep(tember)?", |lex| relative_month(lex, Month::September))]
    #[regex(r"(?i)oct(ober)?", |lex| relative_month(lex, Month::October))]
    #[regex(r"(?i)nov(ember)?", |lex| relative_month(lex, Month::November))]
    #[regex(r"(?i)dec(ember)?", |lex| relative_month(lex, Month::December))]
    Date(i64),

    #[token("true", |_| true)]
    #[token("false", |_| false)]
    Bool(bool),

    #[token("//", parse_regex)]
    #[allow(unused)]
    Regex(Rc<regex::Regex>),

    /// Addition operator with unary mode flag.
    #[token("+", |_| false)]
    Add(bool),

    /// Substraction operator with unary mode flag.
    #[token("-", |_| false)]
    Sub(bool),

    #[token("*")]
    Mul,

    #[token("/")]
    Div,

    #[token("%")]
    Mod,

    #[token("@")]
    #[token("at")]
    At,

    // TODO: P2: add 'functions': 'min', 'max', 'clamp', 'pow'
    // TODO: P2: add 'until' operator which compares the value vs. max
    //           and returns either value itself or 'false'
    #[token(":")]
    FuzzyEq,

    #[token("=")]
    #[token("==")]
    Eq,

    #[token("!=")]
    #[token("~=")]
    NotEq,

    #[token("<")]
    #[token("before")]
    Less,

    #[token("<=")]
    LessEq,

    #[token(">")]
    #[token("after")]
    Greater,

    #[token(">=")]
    GreaterEq,

    #[token("&&")]
    #[token("and")]
    And,

    #[token("||")]
    #[token("or")]
    Or,

    #[token("!")]
    #[token("~")]
    #[token("not")]
    Not,

    #[token("sqrt")]
    Sqrt,

    #[token("ln")]
    Ln,

    #[token("abs")]
    Abs,

    #[token("sig")]
    Sig,

    #[token("len")]
    Len,

    #[token("has")]
    Has,

    #[token("(")]
    #[token("[")]
    LParen,

    #[token(")")]
    #[token("]")]
    RParen,

    #[token("title", |_| FieldRef::Title)]
    #[token("desc", |_| FieldRef::Desc)]
    #[token("status", |_| FieldRef::Status)]
    #[token("tag", |_| FieldRef::Tag)]
    #[token("created", |_| FieldRef::Created)]
    #[token("modified", |_| FieldRef::Modified)]
    #[token("due", |_| FieldRef::Due)]
    #[regex("end", |_| FieldRef::End)]
    Reference(FieldRef),

    /// String value token.
    #[regex(r"[A-Za-z]\w*", |l| Rc::from(l.slice()))]
    #[regex(r#"'[^']*'"#, parse_quoted_string)]
    String(Rc<str>),
}

impl Token {
    /// Check token precedence and if it's left associative.
    pub fn prec_and_assoc(&self) -> (u8, bool) {
        use Token::*;

        match self {
            FuzzyEq => (9, true),
            Not | Sqrt | Ln | Abs | Sig | Len | Has => (8, false),
            At => (7, true),
            Mul | Div | Mod => (6, true),
            Add(_) | Sub(_) => (5, true),
            Less | LessEq | Greater | GreaterEq => (4, true),
            Eq | NotEq => (3, true),
            And => (2, true),
            Or => (1, true),
            _ => panic!("Token {:?} is not operator", self),
        }
    }

    /// If operator can be used in unary form, return it with mode flag and new assoc flag.
    pub fn to_unary(&self) -> (Self, bool) {
        use Token::*;

        match self {
            Add(_) => (Add(true), false),
            Sub(_) => (Sub(true), false),
            Not => (Not, false),
            Sqrt | Ln | Abs | Sig | Len | Has => (self.clone(), false),
            _ => (self.clone(), true),
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

        match (self, rhs) {
            (Duration(lhs), Duration(rhs)) => Ok(Duration(lhs - rhs)),
            (Date(lhs), Duration(rhs)) => Ok(Date(lhs - rhs as i64)),
            (Date(lhs), Date(rhs)) => Ok(Duration((lhs - rhs) as f64)),
            (Duration(_), Date(_)) => bail!("Unable substract date from span"),
            _ => bail!("Unsupported '-' operator arguments"),
        }
    }

    /// Produce a multiplication of two duration values.
    pub fn mul(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (self, rhs) {
            (Duration(lhs), Duration(rhs)) => Ok(Duration(lhs * rhs)),
            _ => bail!("Unsupported '*' operator arguments"),
        }
    }

    /// Produce a multiplication of two duration values.
    /// On booleans, produce 'or'.
    pub fn div(self, rhs: Self) -> Result<Self> {
        use Token::*;

        match (self, rhs) {
            (Duration(lhs), Duration(rhs)) => {
                if rhs.abs() == 0.0 {
                    bail!("Division '/' by zero");
                }
                Ok(Duration(lhs / rhs))
            }
            _ => bail!("Unsupported '/' operator arguments"),
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
        match self {
            Self::Duration(duration) => Ok(Self::Duration(-duration)),
            Self::Date(_) => bail!("Date can't be negative"),
            _ => panic!("Operator applied to the wrong token"),
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

    /// Perform logical AND.
    ///
    /// NOTE: It's intentionally not allowed to have right argument as boolean to\
    /// prevent ternary operator usage caveat (`x < 0 and false or true`).
    pub fn and(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Bool(lhs), Self::Bool(rhs)) => Ok(Self::Bool(lhs && rhs)),
            (Self::Bool(lhs), rhs) => Ok(if lhs { rhs } else { Self::Bool(false) }),
            _ => bail!("'and' ('&&') left argument should be a boolean"),
        }
    }

    /// Perform logical OR.
    pub fn or(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Bool(lhs), Self::Bool(rhs)) => Ok(Self::Bool(lhs || rhs)),
            (Self::Bool(lhs), rhs) => Ok(if lhs { Self::Bool(true) } else { rhs }),
            (lhs, _) => Ok(lhs),
        }
    }

    /// Check if two are exactly the same.
    pub fn eq(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Bool(lhs), Self::Bool(rhs)) => Ok(Self::Bool(lhs == rhs)),
            (Self::Date(_lhs), Self::Bool(rhs)) => Ok(Self::Bool(rhs)),
            (Self::Bool(lhs), Self::Date(_rhs)) => Ok(Self::Bool(lhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs == rhs)),
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs == rhs)),
            _ => bail!("'eq' ('==') was used on incompatible values"),
        }
    }

    /// Peform loose comparison.
    pub fn fuzzy_eq(&self, rhs: &Self, issue: &Issue) -> Result<Self> {
        match (self, rhs) {
            (Self::Bool(lhs), Self::Bool(rhs)) => Ok(Self::Bool(*lhs == *rhs)),
            (Self::Date(_lhs), Self::Bool(rhs)) => Ok(Self::Bool(*rhs)),
            (Self::String(lhs), Self::String(rhs)) => Ok(Self::Bool(lhs.contains(&**rhs))),
            (Self::Reference(lhs), token) => Ok(Self::Bool(lhs.fuzzy_eq(token, issue)?)),
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

    /// Apply unary operation to single numeric value.
    #[inline]
    pub fn unary_op(self, f: impl Fn(f64) -> f64) -> Result<Self> {
        match self {
            Self::Duration(val) => Ok(Self::Duration(f(val))),
            _ => bail!(
                "Unary function got incompatible argument ({})",
                self.ttype()
            ),
        }
    }

    /// Produce length of the token value.
    pub fn length(self, entry: &Issue) -> Result<Self> {
        match self {
            Self::String(val) => Ok(Self::Duration(val.len() as f64)),
            Self::Reference(field) => Ok(Self::Duration(field.length(entry))),
            _ => bail!(
                "'len' function got incompatible argument ({})",
                self.ttype()
            ),
        }
    }

    /// Convert various values to boolean. Strings and arrays are 'true' if not
    /// empty, dates if less than 'someday'.
    pub fn has(self, entry: &Issue) -> Result<Self> {
        match self {
            Self::String(val) => Ok(Self::Bool(!val.is_empty())),
            Self::Bool(val) => Ok(Self::Bool(val)),
            Self::Date(val) => Ok(Self::Bool(val <= SOMEDAY)),
            Self::Reference(field) => Ok(Self::Bool(field.has(entry))),
            _ => bail!(
                "'has' function got incompatible argument ({})",
                self.ttype()
            ),
        }
    }

    /// Perform greater comparison.
    pub fn greater(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        match (self, rhs) {
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs > rhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs > rhs)),
            (Self::Date(lhs), Self::Duration(rhs)) => {
                Ok(Self::Bool(lhs > duration_to_date(rhs, ts)))
            }
            (Self::Duration(lhs), Self::Date(rhs)) => {
                Ok(Self::Bool(duration_to_date(lhs, ts) > rhs))
            }
            _ => bail!("'>' operator got incompatibe arguments"),
        }
    }

    /// Perform greater comparison.
    pub fn greater_eq(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        match (self, rhs) {
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs >= rhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs >= rhs)),
            (Self::Date(lhs), Self::Duration(rhs)) => {
                Ok(Self::Bool(lhs >= duration_to_date(rhs, ts)))
            }
            (Self::Duration(lhs), Self::Date(rhs)) => {
                Ok(Self::Bool(duration_to_date(lhs, ts) >= rhs))
            }
            _ => bail!("'>=' operator got incompatibe arguments"),
        }
    }

    /// Perform greater comparison.
    pub fn less(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        match (self, rhs) {
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs < rhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs < rhs)),
            (Self::Date(lhs), Self::Duration(rhs)) => {
                Ok(Self::Bool(lhs < duration_to_date(rhs, ts)))
            }
            (Self::Duration(lhs), Self::Date(rhs)) => {
                Ok(Self::Bool(duration_to_date(lhs, ts) < rhs))
            }
            _ => bail!("'<' operator got incompatibe arguments"),
        }
    }

    /// Perform greater comparison.
    pub fn less_eq(self, rhs: Self, ts: OffsetDateTime) -> Result<Self> {
        match (self, rhs) {
            (Self::Date(lhs), Self::Date(rhs)) => Ok(Self::Bool(lhs <= rhs)),
            (Self::Duration(lhs), Self::Duration(rhs)) => Ok(Self::Bool(lhs <= rhs)),
            (Self::Date(lhs), Self::Duration(rhs)) => {
                Ok(Self::Bool(lhs <= duration_to_date(rhs, ts)))
            }
            (Self::Duration(lhs), Self::Date(rhs)) => {
                Ok(Self::Bool(duration_to_date(lhs, ts) <= rhs))
            }
            _ => bail!("'<=' operator got incompatibe arguments"),
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

/// Custom lexing error type.
#[derive(Clone, Default, Debug, PartialEq, Error)]
pub enum LexerError {
    #[default]
    #[error("Unrecognized token")]
    UnknownError,

    #[error("Unable to parse token: {token}")]
    TokenError { token: String },

    #[error("Unable to parse date token: {token}")]
    DateError { token: String },

    #[error(transparent)]
    ParseInt(#[from] ParseIntError),

    #[error(transparent)]
    ComponentRange(#[from] time::error::ComponentRange),
}

impl LexerError {
    fn token_error(token: &str) -> Self {
        Self::TokenError {
            token: token.to_owned(),
        }
    }

    fn date_error(token: &str) -> Self {
        Self::DateError {
            token: token.to_owned(),
        }
    }
}

/// Parse no-suffix duration.
#[inline]
fn parse_no_suffix_span(lex: &Lexer<Token>) -> Option<f64> {
    lex.slice().parse().ok()
}

/// Exclude suffix and parse with multiplier.
#[inline]
fn parse_suffix_span(lex: &Lexer<Token>, width: usize, mlt: f64) -> Option<f64> {
    let slice = lex.slice();
    slice[..slice.len() - width]
        .parse()
        .ok()
        .map(|v: f64| v * mlt)
}

/// Parse the closest month day.
fn parse_st_nd_rd_th(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let slice = lex.slice();
    let num = slice[..slice.len() - 2].parse::<u8>()?;

    let prev = lex.extras;
    let date = if prev.day() >= num {
        let year = if prev.month() == Month::December { prev.year() + 1 } else { prev.year() };
        prev.replace_date(Date::from_calendar_date(
            year,
            lex.extras.month().next(),
            num,
        )?)
    } else {
        prev.replace_day(num)?
    }
    .replace_time(Time::MIDNIGHT);

    Ok(date.unix_timestamp())
}

/// Parse time in 24H format.
fn parse_24h(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!("[hour padding:none]:[minute]");
    let time = unwrap_ok_or!(Time::parse(lex.slice(), &format), _, {
        return Err(LexerError::token_error(lex.slice()));
    });
    Ok(relative_time(time, lex.extras))
}

/// Parse time in 24H format with seconds.
fn parse_24h_sec(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!("[hour padding:none]:[minute]:[second]");
    let time = unwrap_ok_or!(Time::parse(lex.slice(), &format), _, {
        return Err(LexerError::token_error(lex.slice()));
    });
    Ok(relative_time(time, lex.extras))
}

/// Parse time in 12H format.
fn parse_12h(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format =
        format_description!("[hour repr:12 padding:none]:[minute][period case_sensitive:false]");
    let time = unwrap_ok_or!(Time::parse(lex.slice(), &format), _, {
        return Err(LexerError::token_error(lex.slice()));
    });
    Ok(relative_time(time, lex.extras))
}

/// Parse time in 12H format with seconds.
fn parse_12h_sec(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!(
        "[hour repr:12 padding:none]:[minute]:[second][period case_sensitive:false]"
    );
    let time = unwrap_ok_or!(Time::parse(lex.slice(), &format), _, {
        return Err(LexerError::token_error(lex.slice()));
    });
    Ok(relative_time(time, lex.extras))
}

/// Produce relative date with specified time. If time has already passed for today,
/// switch to the next day.
fn relative_time(time: Time, date: OffsetDateTime) -> i64 {
    if date.time() >= time {
        date.saturating_add(1.days()).replace_time(time)
    } else {
        date.replace_time(time)
    }
    .unix_timestamp()
}

/// Parse date in `[month]-[day]` format (non-ISO 8601).
fn parse_short_date(_lex: &Lexer<Token>) -> Result<i64, LexerError> {
    // TODO: P2: parse short relative dates?
    todo!()
}

/// Parse ordinal date format (`[year]-[ordinal]`).
fn parse_ordinal(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!("[year]-[ordinal]");
    let res = unwrap_ok_or!(Date::parse(lex.slice(), &format), _, {
        return Err(LexerError::token_error(lex.slice()));
    });
    let time = res.with_time(Time::MIDNIGHT);
    Ok(time.assume_offset(lex.extras.offset()).unix_timestamp())
}

/// Parse date in `[year]-[month]-[day]` format.
fn parse_full_date(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!("[year]-[month]-[day]");
    let res = unwrap_ok_or!(Date::parse(lex.slice(), &format), _, {
        return Err(LexerError::date_error(lex.slice()));
    });
    let time = res.with_time(Time::MIDNIGHT);
    Ok(time.assume_offset(lex.extras.offset()).unix_timestamp())
}

/// Parse date and time in `[year]-[month]-[day]T[hour]:[minute] format.
fn parse_date_time(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]");
    let res = unwrap_ok_or!(PrimitiveDateTime::parse(lex.slice(), &format), _, {
        return Err(LexerError::date_error(lex.slice()));
    });
    Ok(res.assume_offset(lex.extras.offset()).unix_timestamp())
}

/// Parse date and time in `[year]-[month]-[day]T[hour]:[minute]:[second] format.
fn parse_date_time_sec(lex: &Lexer<Token>) -> Result<i64, LexerError> {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    let res = unwrap_ok_or!(PrimitiveDateTime::parse(lex.slice(), &format), _, {
        return Err(LexerError::date_error(lex.slice()));
    });
    Ok(res.assume_offset(lex.extras.offset()).unix_timestamp())
}

/// Find the the boundaries of regex.
fn parse_regex(lex: &mut Lexer<Token>) -> Result<Rc<regex::Regex>, LexerError> {
    let remainder = lex.remainder();
    let end = unwrap_some_or!(remainder.find(lex.slice()), {
        return Err(LexerError::token_error("regex teminator ('//') not found"));
    });

    let regex = Rc::new(unwrap_ok_or!(regex::Regex::new(&remainder[..end]), _, {
        return Err(LexerError::token_error(&remainder[..end]));
    }));

    lex.bump(end + lex.slice().len());
    Ok(regex)
}

/// Exclude quotes and return reference-counted str.
fn parse_quoted_string(lex: &Lexer<Token>) -> Rc<str> {
    Rc::from(&lex.slice()[1..lex.slice().len() - 2])
}

/// Parse relative date alias.
fn relative_sod(lex: &Lexer<Token>, offset: i64) -> i64 {
    lex.extras
        .saturating_add(offset.days())
        .replace_time(Time::MIDNIGHT)
        .unix_timestamp()
}

/// Parse nearest month to the selected date.
fn relative_month(lex: &Lexer<Token>, month: Month) -> i64 {
    let ts = lex.extras;
    let year = if ts.month() as u8 >= month as u8 { ts.year() + 1 } else { ts.year() };
    let date = Date::from_calendar_date(year, month, 1).unwrap();
    ts.replace_date(date)
        .replace_time(Time::MIDNIGHT)
        .unix_timestamp()
}

/// Convert weekday to the nearest date.
fn relative_weekday(lex: &Lexer<Token>, day: Weekday) -> i64 {
    let ts = lex.extras;
    let diff = ts.weekday().number_days_from_monday() as i64 - day.number_days_from_monday() as i64;

    let offset = if diff >= 0 { 7 - diff } else { -diff };
    ts.saturating_add(offset.days())
        .replace_time(Time::MIDNIGHT)
        .unix_timestamp()
}

/// Convert f64 duration to i64 absolute time.
#[inline]
fn duration_to_date(duration: f64, ts: OffsetDateTime) -> i64 {
    duration as i64 + ts.unix_timestamp()
}
