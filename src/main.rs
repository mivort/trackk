mod args;
mod config;
mod display;
mod index;
mod issue;
mod prelude;
mod repo;
mod storage;

use args::{Args, Command};
use clap::Parser;
use config::Config;
use prelude::*;

fn main() -> Result<()> {
    let args = Args::parse();

    let mut config = Config::default(); // TODO: use argument to read config
    config.set_data_directory(args.data);
    config.fallback_values();

    let command = args.command.unwrap_or_default();
    match command {
        Command::List(f) => {
            display::show_entries(&storage::fetch_entries(&f, &config)?);
        }
        Command::Add(e) => {
            storage::add_entry(&e, &config)?;
        }
        Command::Modify(e) => {
            storage::modify_entries(&e, &config)?;
        }
        Command::CheckRepo => {
            repo::check_repo();
        }
        _ => {}
    }

    Ok(())
}
