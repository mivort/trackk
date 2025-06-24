mod app;
mod args;
mod bucket;
mod config;
mod dateexp;
mod display;
mod editor;
mod filter;
mod functions;
mod index;
mod input;
mod issue;
mod merge;
mod prelude;
mod repo;
mod sort;
mod storage;
mod templates {
    pub(crate) mod colors;
    pub(crate) mod dates;
    pub(crate) mod layout;
    pub(crate) mod strings;
}
mod templating;
mod token;
mod sync {
    pub(crate) mod driver;
    pub(crate) mod git;
}
mod import {
    pub(crate) mod tw;
}

use std::path::PathBuf;
use std::rc::Rc;
use std::{env, fs, io};

use args::{Args, Command, ImportMode};
use clap::Parser;
use config::Config;
use config::IndexType;
use log::Level;
use prelude::*;

fn main() -> Result<()> {
    let args = Args::parse();
    let config = read_config(&args)?;

    setup_logging(config.no_color(), args.verbose)?;

    if let Some(Command::Alias(alias)) = &args.command {
        // TODO: P3: replace with alias and re-parse
        warn!("Alias '{}' not found", alias.first().unwrap());
    }

    let mut app = app::App::new(config);
    app.merge_filter_args(&args.filter_args)?;

    // TODO: P2: customize default error handling

    match args.command {
        Some(Command::List(_args)) => {
            app.merge_filter_args(&args.filter_args)?;
            let ids = Default::default();
            let report = &app.config.report_next;
            display::show_entries(&ids, report, &app)?;
        }
        Some(Command::All(args)) => {
            app.merge_filter_args(&args.filter_args)?;
            let ids = Default::default();
            let report = app.config.report_all();
            display::show_entries(&ids, &report, &app)?;
        }
        Some(Command::Count) => {
            let filters = filter::Filter {
                ids: &Default::default(),
                query: &mut Default::default(),
            };
            let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;
            println!("{}", entries.len());
        }

        Some(Command::Info(args)) => {
            let filters = filter::Filter {
                ids: &filter::IdFilter::from_shorthands(args.ids, &app)?,
                query: &mut Default::default(),
            };
            let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

            let entries = if filters.ids.index.is_empty() {
                // TODO: P3: check for partial uuid matches
                input::pick_prompt("Show", entries, &app)?
            } else {
                entries
            };

            for entry in &entries {
                display::show_entry(entry, &app)?;
            }
        }
        Some(Command::Edit(args)) => {
            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            editor::edit_entries(&ids, &app)?;
        }

        Some(Command::Add(a)) => {
            let mut issue = issue::Issue::new(&a.entry, &app)?;
            issue.apply_description(&a.description);

            if !a.no_edit {
                let status = editor::edit_entry(&mut issue, &app)?;
                if !status.success() {
                    return Ok(());
                }
            }
            issue.validate()?;
            storage::add_entry(issue, &app)?;
        }
        Some(Command::Log(a)) => {
            let mut issue = issue::Issue::new(&a.entry, &app)?;
            issue.apply_description(&a.description);

            issue.status = app.config.defaults.status_complete().to_string();
            issue.update_end(&app.config);

            if !a.no_edit {
                let status = editor::edit_entry(&mut issue, &app)?;
                if !status.success() {
                    return Ok(());
                }
            }
            issue.validate()?;
            storage::add_entry(issue, &app)?;
        }

        Some(Command::_Dup) => {
            todo!()
            // TODO: P2: implement duplicate command
        }
        Some(Command::_Copy) => {
            todo!()
            // TODO: P2: implement context copy command
        }

        Some(Command::Modify(e)) => {
            let ids = filter::IdFilter::from_shorthands(e.ids, &app)?;
            storage::modify_entries(&ids, &e.entry, &app)?;
        }
        Some(Command::Complete(mut args)) => {
            // TODO: P2: only look for active/non-complete entries?

            app.filter.merge(|e| {
                e.push(token::Token::Reference(issue::FieldRef::Status));
                e.push(token::Token::String(Rc::from(
                    app.config.defaults.status_complete(),
                )));
                e.push(token::Token::NotEq);
            });

            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            if args.entry.status.is_none() {
                args.entry.status = Some(app.config.defaults.status_complete().to_string());
            }
            storage::modify_entries(&ids, &args.entry, &app)?;
        }
        Some(Command::Start(_args)) => {
            todo!(); // TODO: P3: implement start command
        }
        Some(Command::Reset(mut args)) => {
            let ids = filter::IdFilter::from_shorthands(args.ids, &app)?;
            if args.entry.status.is_none() {
                args.entry.status = Some(app.config.defaults.status_initial().to_string());
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
        Some(Command::Calc(exp)) => {
            let expr = exp.expr.join(" ");
            let mut output = Vec::new();
            let mut op_stack = Vec::new();
            let local = app.local_time()?;

            dateexp::parse_exp(&expr, local, &mut output)?;
            let res = dateexp::eval(&output, local, &mut op_stack, &issue::Issue::default())?;

            println!("{}", res.to_string()?);
        }
        Some(Command::Init(init)) => {
            repo::init_repo(&app, &init)?;
        }
        Some(Command::Check) => {
            repo::check_repo(&app.config)?;
        }
        Some(Command::Commit) => {
            repo::commit_repo(&app.config)?;
        }
        Some(Command::Sync) => {
            repo::sync_repo(&app)?;
        }
        Some(Command::Merge(merge)) => {
            merge::merge_driver(&merge)?;
        }

        Some(Command::Template(args)) => {
            use templates::colors::{RESET, fg};
            let (color, reset) = if app.config.no_color() { ("", "") } else { (fg(10), RESET) };

            if let Some((id, content)) = &templating::builtin_template(&args.template) {
                println!("{color}{{#- TEMPLATE: {} -#}}{reset}", id);
                print!("{}", content);
                println!("{color}{{#- END OF TEMPLATE -#}}{reset}");
            }
        }
        Some(Command::Import(import)) => match import.format {
            ImportMode::Taskwarrior => import::tw::import_from_file(import.input, &app)?,
            ImportMode::Native => {
                todo!()
            }
        },
        Some(Command::Alias(ids)) => {
            let filters = filter::Filter {
                ids: &filter::IdFilter::from_shorthands(ids, &app)?,
                query: &mut Default::default(),
            };
            let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

            for entry in &entries {
                display::show_entry(entry, &app)?;
            }
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
        &unwrap_ok_or!(env::var("TRACKK_CONFIG").map(PathBuf::from), _, {
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
    config.override_from_args(args);
    config.fallback_values();

    Ok(config)
}

/// Use Fern to setup colored logging output.
fn setup_logging(no_color: bool, verbose: bool) -> Result<(), log::SetLoggerError> {
    use log::LevelFilter::*;

    fern::Dispatch::new()
        .format(move |out, message, record| {
            use templates::colors::{RESET, fg};

            if no_color {
                out.finish(format_args!("{}: {}", record.level(), message))
            } else {
                let reset = RESET;
                let color = match record.level() {
                    Level::Info => fg(11),
                    Level::Warn => fg(10),
                    Level::Error => fg(9),
                    Level::Trace => fg(12),
                    _ => "",
                };
                out.finish(format_args!("{color}●{reset} {message}",))
            }
        })
        .level(if verbose { Trace } else { Info })
        .chain(std::io::stdout())
        .apply()
}
