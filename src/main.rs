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
mod token;

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use args::{Args, Command};
use clap::Parser;
use config::Config;
use prelude::*;

#[derive(Default)]
pub struct App {
    /// Application config.
    config: config::Config,

    /// Active entry index.
    index: OnceCell<index::Index>,

    /// Current global entry filter.
    filter: filter::Filter,

    /// UTC timestamp during the init.
    ts: i64,

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

    /// Convert start timestamp to time with offset.
    pub fn local_time(&self) -> Result<time::OffsetDateTime> {
        use time::*;

        let utc = UtcDateTime::from_unix_timestamp(self.ts)?;
        Ok(utc.to_offset(UtcOffset::current_local_offset()?))
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = App {
        config: read_config(&args.data),
        ts: time::UtcDateTime::now().unix_timestamp(),
        ..Default::default()
    };

    app.filter = filter::parse_filter_args(&args, &app)?;

    match args.command {
        Some(Command::List) => {
            let ids = filter::IdFilter::new();
            display::show_entries(&storage::fetch_entries(&ids, &args.filter_args, &app)?);
        }
        Some(Command::Edit(args)) => {
            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            editor::edit_entries(&ids, &app)?;
        }
        Some(Command::Add(a)) => {
            let mut issue = issue::Issue::new(&a.entry, &app)?;

            if !a.no_edit {
                let status = editor::edit_entry(&mut issue, &app)?;
                if !status.success() {
                    return Ok(());
                }
            }
            storage::add_entry(issue, &app)?;
        }
        Some(Command::Log(a)) => {
            let mut issue = issue::Issue::new(&a.entry, &app)?;
            issue.status = app.config.defaults.status_complete.clone();
            issue.update_end_ts();

            if !a.no_edit {
                editor::edit_entry(&mut issue, &app)?;
            }
            storage::add_entry(issue, &app)?;
        }
        Some(Command::Modify(e)) => {
            let ids = filter::IdFilter::from_shorthands(e.ids, &app)?;
            storage::modify_entries(&ids, &e.entry, &app)?;
        }
        Some(Command::Done(mut args)) => {
            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            if args.entry.status.is_none() {
                args.entry.status = Some(app.config.defaults.status_complete.clone());
            }
            storage::modify_entries(&ids, &args.entry, &app)?;
        }
        Some(Command::Init) => {
            repo::init_repo(&app.config)?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        Some(Command::Report(_)) => {
            println!("Custom reports are not supported yet");
        }
        None => {
            let ids = filter::IdFilter::new();
            let entries = storage::fetch_entries(&ids, &args.filter_args, &app)?;

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
