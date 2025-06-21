use std::io::{Write, stdin, stdout};

use crate::prelude::*;

/// Read user input from stdin.
pub fn prompt(prompt: &str) -> Result<String> {
    let mut input = String::new();

    print!("{}", prompt);
    stdout().flush()?;

    stdin().read_line(&mut input)?;
    input = input.trim().to_string();

    Ok(input)
}
