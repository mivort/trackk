mod args;
mod issue;
mod prelude;
mod repo;
mod storage;

use args::{Args, Command};
use clap::Parser;
use prelude::*;

fn main() -> Result<()> {
    let args = Args::parse();

    let command = args.command.unwrap_or_default();
    match command {
        Command::List(_f) => {}
        Command::Add(e) => {
            storage::add_entry(&e);
        }
        Command::Modify(e) => {
            storage::modify_entry(&e);
        }
        Command::CheckRepo => {
            repo::check_repo();
        }
        _ => {}
    }

    Ok(())
}
