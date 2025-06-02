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
    let mut filter = args.filter;

    match args.command {
        Some(Command::List) => {
            let config = read_config(&args.data);
            let index = index::Index::load(&config)?;
            if !filter.resolve_shorthands(&index) {
                println!("No results.");
                return Ok(());
            }
            display::show_entries(&storage::fetch_entries(&filter, &config, &index)?);
        }
        Some(Command::Edit) => {
            let config = read_config(&args.data);
            let index = index::Index::load(&config)?;
            if !filter.resolve_shorthands(&index) {
                println!("No entries to edit.");
                return Ok(());
            }
            editor::edit_entries(&filter, &config)?;
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
            let config = read_config(&args.data);
            let mut index = index::Index::load(&config)?;
            if !filter.resolve_shorthands(&index) {
                println!("No entries to modify.");
                return Ok(());
            }
            storage::modify_entries(&e, &filter, &config, &mut index)?;
        }
        Some(Command::Done) => {
            let config = read_config(&args.data);
            let args = args::ModArgs {
                entry: args::EntryArgs {
                    status: Some(config.defaults.status_complete.clone()),
                    ..Default::default()
                },
            };
            let mut index = index::Index::load(&config)?;
            storage::modify_entries(&args, &filter, &config, &mut index)?;
        }
        Some(Command::Init) => {
            repo::init_repo(&read_config(&args.data))?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        Some(Command::External(_args)) => {}
        None => {
            let config = read_config(&args.data);
            let index = index::Index::load(&config)?;
            display::show_entries(&storage::fetch_entries(
                &Default::default(),
                &config,
                &index,
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
