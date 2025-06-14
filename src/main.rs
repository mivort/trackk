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

use std::cell::{Ref, RefCell, RefMut};
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
    index: RefCell<index::Index>,

    /// Current global entry filter.
    filter: filter::Filter,

    /// UTC timestamp during the init.
    ts: i64,

    /// Parsed entries cache.
    cache: RefCell<HashMap<String, Rc<bucket::Bucket>>>,
}

impl App {
    /// Lazy load and access the active entry index.
    pub fn index(&self) -> Result<Ref<'_, index::Index>> {
        let index = self.index.borrow();
        if index.loaded() {
            return Ok(index);
        }
        drop(index);

        let mut index = self.index.borrow_mut();
        index.load(&self.config)?;
        drop(index);

        Ok(self.index.borrow())
    }

    /// Load load and get mutable reference to the index.
    pub fn index_mut(&self) -> Result<RefMut<'_, index::Index>> {
        let mut index = self.index.borrow_mut();
        if !index.loaded() {
            index.load(&self.config)?;
        }
        Ok(index)
    }

    /// Convert start timestamp to time with offset.
    pub fn local_time(&self) -> Result<time::OffsetDateTime> {
        use time::*;

        let utc = UtcDateTime::from_unix_timestamp(self.ts)?;
        Ok(utc.to_offset(UtcOffset::current_local_offset()?))
    }
}

fn main() -> Result<()> {
    use log::LevelFilter::*;

    let args = Args::parse();

    let mut app = App {
        config: read_config(&args.data),
        ts: time::UtcDateTime::now().unix_timestamp(),
        ..Default::default()
    };

    app.filter = filter::parse_filter_args(&args, &app)?;

    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("{}: {}", record.level(), message)))
        .level(if args.verbose { Trace } else { Info })
        .chain(std::io::stdout())
        .apply()?;

    match args.command {
        Some(Command::List(args)) => {
            let ids = Default::default();
            let entries = storage::fetch_entries(&ids, &app, args.all)?;
            if args.json {
                display::show_json(&entries)?;
                return Ok(());
            }
            display::show_entries(&entries);
        }
        Some(Command::Info(args)) => {
            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            let entries = storage::filter_active_entries(&ids, &app)?;

            for entry in &entries {
                display::show_entry(entry);
            }
        }
        Some(Command::All) => {
            let ids = Default::default();
            display::show_entries(&storage::filter_all_entries(&ids, &app)?);
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
            issue.status = app.config.defaults.status_complete().to_string();
            issue.update_end(&app.config);

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
                args.entry.status = Some(app.config.defaults.status_complete().to_string());
            }
            storage::modify_entries(&ids, &args.entry, &app)?;
        }
        Some(Command::Remove(mut args)) => {
            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            if args.entry.status.is_none() {
                args.entry.status = Some(app.config.defaults.status_deleted().to_string());
            }
            storage::modify_entries(&ids, &args.entry, &app)?;
        }
        Some(Command::Refresh(args)) => {
            storage::refresh_index(&app, args.force)?;
        }
        Some(Command::Init) => {
            repo::init_repo(&app.config)?;
        }
        Some(Command::Check) => {
            repo::check_repo();
        }
        Some(Command::Merge(_)) => {
            // TODO: P3: implement merge driver
        }
        Some(Command::Report(report)) => {
            // TODO: P2: handle custom reports
            bail!(
                "Custom report config '{}' not found",
                report.first().unwrap()
            );
        }
        None => {
            let ids = Default::default();
            let entries = storage::filter_active_entries(&ids, &app)?;

            if !app.filter.ids.is_empty() {
                for entry in &entries {
                    display::show_entry(entry);
                }
            } else {
                display::show_entries(&entries);
            }
        }
    }

    Ok(())
}

/// Read config from file and (optionally) from storage directory.
fn read_config(data: &Option<String>) -> Config {
    let mut config = Config::default(); // TODO: P3: use argument to read config
    config.set_data_directory(data.clone());
    config.fallback_values();
    config
}
