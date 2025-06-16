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
mod template;
mod token;

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::{env, fs, io};

use args::{Args, Command};
use clap::Parser;
use config::Config;
use prelude::*;

use self::config::IndexType;

#[derive(Default)]
pub struct App<'env> {
    /// Application config.
    config: config::Config,

    /// Active entry index.
    index: RefCell<index::Index>,

    /// Current global entry filter.
    filter: filter::Filter,

    /// UTC timestamp during the init.
    ts: i64,

    /// Tera templates reference.
    templates: template::Templates<'env>,

    /// Parsed entries cache.
    cache: RefCell<HashMap<String, Rc<bucket::Bucket>>>,
}

impl<'env> App<'env> {
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

    /// Get reference to empty index.
    pub fn index_empty_mut(&self) -> Result<RefMut<'_, index::Index>> {
        let mut index = self.index.borrow_mut();
        index.load_path(&self.config)?;
        index.clear();

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
        config: read_config(&args)?,
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
            let index = if args.all {
                IndexType::All
            } else {
                IndexType::Active
            };
            let entries = storage::fetch_entries(&ids, index, &app)?;
            if args.json {
                display::show_json(&entries)?;
                return Ok(());
            }
            let report = &app.config.report_next;
            display::show_entries(&ids, report, &app)?;
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
            let report = app.config.report_all();
            display::show_entries(&ids, &report, &app)?;
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
        Some(Command::Config) => {
            config::print_config(&app.config)?;
        }
        Some(Command::Refresh(args)) => {
            storage::refresh_index(&app, args.force)?;
        }
        Some(Command::Init(_)) => {
            repo::init_repo(&app.config)?;
        }
        Some(Command::Check) => {
            repo::check_repo(&app.config)?;
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
            let report = &app.config.report_next();
            display::show_entries(&ids, report, &app)?;
        }
    }

    Ok(())
}

/// Read config from file and (optionally) from storage directory.
fn read_config(args: &Args) -> Result<Config> {
    let path = if let Some(config) = &args.config {
        config
    } else {
        &unwrap_ok_or!(env::var("TRACKIT_CONFIG").map(PathBuf::from), _, {
            let mut dir = dirs::config_dir().context("Unable to find config directory")?;
            dir.push(env!("CARGO_PKG_NAME"));
            dir.push("config.json5");
            dir
        })
    };

    let mut config: Config = match fs::read_to_string(path) {
        Ok(data) => json5::from_str(data.as_str())?,
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Config::default(),
            _ => bail!("Unable to read config: {}", path.to_string_lossy()),
        },
    };
    config.set_data_directory(&args.data);
    config.fallback_values();

    Ok(config)
}
