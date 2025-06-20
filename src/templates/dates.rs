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
        let mlt = 10_f64.powi(precision.unwrap_or(1));
        (v * mlt).round() / mlt
    };

    if abs >= YEAR {
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
        let mlt = 10_f64.powi(precision.unwrap_or(1));
        let val = (v * mlt).round() / mlt;
        (val, if val > 1. { "s" } else { "" })
    };

    if abs >= YEAR {
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
