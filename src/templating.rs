use minijinja as mj;
use std::cell::{Cell, RefCell};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
            j2.add_function("fg", fg);
        } else {
            j2.add_function("fg", |_: u8| "");
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

/// Set foreground color using the value from 0 to 255.
const fn fg(color: u8) -> &'static str {
    /// Macro which adds ANSI escape codes based on provided category.
    macro_rules! escape {
        (a16: $code:literal) => {
            concat!("\x1B[", stringify!($code), "m")
        };
        (a256: $code:literal) => {
            concat!("\x1B[38;5;", stringify!($code), "m")
        };
    }

    /// Compact notation for group of ANSI codes definitions.
    macro_rules! escape_match {
        ($var:expr, $( $cat:ident [ $($code:literal / $res:literal )* ] )* ) => {
            match $var {
                $( $( $code => escape!($cat : $res), )* )*
            }
        };
    }

    escape_match!(color,
        a16[00/30 01/31 02/32 03/33 04/34 05/35 06/36 07/37]
        a16[08/90 09/91 10/92 11/93 12/94 13/95 14/96 15/97]

        a256[016/016 017/017 018/018 019/019 020/020 021/021 022/022 023/023]
        a256[024/024 025/025 026/026 027/027 028/028 029/029 030/030 031/031]
        a256[032/032 033/033 034/034 035/035 036/036 037/037 038/038 039/039]
        a256[040/040 041/041 042/042 043/043 044/044 045/045 046/046 047/047]
        a256[048/048 049/049 050/050 051/051 052/052 053/053 054/054 055/055]
        a256[056/056 057/057 058/058 059/059 060/060 061/061 062/062 063/063]
        a256[064/064 065/065 066/066 067/067 068/068 069/069 070/070 071/071]
        a256[072/072 073/073 074/074 075/075 076/076 077/077 078/078 079/079]
        a256[080/080 081/081 082/082 083/083 084/084 085/085 086/086 087/087]
        a256[088/088 089/089 090/090 091/091 092/092 093/093 094/094 095/095]
        a256[096/096 097/097 098/098 099/099 100/100 101/101 102/102 103/103]
        a256[104/104 105/105 106/106 107/107 108/108 109/109 110/110 111/111]
        a256[112/112 113/113 114/114 115/115 116/116 117/117 118/118 119/119]
        a256[120/120 121/121 122/122 123/123 124/124 125/125 126/126 127/127]
        a256[128/128 129/129 130/130 131/131 132/132 133/133 134/134 135/135]
        a256[136/136 137/137 138/138 139/139 140/140 141/141 142/142 143/143]
        a256[144/144 145/145 146/146 147/147 148/148 149/149 150/150 151/151]
        a256[152/152 153/153 154/154 155/155 156/156 157/157 158/158 159/159]
        a256[160/160 161/161 162/162 163/163 164/164 165/165 166/166 167/167]
        a256[168/168 169/169 170/170 171/171 172/172 173/173 174/174 175/175]
        a256[176/176 177/177 178/178 179/179 180/180 181/181 182/182 183/183]
        a256[184/184 185/185 186/186 187/187 188/188 189/189 190/190 191/191]
        a256[192/192 193/193 194/194 195/195 196/196 197/197 198/198 199/199]
        a256[200/200 201/201 202/202 203/203 204/204 205/205 206/206 207/207]
        a256[208/208 209/209 210/210 211/211 212/212 213/213 214/214 215/215]
        a256[216/216 217/217 218/218 219/219 220/220 221/221 222/222 223/223]
        a256[224/224 225/225 226/226 227/227 228/228 229/229 230/230 231/231]
        a256[232/232 233/233 234/234 235/235 236/236 237/237 238/238 239/239]
        a256[240/240 241/241 242/242 243/243 244/244 245/245 246/246 247/247]
        a256[248/248 249/249 250/250 251/251 252/252 253/253 254/254 255/255]
    )
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
