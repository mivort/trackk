use minijinja as mj;

/// Truncate string to only leave the first line.
pub fn firstline(mut input: String) -> String {
    let pos = input.lines().next().unwrap_or_default().len();
    input.truncate(pos);
    input
}

/// Use format string to format the numeric value.
pub fn numfmt(value: f64, fmt: &str) -> Result<String, mj::Error> {
    match formatx::formatx!(fmt, value) {
        Ok(r) => Ok(r),
        Err(e) => Err(mj::Error::new(mj::ErrorKind::SyntaxError, e.to_string())),
    }
}
