const YEAR: f64 = 365. * DAY;
const MONTH: f64 = 30. * DAY;
const WEEK: f64 = 7. * DAY;
const DAY: f64 = 86400.;
const HOUR: f64 = 3600.;
const MINUTE: f64 = 60.;

/// Format as relative date adding a unit suffix (Y, M, W, D, h, m, s).
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

pub fn longreldate(date: i64, now: i64, precision: Option<i32>) -> String {
    let diff = (date - now) as f64;
    let ago = if diff < 0. { " ago" } else { "" };

    let abs = diff.abs();

    let round = |v: f64| {
        let mlt = 10_f64.powi(precision.unwrap_or(0));
        (v * mlt).round() / mlt
    };

    let s = "s"; // TODO: P2: correct plural forms

    if abs >= YEAR {
        format!("{} year{s}{ago}", round(abs / YEAR))
    } else if abs >= MONTH {
        format!("{} month{s}{ago}", round(abs / MONTH))
    } else if abs >= WEEK {
        format!("{} week{s}{ago}", round(abs / WEEK))
    } else if abs >= DAY {
        format!("{} day{s}{ago}", round(abs / DAY))
    } else if abs >= HOUR {
        format!("{} hour{s}{ago}", round(abs / HOUR))
    } else if abs >= MINUTE {
        format!("{} minute{s}{ago}", round(abs / MINUTE))
    } else {
        format!("{} second{s}{ago}", abs)
    }
}
