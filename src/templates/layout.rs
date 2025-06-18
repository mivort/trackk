use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Produce the string by repeating the character N times.
pub fn fill(value: &str, repeat: i32) -> String {
    (0..repeat).map(|_| value).collect()
}

/// Iterate over Unicode segments and count length excluding escape sequences.
pub fn width(input: &str) -> usize {
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
pub fn trunc(mut input: String, max: i32, _end: Option<&str>) -> String {
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
