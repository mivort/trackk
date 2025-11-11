use std::fmt::Write;

use crate::templates::colors::{BOLD, ITALIC, UNDERLINE, bg, defaults, fg};
use serde_derive::Deserialize;

use super::Config;
use crate::prelude::*;

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(unused)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub enum ColorConfig {
    Options(ColorOptions),
    Custom(Box<str>),
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ColorOptions {
    #[serde(default)]
    pub fg: Option<u8>,

    #[serde(default)]
    pub bg: Option<u8>,

    #[serde(default)]
    pub fg_rgb: i32,

    #[serde(default)]
    pub bg_rgb: i32,

    #[serde(default)]
    bold: bool,

    #[serde(default)]
    italic: bool,

    #[serde(default)]
    underline: bool,

    #[serde(default)]
    _inversed: bool,

    #[serde(default)]
    _crossed_out: bool,
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            fg_rgb: -1,
            bg_rgb: -1,

            bold: false,
            italic: false,
            underline: false,
            _inversed: false,
            _crossed_out: false,
        }
    }
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
                Self::format_fg(options, &mut res);
                Self::format_bg(options, &mut res);
                Self::format_style(options, &mut res);
                res
            }
            ColorConfig::Custom(_) => Default::default(),
        }
    }

    /// Check if RGB value is not defined and set indexed fg color.
    fn format_fg(options: &ColorOptions, out: &mut String) {
        if options.fg_rgb > 0 {
            let [b, g, r, _] = options.fg_rgb.to_le_bytes();
            let _ = write!(out, "{}", anstyle::RgbColor(r, g, b).render_fg());
            return;
        }
        out.push_str(fg(unwrap_some_or!(options.fg, { return })));
    }

    /// Check if RGB value is not defined and set indexed bg color.
    fn format_bg(options: &ColorOptions, out: &mut String) {
        if options.bg_rgb > 0 {
            let [b, g, r, _] = options.bg_rgb.to_le_bytes();
            let _ = write!(out, "{}", anstyle::RgbColor(r, g, b).render_bg());
            return;
        }
        out.push_str(bg(unwrap_some_or!(options.bg, { return })));
    }

    /// Write additional drawing options (bold, underline, etc.).
    fn format_style(options: &ColorOptions, out: &mut String) {
        if options.bold {
            let _ = write!(out, "{}", BOLD);
        }
        if options.italic {
            let _ = write!(out, "{}", ITALIC);
        }
        if options.underline {
            let _ = write!(out, "{}", UNDERLINE);
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
        use defaults::*;

        &[
            ("date_due", DATE_DUE),
            ("date_end", DATE_END),
            ("date_overdue", DATE_OVERDUE),
            ("date_when", DATE_WHEN),
            ("desc_completed", DESC_COMPLETED),
            ("desc_deleted", DESC_DELETED),
            ("desc_deleted", DESC_DELETED),
            ("desc_due", DESC_DUE),
            ("desc_overdue", DESC_OVERDUE),
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
