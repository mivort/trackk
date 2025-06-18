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
        format!("{}Y", round(diff / YEAR))
    } else if abs >= MONTH {
        format!("{}M", round(diff / MONTH))
    } else if abs >= WEEK {
        format!("{}W", round(diff / WEEK))
    } else if abs >= DAY {
        format!("{}D", round(diff / DAY))
    } else if abs >= HOUR {
        format!("{}h", round(diff / HOUR))
    } else if abs >= MINUTE {
        format!("{}m", round(diff / MINUTE))
    } else {
        format!("{}s", diff)
    }
}
