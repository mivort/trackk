use crate::templates::colors::{bg, fg};
use serde_derive::Deserialize;

use super::Config;

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(unused)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub enum ColorConfig {
    Options(ColorOptions),
    Custom(Box<str>),
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ColorOptions {
    pub fg: Option<u8>,
    pub bg: Option<u8>,
    _bold: bool,
    _italic: bool,
    _underscore: bool,
}

impl ColorConfig {
    /// Convert color config entry to escape sequence.
    pub fn format(&self) -> String {
        match self {
            ColorConfig::Options(options) => {
                let mut res = String::new();
                if let Some(color) = options.fg {
                    res.push_str(fg(color));
                }
                if let Some(color) = options.bg {
                    res.push_str(bg(color));
                }
                res
            }
            ColorConfig::Custom(_) => Default::default(),
        }
    }
}

impl Config {
    /// Provide key-value list of default colors.
    pub const fn default_colors(&self) -> &[(&'static str, &'static str)] {
        const DIVIDER: &str = fg(4);
        const DUE: &str = fg(15);
        const HEADER: &str = fg(15);
        const OVERDUE: &str = fg(9);
        const TAG: &str = fg(13);
        const URGENCY: &str = fg(2);
        const WHEN: &str = fg(12);
        const SPACER: &str = fg(8);

        &[
            ("divider", DIVIDER),
            ("due", DUE),
            ("header", HEADER),
            ("overdue", OVERDUE),
            ("spacer", SPACER),
            ("tag", TAG),
            ("urgency", URGENCY),
            ("when", WHEN),
        ]

        // TODO: P3: add default colors here and to templates
    }
}
