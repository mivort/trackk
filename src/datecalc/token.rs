use std::num::ParseIntError;
use std::rc::Rc;

use logos::{Lexer, Logos};
use thiserror::Error;
use time::ext::NumericalDuration;
use time::macros::format_description;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, Weekday};

use super::functions::FuncRef;
use crate::entry::FieldRef;
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
#[derive(Clone, Logos, Debug)]
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
    #[regex(r"(?i)epoch", |_| 0)]
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

    #[regex("[Tt]rue", |_| true)]
    #[regex("[Ff]alse", |_| false)]
    #[regex("[Nn]one", |_| false)]
    Bool(bool),

    Regex(Rc<regex::Regex>),

    /// Addition operator with unary mode flag.
    #[token("+", |_| false)]
    Add(bool),

    /// Substraction operator with unary mode flag.
    #[token("-", |_| false)]
    Sub(bool),

    #[token("*")]
    Mul,

    // TODO: P2: implement Python-like pow
    // `#[token("**")]`
    // `Pow,`
    #[token("/")]
    Div,

    // TODO: P2: implement Python-like integer div
    // `#[token("//")]`
    // `DivInt,`
    #[token("%")]
    Mod,

    #[token("@")]
    #[token("at")]
    At,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    // TODO: P2: add 'functions': 'min', 'max', 'clamp', 'pow'
    // TODO: P2: add 'until' operator which compares the value vs. max
    //           and returns either value itself or 'false'
    // TODO: P2: replace fuzzy eq with 'in'?
    #[token(":")]
    #[token("contains")]
    Contains,

    #[token("=")]
    #[token("==")]
    Eq,

    #[token("!=")]
    #[token("~=")]
    NotEq,

    #[token("<", |_| false)]
    #[token("before", |_| false)]
    Less(bool),

    #[token("<=", |_| false)]
    #[token("before_eq", |_| false)]
    LessEq(bool),

    #[token(">", |_| false)]
    #[token("after", |_| false)]
    Greater(bool),

    #[token(">=", |_| false)]
    #[token("after_eq", |_| false)]
    GreaterEq(bool),

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

    #[token("abs", |_| FuncRef::Abs)]
    #[token("has", |_| FuncRef::Has)]
    #[token("len", |_| FuncRef::Len)]
    #[token("lines", |_| FuncRef::Lines)]
    #[token("ln", |_| FuncRef::Ln)]
    #[token("sig", |_| FuncRef::Sig)]
    #[token("sqrt", |_| FuncRef::Sqrt)]
    Func(FuncRef),

    #[token("(")]
    #[token("[")]
    LParen,

    #[token(")")]
    #[token("]")]
    RParen,

    #[token("id", |_| FieldRef::Id)]
    #[token("title", |_| FieldRef::Title)]
    #[token("desc", |_| FieldRef::Desc)]
    #[token("status", |_| FieldRef::Status)]
    #[regex("tags?", |_| FieldRef::Tag)]
    #[token("created", |_| FieldRef::Created)]
    #[token("modified", |_| FieldRef::Modified)]
    #[token("when", |_| FieldRef::When)]
    #[token("due", |_| FieldRef::Due)]
    #[regex("end", |_| FieldRef::End)]
    Reference(FieldRef),

    /// String value token.
    #[regex(r"[^\d\W]\w*", |l| Rc::from(l.slice()))]
    #[regex(r#"'[^']*'"#, parse_quoted_string)]
    String(Rc<str>),
}

/// Custom lexing error type.
#[derive(Clone, Default, PartialEq, Error, Debug)]
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

/// Exclude quotes and return reference-counted str.
fn parse_quoted_string(lex: &Lexer<Token>) -> Rc<str> {
    Rc::from(&lex.slice()[1..lex.slice().len() - 1])
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
