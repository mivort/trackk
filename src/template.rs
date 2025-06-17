use minijinja as mj;
use std::cell::{Cell, RefCell};
use unicode_segmentation::UnicodeSegmentation;

use crate::args::ColorMode;
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
        j2.add_filter("ulength", ulength);

        let (Width(cols), Height(rows)) = terminal_size().unwrap_or((Width(0), Height(0)));
        j2.add_global("cols", cols);
        j2.add_global("rows", rows);

        if !matches!(app.config.color_mode, ColorMode::Never) {
            j2.add_global("green", anstyle::AnsiColor::Green.render_fg().to_string());
            j2.add_global("blue", anstyle::AnsiColor::Blue.render_fg().to_string());
            j2.add_global("reset", anstyle::Reset.render().to_string());
            j2.add_function("fg", fg);
        } else {
            j2.add_function("fg", |_: u8| "");
        }

        self.init.set(true);
    }

    /// Check template ID existence, if template doesn't exist yet - load and parse it.
    pub fn load_template(&self, template: &'env str) -> Result<()> {
        let mut j2 = self.j2.borrow_mut();
        let err = unwrap_err_or!(j2.get_template(template), _, { return Ok(()) });

        if !matches!(err.kind(), mj::ErrorKind::TemplateNotFound) {
            return Err(anyhow!(err));
        }

        match template {
            "next" => j2.add_template(template, include_str!("../templates/row.jinja"))?,
            "all" => j2.add_template(template, include_str!("../templates/row.jinja"))?,

            // TODO: P3: resolve external templates
            _ => panic!(),
        }

        Ok(())
    }
}

/// Use format string to format the value.
fn format(fmt: String, value: String) -> Result<String, mj::Error> {
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

/// Return the number of unicode segents in the string.
fn ulength(input: String) -> usize {
    input.graphemes(true).count()
}

/// Set foreground color using the value from 0 to 255.
fn fg(color: u8) -> String {
    anstyle::Ansi256Color::from(color).render_fg().to_string()
}
