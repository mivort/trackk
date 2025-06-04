mod args;
mod bucket;
mod config;
mod display;
mod editor;
mod filter;
mod index;
mod issue;
mod prelude;
mod repo;
mod storage;

use std::cell::OnceCell;
use std::collections::HashMap;

use args::{Args, Command};
use clap::Parser;
use config::Config;
use prelude::*;

pub struct App {
    /// Application config.
    config: config::Config,

    /// Active entry index.
    index: OnceCell<index::Index>,

    /// Current global entry filter.
    filter: filter::Filter,

    /// Parsed entries cache.
    _cache: HashMap<String, issue::Issue>,
}

impl App {
    /// Load load and access the active entry index.
    pub fn index(&self) -> Result<&index::Index> {
        // TODO: replace with 'get_or_try_init' once it gets stable
        if let Some(index) = self.index.get() {
            return Ok(index);
        }

        let index = index::Index::load(&self.config)?;

        self.index.set(index).unwrap();
        Ok(self.index.get().unwrap())
    }

    /// If index is loaded, clone lazy-initialized value. Otherwise, produce an independent clone.
    pub fn index_owned(&self) -> Result<index::Index> {
        if let Some(index) = self.index.get() {
            return Ok(index.clone());
        }

        index::Index::load(&self.config)
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = App {
        config: read_config(&args.data),
        index: Default::default(),
        filter: Default::default(),
        _cache: Default::default(),
    };

    app.filter = filter::parse_filter_args(&args, &app)?;

    let mut filter = args.filter_args;

    match args.command {
        Some(Command::List) => {
            let index = app.index()?;
            if !filter.resolve_shorthands(index) {
                println!("No results.");
                return Ok(());
            }
            display::show_entries(&storage::fetch_entries(&filter, &app)?);
        }
        Some(Command::Edit) => {
            let index = app.index()?;
            if !filter.resolve_shorthands(index) {
                println!("No entries to edit.");
                return Ok(());
            }
            editor::edit_entries(&filter, &app)?;
        }
        Some(Command::Add(a)) => {
            let mut issue = issue::Issue::new(&a.entry, &app);

            if !a.no_edit {
                editor::edit_entry(&mut issue, &app)?;
            }
            storage::add_entry(issue, &app)?;
        }
        Some(Command::Log(a)) => {
            let mut issue = issue::Issue::new(&a.entry, &app);
            issue.status = app.config.defaults.status_complete.clone();
            issue.update_end_ts();

            if !a.no_edit {
                editor::edit_entry(&mut issue, &app)?;
            }
            storage::add_entry(issue, &app)?;
        }
        Some(Command::Modify(e)) => {
            let index = app.index()?;
            if !filter.resolve_shorthands(&index) {
                println!("No entries to modify.");
                return Ok(());
            }
            storage::modify_entries(&e, &filter, &app)?;
        }
        Some(Command::Done) => {
            let args = args::ModArgs {
                entry: args::EntryArgs {
                    status: Some(app.config.defaults.status_complete.clone()),
                    ..Default::default()
                },
            };
            storage::modify_entries(&args, &filter, &app)?;
        }
        Some(Command::Init) => {
            repo::init_repo(&app.config)?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        None => {
            display::show_entries(&storage::fetch_entries(&Default::default(), &app)?);
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
