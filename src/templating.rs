use minijinja as mj;
use std::cell::{Cell, RefCell};
use unicode_width::UnicodeWidthStr;

use crate::templates::{colors, dates, layout};
use crate::{App, prelude::*};

/// Rendering template lazy loader.
pub struct Templates<'env> {
    pub j2: RefCell<mj::Environment<'env>>,

    /// Flag if initial lazy setup was done.
    init: Cell<bool>,
}

impl<'env> Default for Templates<'env> {
    fn default() -> Self {
        Self {
            j2: RefCell::new(mj::Environment::new()),
            init: Cell::new(false),
        }
    }
}

impl<'env> Templates<'env> {
    /// Initialize the templating environment.
    pub fn init(&self, app: &App) {
        use terminal_size::*;

        if self.init.get() {
            return;
        }

        let mut j2 = self.j2.borrow_mut();
        j2.set_keep_trailing_newline(true);
        j2.set_auto_escape_callback(|_| mj::AutoEscape::None);

        j2.add_filter("format", format);
        j2.add_filter("firstline", firstline);

        let now = app.ts;
        j2.add_filter("reldate", move |d: i64, p: Option<i32>| {
            dates::reldate(d, now, p)
        });
        j2.add_filter("date", || ""); // TODO: P3: add date formatter

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

        if app.config.no_color() {
            j2.add_function("fg", |_: u8| "");
            j2.add_function("bg", |_: u8| "");
        } else {
            j2.add_global("reset", colors::RESET);
            j2.add_global("bold", colors::BOLD);
            j2.add_global("italic", colors::ITALIC);
            j2.add_global("underline", colors::UNDERLINE);

            j2.add_function("fg", colors::fg);
            j2.add_function("bg", colors::bg);
        }

        j2.add_function("min", |a: i32, b: i32| a.min(b));
        j2.add_function("max", |a: i32, b: i32| a.max(b));

        self.init.set(true);
    }

    /// Check template ID existence, if template doesn't exist yet - load and parse it.
    pub fn load_template(&self, template: &str) -> Result<()> {
        let mut j2 = self.j2.borrow_mut();
        let err = unwrap_err_or!(j2.get_template(template), _, { return Ok(()) });

        if !matches!(err.kind(), mj::ErrorKind::TemplateNotFound) {
            return Err(anyhow!(err));
        }

        if let Some((id, content)) = builtin_template(template) {
            j2.add_template(id, content)?;
        } else {
            // TODO: P3: resolve external templates
            todo!()
        }

        Ok(())
    }
}

/// Return one of the built-in templates.
pub fn builtin_template(template: &str) -> Option<(&'static str, &'static str)> {
    const ROW: &str = include_str!("../templates/row.jinja");
    const ISSUE: &str = include_str!("../templates/issue.jinja");

    match template {
        "next" => Some(("next", ROW)),
        "all" => Some(("all", ROW)),
        "issue" => Some(("issue", ISSUE)),
        _ => None,
    }
}

/// Use format string to format the value.
fn format(fmt: &str, value: String) -> Result<String, mj::Error> {
    match formatx::formatx!(fmt, value) {
        Ok(r) => Ok(r),
        Err(e) => Err(mj::Error::new(mj::ErrorKind::SyntaxError, e.to_string())),
    }
}

/// Truncate string to only leave the first line.
fn firstline(mut input: String) -> String {
    let pos = input.lines().next().unwrap_or_default().len();
    input.truncate(pos);
    input
}
