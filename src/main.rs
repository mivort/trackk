mod args;
mod bucket;
mod config;
mod display;
mod editor;
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

    match args.command {
        Some(Command::List(f)) => {
            display::show_entries(&storage::fetch_entries(&f, &read_config(&args.data))?);
        }
        Some(Command::Edit(f)) => {
            editor::edit_entries(&f, &read_config(&args.data))?;
        }
        Some(Command::Add(a)) => {
            let config = read_config(&args.data);
            let mut issue = issue::Issue::new(&a.entry, &config);

            if !a.no_edit {
                editor::edit_entry(&mut issue, &config)?;
            }
            storage::add_entry(issue, &read_config(&args.data))?;
        }
        Some(Command::Log(a)) => {
            let config = read_config(&args.data);
            let mut issue = issue::Issue::new(&a.entry, &config);
            issue.status = config.defaults.status_complete.clone();
            issue.update_end_ts();

            if !a.no_edit {
                editor::edit_entry(&mut issue, &config)?;
            }
            storage::add_entry(issue, &read_config(&args.data))?;
        }
        Some(Command::Modify(e)) => {
            storage::modify_entries(&e, &read_config(&args.data))?;
        }
        Some(Command::Done(filter)) => {
            let config = read_config(&args.data);
            let args = args::ModArgs {
                filter,
                entry: args::EntryArgs {
                    status: Some(config.defaults.status_complete.clone()),
                    ..Default::default()
                },
                ..Default::default()
            };
            storage::modify_entries(&args, &config)?;
        }
        Some(Command::Init) => {
            repo::init_repo(&read_config(&args.data))?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        None => {
            display::show_entries(&storage::fetch_entries(
                &Default::default(),
                &read_config(&args.data),
            )?);
        }
        _ => {}
    }

    Ok(())
}

fn read_config(data: &Option<String>) -> Config {
    let mut config = Config::default(); // TODO: use argument to read config
    config.set_data_directory(data.clone());
    config.fallback_values();
    config
}
