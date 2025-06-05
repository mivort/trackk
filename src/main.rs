mod args;
mod bucket;
mod config;
mod dateexp;
mod display;
mod editor;
mod filter;
mod index;
mod issue;
mod prelude;
mod repo;
mod storage;

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

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
    cache: RefCell<HashMap<String, Rc<bucket::Bucket>>>,
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
        cache: Default::default(),
    };

    app.filter = filter::parse_filter_args(&args, &app)?;

    match args.command {
        Some(Command::List) => {
            display::show_entries(&storage::fetch_entries(&args.filter_args, &app)?);
        }
        Some(Command::Edit) => {
            editor::edit_entries(&app)?;
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
            storage::modify_entries(&e, &app)?;
        }
        Some(Command::Done) => {
            let args = args::ModArgs {
                entry: args::EntryArgs {
                    status: Some(app.config.defaults.status_complete.clone()),
                    ..Default::default()
                },
            };
            storage::modify_entries(&args, &app)?;
        }
        Some(Command::Init) => {
            repo::init_repo(&app.config)?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        None => {
            let entries = storage::fetch_entries(&args.filter_args, &app)?;

            if !app.filter.ids.is_empty() {
                for entry in &entries {
                    display::show_entry(entry);
                }
            } else {
                display::show_entries(&entries);
            }
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
