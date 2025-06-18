use minijinja as mj;
use std::cell::{Cell, RefCell};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::args::ColorMode;
use crate::templates::colors;
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

        j2.add_filter("uwidth", |s: &str| s.width());
        j2.add_filter("width", width);
        j2.add_filter("trunc", trunc);

        let (Width(cols), Height(rows)) = terminal_size().unwrap_or((Width(0), Height(0)));
        j2.add_global("cols", cols);
        j2.add_global("rows", rows);

        if !matches!(app.config.color_mode, ColorMode::Never) {
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

            j2.add_global("reset", anstyle::Reset.render().to_string());
            j2.add_function("fg", colors::fg);
            j2.add_function("bg", colors::bg);

            // TODO: P2: add underline, bold and italic
        } else {
            j2.add_function("fg", |_: u8| "");
            j2.add_function("bg", |_: u8| "");
        }

        j2.add_function("fill", fill);
        j2.add_function("min", |a: i32, b: i32| a.min(b));
        j2.add_function("max", |a: i32, b: i32| a.max(b));

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

/// Produce the string by repeating the character N times.
fn fill(value: &str, repeat: i32) -> String {
    (0..repeat).map(|_| value).collect()
}

/// Iterate over Unicode segments and count length excluding escape sequences.
fn width(input: &str) -> usize {
    use vte::{Parser, Perform};

    let mut parser = Parser::new();

    struct Performer {
        printable: bool,
        count: usize,
    }

    let mut performer = Performer {
        count: 0,
        printable: false,
    };

    impl Perform for Performer {
        fn print(&mut self, _c: char) {
            self.printable = true
        }
    }

    for g in input.graphemes(true) {
        parser.advance(&mut performer, g.as_bytes());
        if performer.printable {
            performer.count += g.width();
            performer.printable = false;
        }
    }

    performer.count
}

/// Truncate string to a maximum length and add optional character at the end.
fn trunc(mut input: String, max: i32, _end: Option<&str>) -> String {
    // TODO: P2: implement more precise truncation
    // TODO: P2: implement truncate chararater support

    use vte::{Parser, Perform};

    let mut parser = Parser::new();

    struct Performer {
        printable: bool,
        byte: usize,
        count: usize,
    }

    let mut performer = Performer {
        count: 0,
        byte: 0,
        printable: false,
    };

    impl Perform for Performer {
        fn print(&mut self, _c: char) {
            self.printable = true
        }
    }

    for g in input.graphemes(true) {
        parser.advance(&mut performer, g.as_bytes());
        if performer.printable {
            performer.count += g.width();

            if performer.count >= max as usize {
                break;
            }

            performer.printable = false;
        }
        performer.byte += g.len();
    }

    input.truncate(performer.byte);
    input
}

#[test]
fn width_no_escapes() {
    assert_eq!(width("\x1B[30mひびぴ\x1B[30m"), 6);
}
