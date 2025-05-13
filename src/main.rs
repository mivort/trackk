mod args;
mod issue;
mod prelude;

use args::{Args, Command};
use clap::Parser;
use prelude::*;

fn main() -> Result<()> {
    let args = Args::parse();

    let command = args.command.unwrap_or_default();
    match command {
        Command::List(_f) => {}
        Command::Add(_e) => {}
        _ => {}
    }

    Ok(())
}
