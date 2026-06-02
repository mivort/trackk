pub mod colors;
pub mod dates;
pub mod layout;
pub mod strings;

use builtin_templates as bt;
use minijinja as mj;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use time::OffsetDateTime;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::config::Config;
use crate::prelude::*;

/// Rendering template lazy loader.
pub struct Templates<'env> {
    pub j2: mj::Environment<'env>,

    /// Flag if initial lazy setup was done.
    init: bool,
}

impl<'env> Default for Templates<'env> {
    fn default() -> Self {
        Self {
            j2: mj::Environment::new(),
            init: false,
        }
    }
}

impl<'env> Templates<'env> {
    /// Initialize the templating environment.
    pub fn init(&mut self, ts: OffsetDateTime, config: &'env Config) -> Result<()> {
        use terminal_size::*;

        if self.init {
            return Ok(());
        }

        let offset = ts.offset().whole_seconds() as i64;
        let now = ts.unix_timestamp();
        let today = now - (now + offset) % 86400;

        let j2 = &mut self.j2;
        j2.set_keep_trailing_newline(true);
        j2.set_auto_escape_callback(|_| mj::AutoEscape::None);

        j2.add_filter("numfmt", strings::numfmt); // TODO: P1: deprecate this method

        j2.add_filter("lpad", strings::lpad);
        j2.add_filter("rpad", strings::rpad);

        j2.add_filter("firstline", strings::firstline);
        j2.add_filter("hasnote", strings::hasnote);

        j2.add_filter("reldate", move |d: i64, p: Option<i32>| {
            dates::reldate(d, now, p)
        });
        j2.add_filter("reltoday", move |d: i64, p: Option<i32>| {
            dates::reldate(d, today, p)
        });
        j2.add_filter("longreldate", move |d: i64, p: Option<i32>| {
            dates::longreldate(d, now, p)
        });
        j2.add_filter("longreltoday", move |d: i64, p: Option<i32>| {
            dates::longreldate(d, today, p)
        });

        j2.add_global("now", now);
        j2.add_global("today", today);

        let formats = dates::parse_formats(&config.date_formats)?;
        let offset = time::UtcOffset::current_local_offset()?;
        j2.add_filter("datefmt", move |ts: i64, fmt: Option<&str>| {
            dates::datefmt(ts, fmt, &formats, offset)
        });

        j2.add_filter("uwidth", |s: &str| s.width());
        j2.add_filter("width", layout::width);
        j2.add_filter("trunc", layout::trunc);
        j2.add_function("fill", layout::fill);

        let (Width(cols), Height(rows)) = terminal_size().unwrap_or((Width(0), Height(0)));
        j2.add_global("cols", cols);
        j2.add_global("rows", rows);

        j2.add_global("black", anstyle::AnsiColor::Black as u8);
        j2.add_global("red", anstyle::AnsiColor::Red as u8);
        j2.add_global("green", anstyle::AnsiColor::Green as u8);
        j2.add_global("yellow", anstyle::AnsiColor::Yellow as u8);
        j2.add_global("blue", anstyle::AnsiColor::Blue as u8);
        j2.add_global("magenta", anstyle::AnsiColor::Magenta as u8);
        j2.add_global("cyan", anstyle::AnsiColor::Cyan as u8);
        j2.add_global("white", anstyle::AnsiColor::White as u8);

        j2.add_global("lightblack", anstyle::AnsiColor::BrightBlack as u8);
        j2.add_global("lightred", anstyle::AnsiColor::BrightRed as u8);
        j2.add_global("lightgreen", anstyle::AnsiColor::BrightGreen as u8);
        j2.add_global("lightyellow", anstyle::AnsiColor::BrightYellow as u8);
        j2.add_global("lightblue", anstyle::AnsiColor::BrightBlue as u8);
        j2.add_global("lightmagenta", anstyle::AnsiColor::BrightMagenta as u8);
        j2.add_global("lightcyan", anstyle::AnsiColor::BrightCyan as u8);
        j2.add_global("lightwhite", anstyle::AnsiColor::BrightWhite as u8);

        let mut colors = HashMap::<&'static str, &'static str>::new();
        let mut tags = HashMap::<&'static str, &'static str>::new();

        if config.no_color() {
            j2.add_function("fg", |_: u8| "");
            j2.add_function("bg", |_: u8| "");
        } else {
            j2.add_function("fg", colors::fg);
            j2.add_function("bg", colors::bg);

            j2.add_global("reset", colors::RESET);
            j2.add_global("bold", colors::BOLD);
            j2.add_global("italic", colors::ITALIC);
            j2.add_global("underline", colors::UNDERLINE);
            j2.add_global("inverse", colors::INVERSE);
            j2.add_global("crossedout", colors::CROSSEDOUT);

            for (key, value) in &config.templates.colors {
                let key = key.clone().leak();
                let val = value.format().leak();
                colors.insert(key, val);
            }
            for (key, value) in config.default_colors() {
                colors.entry(key).or_insert(*value);
            }
            for (key, value) in &config.templates.tags {
                let key = key.clone().leak();
                let val = value.format().leak();
                tags.insert(key, val);
            }
        }

        j2.add_global("c", colors);
        j2.add_global("tc", tags);

        j2.add_function("min", |a: i32, b: i32| a.min(b));
        j2.add_function("max", |a: i32, b: i32| a.max(b));

        self.init = true;

        Ok(())
    }

    /// Check template ID existence, if template doesn't exist yet - load and parse it.
    pub fn load_template(&mut self, template: &str, app: &App) -> Result<()> {
        let j2 = &mut self.j2;
        let err = unwrap_err_or!(j2.get_template(template), _, { return Ok(()) });

        if !matches!(err.kind(), mj::ErrorKind::TemplateNotFound) {
            return Err(anyhow!(err));
        }

        if let Some((id, content)) = builtin_template(template) {
            j2.add_template(id, content)?;
            return Ok(());
        }

        let mut template_path = app.config.config_path().clone();
        template_path.push(template);
        match fs::read_to_string(&template_path) {
            Ok(template_data) => {
                j2.add_template(template.to_owned().leak(), template_data.leak())?;
                return Ok(());
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(anyhow!(err)),
        }

        let mut template_path = app.config.data_path().with_context(|| {
            format!("Unable to read template in config or data directory: {template}")
        })?;
        template_path.push(template);

        let template_data = fs::read_to_string(&template_path)?;
        j2.add_template(template.to_owned().leak(), template_data.leak())?;

        Ok(())
    }
}

/// Builtin template names.
pub mod builtin_templates {
    pub const HEADER: &str = "header";
    pub const GROUP_DAY: &str = "group_day";
    pub const UTILS: &str = "utils";
    pub const NEXT: &str = "next";
    pub const ALL: &str = "all";
    pub const CALENDAR: &str = "calendar";
    pub const ISSUE: &str = "issue";
    pub const ENTRY: &str = "entry";
    pub const PICKER: &str = "picker";
    pub const NONE: &str = "none";
}

/// Return one of the built-in templates.
pub fn builtin_template(template: &str) -> Option<(&'static str, &'static str)> {
    use builtin_templates as bt;

    const ROW: &str = include_str!("../../templates/row.jinja");
    const ROW_TIME: &str = include_str!("../../templates/row_time.jinja");
    const ENTRY: &str = include_str!("../../templates/entry.jinja");
    const HEADER: &str = include_str!("../../templates/header.jinja");
    const GROUP_DAY: &str = include_str!("../../templates/group_day.jinja");
    const UTILS: &str = include_str!("../../templates/utils.jinja");

    match template {
        bt::HEADER => Some((bt::HEADER, HEADER)),
        bt::GROUP_DAY => Some((bt::GROUP_DAY, GROUP_DAY)),
        bt::NEXT => Some((bt::NEXT, ROW)),
        bt::ALL => Some((bt::ALL, ROW)),
        bt::CALENDAR => Some((bt::CALENDAR, ROW_TIME)),
        bt::ISSUE => Some((bt::ISSUE, ENTRY)),
        bt::ENTRY => Some((bt::ENTRY, ENTRY)),
        bt::PICKER => Some((bt::PICKER, ROW)),
        bt::UTILS => Some((bt::UTILS, UTILS)),
        bt::NONE => Some((bt::NONE, "")),
        _ => None,
    }
}

/// List of built-in templates.
const BUILTIN_TEMPLATES: [&str; 10] = [
    bt::HEADER,
    bt::GROUP_DAY,
    bt::NEXT,
    bt::ALL,
    bt::CALENDAR,
    bt::ISSUE,
    bt::ENTRY,
    bt::PICKER,
    bt::NONE,
    bt::UTILS,
];

/// Print the list of available templates.
pub fn print_builtin_templates() {
    for t in BUILTIN_TEMPLATES {
        println!("{}", t);
    }
}

#[test]
fn check_builtin_list() {
    for b in BUILTIN_TEMPLATES {
        assert!(builtin_template(b).unwrap().0 == b);
    }
}
