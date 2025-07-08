use crate::templates::colors::{RESET, bg, fg};
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
    #[serde(default)]
    pub fg: ColorValue,
    #[serde(default)]
    pub bg: ColorValue,
    #[serde(default)]
    _bold: bool,
    #[serde(default)]
    _italic: bool,
    #[serde(default)]
    _underscore: bool,
    #[serde(default)]
    _inversed: bool,
    #[serde(default)]
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
        const DATE_DUE: &str = fg(15);
        const DATE_END: &str = fg(6);
        const DATE_OVERDUE: &str = fg(9);
        const DATE_WHEN: &str = fg(12);
        const DESC_COMPLETED: &str = fg(2);
        const DESC_DELETED: &str = fg(8);
        const DESC_PENDING: &str = RESET;
        const DIVIDER: &str = fg(4);
        const ENTRY_DATE: &str = fg(15);
        const ENTRY_FIELD: &str = fg(12);
        const ENTRY_NO_VALUE: &str = fg(8);
        const ENTRY_VALUE: &str = RESET;
        const HEADER: &str = fg(15);
        const MARK_BLOCKED: &str = fg(1);
        const MARK_NOTE: &str = fg(6);
        const MARK_REPEAT: &str = fg(6);
        const MARK_STARTED: &str = fg(12);
        const MORE: &str = fg(8);
        const SID_PENDING: &str = fg(12);
        const SPACER: &str = fg(8);
        const TAG: &str = fg(5);
        const URGENCY: &str = fg(2);
        const UUID: &str = fg(4);
        const UUID_COMPLETED: &str = fg(10);
        const UUID_DELETED: &str = fg(8);

        &[
            ("date_due", DATE_DUE),
            ("date_end", DATE_END),
            ("date_overdue", DATE_OVERDUE),
            ("date_when", DATE_WHEN),
            ("desc_completed", DESC_COMPLETED),
            ("desc_deleted", DESC_DELETED),
            ("desc_deleted", DESC_DELETED),
            ("desc_pending", DESC_PENDING),
            ("divider", DIVIDER),
            ("entry_date", ENTRY_DATE),
            ("entry_field", ENTRY_FIELD),
            ("entry_no_value", ENTRY_NO_VALUE),
            ("entry_value", ENTRY_VALUE),
            ("header", HEADER),
            ("mark_blocked", MARK_BLOCKED),
            ("mark_note", MARK_NOTE),
            ("mark_repeat", MARK_REPEAT),
            ("mark_started", MARK_STARTED),
            ("more", MORE),
            ("sid_pending", SID_PENDING),
            ("spacer", SPACER),
            ("tag", TAG),
            ("urgency", URGENCY),
            ("uuid", UUID),
            ("uuid_completed", UUID_COMPLETED),
            ("uuid_deleted", UUID_DELETED),
        ]

        // TODO: P3: add default colors here and to templates
    }
}
