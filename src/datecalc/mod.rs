pub mod eval;
pub mod parse;
pub mod token;
pub mod token_ops;

use time::OffsetDateTime;

/// Convert f64 duration to i64 absolute time.
#[inline]
pub fn duration_to_date(duration: f64, ts: OffsetDateTime) -> i64 {
    duration as i64 + ts.unix_timestamp()
}

/// Convert date to start of the date within current time zone.
#[inline]
pub fn date_to_sod(ts: OffsetDateTime, date: i64) -> i64 {
    date - (date + ts.offset().whole_seconds() as i64) % 86400
}
