mod app;
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
    pub(crate) mod twv2;
}

use std::path::PathBuf;
use std::{env, fs, io};

use args::{Args, Command};
use clap::Parser;
use config::Config;
use log::Level;
use prelude::*;

use self::config::IndexType;

fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = app::App::new(read_config(&args)?);

    app.filter = filter::parse_filter_args(&args, &app)?;
    if let Some(sort) = &args.filter_args.sort {
        app.sort = sort::parse_rules(sort)?;
    }

    setup_logging(app.config.no_color(), args.verbose)?;

    match args.command {
        Some(Command::List(_args)) => {
            let ids = Default::default();
            let report = &app.config.report_next;
            display::show_entries(&ids, report, &app)?;
        }
        Some(Command::All) => {
            let ids = Default::default();
            let report = app.config.report_all();
            display::show_entries(&ids, &report, &app)?;
        }

        Some(Command::Info(args)) => {
            let filters = filter::Filter {
                ids: &filter::IdFilter::from_shorthands(args.ids, &app)?,
                query: &mut Default::default(),
            };
            let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

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
            issue.validate(&app)?;
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
            issue.validate(&app)?;
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
            repo::init_repo(&app.config, init.clone.as_deref())?;
        }
        Some(Command::Check) => {
            repo::check_repo(&app.config)?;
        }
        Some(Command::Sync) => {
            repo::sync_repo(&app.config)?;
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
        Some(Command::Import(_)) => {
            // TODO: P3: implement import from taskwarrior
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
