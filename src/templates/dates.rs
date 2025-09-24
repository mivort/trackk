use std::collections::HashMap;
use std::fmt::Write;

use time::format_description::{self, OwnedFormatItem, well_known};
use time::macros::format_description;
use time::{UtcDateTime, UtcOffset};

use crate::prelude::*;

/// Time durations as i64.
mod i64d {
    pub(super) const YEAR: i64 = 365 * DAY;
    pub(super) const DAY: i64 = 24 * HOUR;
    pub(super) const HOUR: i64 = 60 * MINUTE;
    pub(super) const MINUTE: i64 = 60;
}

/// Time durations as f64.
mod f64d {
    pub(super) const YEAR: f64 = 365. * DAY;
    pub(super) const MONTH: f64 = 30. * DAY;
    pub(super) const WEEK: f64 = 7. * DAY;
    pub(super) const DAY: f64 = 86400.;
    pub(super) const HOUR: f64 = 3600.;
    pub(super) const MINUTE: f64 = 60.;
}

/// Format as short relative date adding a unit suffix (y, mo, w, d, h, m, s).
pub fn reldate(date: i64, now: i64, precision: Option<i32>) -> String {
    use f64d::*;

    let diff = (date - now) as f64;
    let abs = diff.abs();

    let round = |v: f64| {
        let mlt = 10_f64.powi(precision.unwrap_or(0));
        (v * mlt).round() / mlt
    };

    if abs >= MONTH * 11.5 {
        format!("{}y", round(diff / YEAR))
    } else if abs >= MONTH {
        format!("{}mo", round(diff / MONTH))
    } else if abs >= WEEK {
        format!("{}w", round(diff / WEEK))
    } else if abs >= DAY {
        format!("{}d", round(diff / DAY))
    } else if abs >= HOUR {
        format!("{}h", round(diff / HOUR))
    } else if abs >= MINUTE {
        format!("{}m", round(diff / MINUTE))
    } else {
        format!("{}s", diff)
    }
}

/// Produce long relative date (e.g. '1 day ago') with integer precision by default.
pub fn longreldate(date: i64, now: i64, precision: Option<i32>) -> String {
    use f64d::*;

    let diff = (date - now) as f64;
    let ago = if diff < 0. { " ago" } else { "" };

    let abs = diff.abs();

    let floor = |v: f64| -> (f64, &str) {
        let mlt = 10_f64.powi(precision.unwrap_or(0));
        let val = (v * mlt).floor() / mlt;
        (val, if val > 1. { "s" } else { "" })
    };

    if abs >= MONTH * 11.5 {
        let (val, s) = floor(abs / YEAR);
        format!("{val} year{s}{ago}")
    } else if abs >= MONTH {
        let (val, s) = floor(abs / MONTH);
        format!("{val} month{s}{ago}")
    } else if abs >= WEEK {
        let (val, s) = floor(abs / WEEK);
        format!("{val} week{s}{ago}")
    } else if abs >= DAY {
        let (val, s) = floor(abs / DAY);
        format!("{val} day{s}{ago}")
    } else if abs >= HOUR {
        let (val, s) = floor(abs / HOUR);
        format!("{val} hour{s}{ago}")
    } else if abs >= MINUTE {
        let (val, s) = floor(abs / MINUTE);
        format!("{val} minute{s}{ago}")
    } else {
        let (val, s) = floor(abs);
        format!("{val} second{s}{ago}")
    }
}

/// Format duration in datecalc-readable format.
pub fn duration(duration: i64) -> String {
    use i64d::*;
    let mut out = String::new();

    let delim = if duration < 0 {
        out.push('-');
        "-"
    } else {
        "+"
    };

    let abs = duration.abs();

    let years = abs / YEAR;
    let days = abs % YEAR / DAY;
    let hours = abs % DAY / HOUR;
    let minutes = abs % HOUR / MINUTE;
    let seconds = abs % MINUTE;

    let mut append = false;

    let mut add_unit = |value: i64, unit: char| {
        if value <= 0 {
            return;
        }
        if append {
            let _ = write!(out, "{delim}");
        } else {
            append = true;
        }
        let _ = write!(out, "{value}{unit}");
    };

    add_unit(years, 'y');
    add_unit(days, 'd');
    add_unit(hours, 'h');
    add_unit(minutes, 'm');
    add_unit(seconds, 's');

    out
}

/// Format date/time using one of the defined formatters.
pub fn datefmt(
    ts: i64,
    fmt: Option<&str>,
    formats: &HashMap<String, OwnedFormatItem>,
    offset: UtcOffset,
) -> String {
    use well_known::iso8601::{Config, FormattedComponents, Iso8601, TimePrecision};
    let fmt = fmt.unwrap_or("default");

    let ts = safe_clamp(ts);
    let date = UtcDateTime::from_unix_timestamp(ts)
        .unwrap_or_else(|_| panic!("Timestamp value is outside of the valid range: {}", ts))
        .to_offset(offset);

    if let Some(fmt) = formats.get(fmt) {
        date.format(fmt).unwrap()
    } else {
        match fmt {
            "rfc2822" | "long" => date.format(&well_known::Rfc2822),
            "rfc3339" => date.format(&well_known::Rfc3339),
            "date" => date.format(&Iso8601::DATE),
            "time" => date.format(format_description!("[hour]:[minute]:[second]")),
            _ => {
                const CONFIG: u128 = Config::DEFAULT
                    .set_formatted_components(FormattedComponents::DateTime)
                    .set_time_precision(TimePrecision::Second {
                        decimal_digits: None,
                    })
                    .encode();
                date.format(&Iso8601::<CONFIG>)
            }
        }
        .unwrap()
    }
}

/// Apply ISO8601 format to UNIX timestamps.
pub fn datefmt_iso8601(ts: i64, offset: UtcOffset) -> String {
    let ts = safe_clamp(ts);
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    let time = UtcDateTime::from_unix_timestamp(ts)
        .unwrap_or_else(|_| panic!("Timestamp value is outside of the valid range: {}", ts))
        .to_offset(offset);
    let time = time.to_offset(offset);
    time.format(&format).unwrap()
}

/// Convert defined parse formats into parsed format items.
pub fn parse_formats(
    formats: &HashMap<String, String>,
) -> Result<HashMap<String, OwnedFormatItem>> {
    let mut output = HashMap::<String, _>::default();
    for (k, v) in formats {
        output.insert(k.to_owned(), format_description::parse_owned::<2>(v)?);
    }

    Ok(output)
}

/// Limit date range to fit into safe range with all possible offsets.
pub fn safe_clamp(ts: i64) -> i64 {
    ts.clamp(
        UtcDateTime::MIN.unix_timestamp() + 86400,
        UtcDateTime::MAX.unix_timestamp() - 86400,
    )
}

#[test]
pub fn duration_fmt() {
    use i64d::*;

    assert_eq!(duration(HOUR), "1h");
    assert_eq!(duration(DAY + HOUR), "1d+1h");
    assert_eq!(duration(HOUR + 30 * MINUTE), "1h+30m");
    assert_eq!(duration(YEAR + 1), "1y+1s");

    assert_eq!(duration(-HOUR), "-1h");
    assert_eq!(duration(-DAY - HOUR), "-1d-1h");
    assert_eq!(duration(-YEAR - DAY - HOUR - MINUTE), "-1y-1d-1h-1m");
}
