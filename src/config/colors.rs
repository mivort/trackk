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
    pub fg: ColorValue,
    pub bg: ColorValue,
    _bold: bool,
    _italic: bool,
    _underscore: bool,
    _inversed: bool,
    _crossed_out: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(unused)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub enum ColorValue {
    Indexed(Option<u8>),
    Rgb(Box<str>),
}

impl ColorConfig {
    /// Convert color config entry to escape sequence.
    pub fn format(&self) -> String {
        match self {
            ColorConfig::Options(options) => {
                let mut res = String::new();
                options.fg.format_fg(&mut res);
                options.bg.format_bg(&mut res);
                res
            }
            ColorConfig::Custom(_) => Default::default(),
        }
    }
}

impl ColorValue {
    /// Copy format value to output string.
    pub fn format_fg(&self, out: &mut String) {
        match self {
            ColorValue::Indexed(Some(v)) => out.push_str(fg(*v)),
            ColorValue::Rgb(_) => todo!(),
            _ => {}
        }
    }

    /// Copy format value to output string.
    pub fn format_bg(&self, out: &mut String) {
        match self {
            ColorValue::Indexed(Some(v)) => out.push_str(bg(*v)),
            ColorValue::Rgb(_) => todo!(),
            _ => {}
        }
    }
}

impl Default for ColorValue {
    fn default() -> Self {
        Self::Indexed(None)
    }
}

impl Config {
    /// Provide key-value list of default colors.
    pub const fn default_colors(&self) -> &[(&'static str, &'static str)] {
        const BLOCKED: &str = fg(1);
        const DIVIDER: &str = fg(4);
        const DUE: &str = fg(15);
        const END: &str = fg(6);
        const HEADER: &str = fg(15);
        const OVERDUE: &str = fg(9);
        const SPACER: &str = fg(8);
        const STARTED: &str = fg(12);
        const TAG: &str = fg(13);
        const URGENCY: &str = fg(2);
        const WHEN: &str = fg(12);

        &[
            ("blocked", BLOCKED),
            ("divider", DIVIDER),
            ("due", DUE),
            ("end", END),
            ("header", HEADER),
            ("overdue", OVERDUE),
            ("spacer", SPACER),
            ("started", STARTED),
            ("tag", TAG),
            ("urgency", URGENCY),
            ("when", WHEN),
        ]

        // TODO: P3: add default colors here and to templates
    }
}
