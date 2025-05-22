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

    match &args.command {
        Some(Command::List(f)) => {
            display::show_entries(&storage::fetch_entries(f, &read_config(&args))?);
        }
        Some(Command::Add(e)) => {
            storage::add_entry(e, &read_config(&args))?;
        }
        Some(Command::Modify(e)) => {
            storage::modify_entries(e, &read_config(&args))?;
        }
        Some(Command::Init) => {
            repo::init_repo(&read_config(&args))?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        None => {
            display::show_entries(&storage::fetch_entries(
                &Default::default(),
                &read_config(&args),
            )?);
        }
        _ => {}
    }

    Ok(())
}

fn read_config(args: &Args) -> Config {
    let mut config = Config::default(); // TODO: use argument to read config
    config.set_data_directory(args.data.clone());
    config.fallback_values();
    config
}
