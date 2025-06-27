mod app;
mod args;
mod bucket;
mod config;
mod dateexp;
mod display;
mod editor;
mod entry;
mod expansion;
mod filter;
mod functions;
mod index;
mod input;
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

use std::borrow::Cow;
use std::{env, io};

use args::{Args, Command, ImportMode};
use clap::{CommandFactory, Parser};
use clap_complete::{Generator, generate};
use config::IndexType;
use log::Level;
use prelude::*;

fn main() -> Result<()> {
    let mut config = config::read_config_chain()?;
    let exp_args = expansion::pre_process_args(&config)?;

    let args = Args::parse_from(&exp_args);
    config.override_from_args(&args);

    setup_logging(config.no_color(), args.verbose)?;
    trace!("Command expanded to: {:?}", exp_args);

    let mut app = app::App::new(config);
    app.merge_filter_args(&args.filter_args)?;

    // TODO: P2: customize default error handling

    match args.command {
        Some(Command::List(list)) => {
            app.merge_filter_args(&list.filter_args)?;

            let mut ids = filter::IdFilter::from_shorthands(args.filter_args.id, &app)?;
            ids.append_shorthands(list.filter_args.id, &app)?;

            if let Some(format) = list.format {
                display::show_format_override(&format, &ids, &app)?;
                return Ok(());
            }

            let report = if let Some(report) = list.report {
                templating::match_report(&report, &app.config)?
            } else {
                Cow::Owned(app.config.report_next())
            };

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

        Some(Command::Info(info_args)) => {
            let mut ids = filter::IdFilter::from_shorthands(info_args.ids, &app)?;
            ids.append_shorthands(args.filter_args.id, &app)?;

            let filters = filter::Filter {
                ids: &ids,
                query: &mut Default::default(),
            };
            let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

            let entries = if entries.len() > ids.index.len() {
                // TODO: P3: check for partial uuid matches
                input::pick_prompt("Show", entries, &app)?
            } else {
                entries
            };

            for entry in &entries {
                display::show_entry(entry, &app)?;
            }
        }

        Some(Command::Add(a)) => {
            let mut issue = if a.copy {
                let ids = filter::IdFilter::from_shorthands(args.filter_args.id, &app)?;
                let filters = filter::Filter {
                    ids: &ids,
                    query: &mut Default::default(),
                };
                let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;
                let entries = if entries.len() > ids.index.len() {
                    // TODO: P3: check for partial uuid matches
                    input::pick_prompt("Show", entries, &app)?
                } else {
                    entries
                };

                let (mut entry, _) = entries
                    .into_iter()
                    .next()
                    .context("Entry to copy from is not selected")?;
                entry.copy(&app);
                entry
            } else {
                entry::Entry::new(&a.entry, &app)?
            };

            if a.edit || app.config.editor_on_add.unwrap_or_default() {
                let status = editor::edit_entry(&mut issue, &app)?;
                if !status.success() {
                    return Ok(());
                }
            }
            issue.validate(&app.config)?;
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

        Some(Command::Mod(e)) => {
            let ids = filter::IdFilter::from_shorthands(args.filter_args.id, &app)?;
            if e.edit {
                // TODO: P3: apply mod args
                editor::edit_entries(&ids, &app)?;
            } else {
                storage::modify_entries(&ids, &e.entry, &app)?;
            }
        }

        Some(Command::Config) => {
            config::print_config(&app.config)?;
        }
        Some(Command::Refresh(args)) => {
            storage::refresh_index(&app, args.force)?;
        }
        Some(Command::Completions(shell)) => {
            print_completions(shell.shell, &mut Args::command());
        }
        Some(Command::Calc(exp)) => {
            let expr = exp.expr.join(" ");
            let mut output = Vec::new();
            let mut op_stack = Vec::new();
            let local = app.local_time()?;

            dateexp::parse_exp(&expr, local, &mut output)?;
            let res = dateexp::eval(&output, local, &mut op_stack, &entry::Entry::default())?;

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
            let template = match args {
                args::TemplateCommand::List => {
                    templating::print_builtin_templates();
                    return Ok(());
                }
                args::TemplateCommand::Show(template) => template,
            };

            use templates::colors::{RESET, fg};
            let (color, reset) = if app.config.no_color() { ("", "") } else { (fg(10), RESET) };

            if let Some((id, content)) = &templating::builtin_template(&template.template) {
                println!("{color}{{#- TEMPLATE: {} -#}}{reset}", id);
                print!("{}", content);
                println!("{color}{{#- END OF TEMPLATE -#}}{reset}");
            }
        }
        Some(Command::Import(import)) => match import.format {
            ImportMode::Taskwarrior => import::tw::import_from_file(import.input, &app)?,
            ImportMode::Native => {
                // TODO: P2: implement native format import
                todo!()
            }
        },
        None => {
            let ids = filter::IdFilter::from_shorthands(args.filter_args.id, &app)?;
            if ids.enabled {
                let filters = filter::Filter {
                    ids: &ids,
                    query: &mut Default::default(),
                };
                let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

                let entries = if entries.len() > ids.index.len() {
                    // TODO: P3: check for partial uuid matches
                    input::pick_prompt("Show", entries, &app)?
                } else {
                    entries
                };

                for entry in &entries {
                    display::show_entry(entry, &app)?;
                }
            } else {
                let report = &app.config.report_next();
                display::show_entries(&ids, report, &app)?;
            }
        }
    }

    Ok(())
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

/// Use of of the generators to output shell completions.
fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        env!("CARGO_BIN_NAME").to_owned(),
        &mut io::stdout(),
    );
}
