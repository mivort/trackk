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

macro_rules! escape {
    (a16: $code:literal) => {
        concat!("\x1B[", stringify!($code), "m")
    };
    (a256: $code:literal) => {
        concat!("\x1B[38;5;", stringify!($code), "m")
    };
}

macro_rules! escape_match {
    ($var:expr, $($code:literal [ $cat:ident / $res:literal ])*) => {
        match $var {
            $( $code => escape!($cat : $res), )*
        }
    };
}

/// Set foreground color using the value from 0 to 255.
const fn fg(color: u8) -> &'static str {
    escape_match!(color,
        00[a16/30] 01[a16/31] 02[a16/32] 03[a16/33] 04[a16/34] 05[a16/35] 06[a16/36] 07[a16/37]
        08[a16/90] 09[a16/91] 10[a16/92] 11[a16/93] 12[a16/94] 13[a16/95] 14[a16/96] 15[a16/97]

        016[a256/016] 017[a256/017] 018[a256/018] 019[a256/019] 020[a256/020] 021[a256/021] 022[a256/022] 023[a256/023]
        024[a256/024] 025[a256/025] 026[a256/026] 027[a256/027] 028[a256/028] 029[a256/029] 030[a256/030] 031[a256/031]
        032[a256/032] 033[a256/033] 034[a256/034] 035[a256/035] 036[a256/036] 037[a256/037] 038[a256/038] 039[a256/039]
        040[a256/040] 041[a256/041] 042[a256/042] 043[a256/043] 044[a256/044] 045[a256/045] 046[a256/046] 047[a256/047]
        048[a256/048] 049[a256/049] 050[a256/050] 051[a256/051] 052[a256/052] 053[a256/053] 054[a256/054] 055[a256/055]
        056[a256/056] 057[a256/057] 058[a256/058] 059[a256/059] 060[a256/060] 061[a256/061] 062[a256/062] 063[a256/063]
        064[a256/064] 065[a256/065] 066[a256/066] 067[a256/067] 068[a256/068] 069[a256/069] 070[a256/070] 071[a256/071]
        072[a256/072] 073[a256/073] 074[a256/074] 075[a256/075] 076[a256/076] 077[a256/077] 078[a256/078] 079[a256/079]
        080[a256/080] 081[a256/081] 082[a256/082] 083[a256/083] 084[a256/084] 085[a256/085] 086[a256/086] 087[a256/087]
        088[a256/088] 089[a256/089] 090[a256/090] 091[a256/091] 092[a256/092] 093[a256/093] 094[a256/094] 095[a256/095]
        096[a256/096] 097[a256/097] 098[a256/098] 099[a256/099] 100[a256/100] 101[a256/101] 102[a256/102] 103[a256/103]
        104[a256/104] 105[a256/105] 106[a256/106] 107[a256/107] 108[a256/108] 109[a256/109] 110[a256/110] 111[a256/111]
        112[a256/112] 113[a256/113] 114[a256/114] 115[a256/115] 116[a256/116] 117[a256/117] 118[a256/118] 119[a256/119]
        120[a256/120] 121[a256/121] 122[a256/122] 123[a256/123] 124[a256/124] 125[a256/125] 126[a256/126] 127[a256/127]
        128[a256/128] 129[a256/129] 130[a256/130] 131[a256/131] 132[a256/132] 133[a256/133] 134[a256/134] 135[a256/135]
        136[a256/136] 137[a256/137] 138[a256/138] 139[a256/139] 140[a256/140] 141[a256/141] 142[a256/142] 143[a256/143]
        144[a256/144] 145[a256/145] 146[a256/146] 147[a256/147] 148[a256/148] 149[a256/149] 150[a256/150] 151[a256/151]
        152[a256/152] 153[a256/153] 154[a256/154] 155[a256/155] 156[a256/156] 157[a256/157] 158[a256/158] 159[a256/159]
        160[a256/160] 161[a256/161] 162[a256/162] 163[a256/163] 164[a256/164] 165[a256/165] 166[a256/166] 167[a256/167]
        168[a256/168] 169[a256/169] 170[a256/170] 171[a256/171] 172[a256/172] 173[a256/173] 174[a256/174] 175[a256/175]
        176[a256/176] 177[a256/177] 178[a256/178] 179[a256/179] 180[a256/180] 181[a256/181] 182[a256/182] 183[a256/183]
        184[a256/184] 185[a256/185] 186[a256/186] 187[a256/187] 188[a256/188] 189[a256/189] 190[a256/190] 191[a256/191]
        192[a256/192] 193[a256/193] 194[a256/194] 195[a256/195] 196[a256/196] 197[a256/197] 198[a256/198] 199[a256/199]
        200[a256/200] 201[a256/201] 202[a256/202] 203[a256/203] 204[a256/204] 205[a256/205] 206[a256/206] 207[a256/207]
        208[a256/208] 209[a256/209] 210[a256/210] 211[a256/211] 212[a256/212] 213[a256/213] 214[a256/214] 215[a256/215]
        216[a256/216] 217[a256/217] 218[a256/218] 219[a256/219] 220[a256/220] 221[a256/221] 222[a256/222] 223[a256/223]
        224[a256/224] 225[a256/225] 226[a256/226] 227[a256/227] 228[a256/228] 229[a256/229] 230[a256/230] 231[a256/231]
        232[a256/232] 233[a256/233] 234[a256/234] 235[a256/235] 236[a256/236] 237[a256/237] 238[a256/238] 239[a256/239]
        240[a256/240] 241[a256/241] 242[a256/242] 243[a256/243] 244[a256/244] 245[a256/245] 246[a256/246] 247[a256/247]
        248[a256/248] 249[a256/249] 250[a256/250] 251[a256/251] 252[a256/252] 253[a256/253] 254[a256/254] 255[a256/255]
    )
}
