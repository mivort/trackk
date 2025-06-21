use std::collections::HashMap;

use time::format_description::{self, OwnedFormatItem, well_known};
use time::macros::format_description;
use time::{UtcDateTime, UtcOffset};

use crate::prelude::*;

const YEAR: f64 = 365. * DAY;
const MONTH: f64 = 30. * DAY;
const WEEK: f64 = 7. * DAY;
const DAY: f64 = 86400.;
const HOUR: f64 = 3600.;
const MINUTE: f64 = 60.;

/// Format as short relative date adding a unit suffix (y, mo, w, d, h, m, s).
pub fn reldate(date: i64, now: i64, precision: Option<i32>) -> String {
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
    let diff = (date - now) as f64;
    let ago = if diff < 0. { " ago" } else { "" };

    let abs = diff.abs();

    let round = |v: f64| -> (f64, &str) {
        let mlt = 10_f64.powi(precision.unwrap_or(0));
        let val = (v * mlt).round() / mlt;
        (val, if val > 1. { "s" } else { "" })
    };

    if abs >= MONTH * 11.5 {
        let (val, s) = round(abs / YEAR);
        format!("{val} year{s}{ago}")
    } else if abs >= MONTH {
        let (val, s) = round(abs / MONTH);
        format!("{val} month{s}{ago}")
    } else if abs >= WEEK {
        let (val, s) = round(abs / WEEK);
        format!("{val} week{s}{ago}")
    } else if abs >= DAY {
        let (val, s) = round(abs / DAY);
        format!("{val} day{s}{ago}")
    } else if abs >= HOUR {
        let (val, s) = round(abs / HOUR);
        format!("{val} hour{s}{ago}")
    } else if abs >= MINUTE {
        let (val, s) = round(abs / MINUTE);
        format!("{val} minute{s}{ago}")
    } else {
        let (val, s) = round(abs);
        format!("{val} second{s}{ago}")
    }
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
