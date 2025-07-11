mod app;
mod args;
mod bucket;
mod config;
mod datecalc;
mod display;
mod editor;
mod entry;
mod expansion;
mod filter;
mod hooks;
mod import;
mod index;
mod input;
mod merge;
mod prelude;
mod repo;
mod sort;
mod storage;
mod sync;
mod templates;

use std::borrow::Cow;
use std::{env, io};

use args::{Args, Command, ImportMode};
use clap::{CommandFactory, Parser};
use clap_complete::{Generator, generate};
use config::query::IndexType;
use log::Level;
use prelude::*;

fn main() -> Result<()> {
    let mut config = config::read_config_chain()?;
    let exp_args = expansion::pre_process_args(&config)?;

    let args = Args::parse_from(&exp_args);
    config.override_from_args(&args);

    setup_logging(config.no_color(), &args)?;
    debug!("Command expanded to: {:?}", exp_args);

    let mut app = app::App::new(config);
    app.merge_filter_args(&args.filter_args)?;

    let mut ids = filter::IdFilter::from_shorthands(args.filter_args.id, &app)?;

    if args.sync {
        // TODO: P1: add sync before/after options
        repo::sync_repo(&app)?;
    }

    // TODO: P2: customize default error handling

    match args.command {
        Some(Command::List(list)) => {
            app.merge_filter_args(&list.filter_args)?;
            ids.append_shorthands(list.filter_args.id, &app)?;

            if let Some(format) = list.format {
                display::show_format_override(&format, &ids, &app)?;
                return Ok(());
            }

            let report = if let Some(report) = list.report {
                app.config.report(&report)?
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

        Some(Command::Info(info)) => {
            // TODO: P2: deprecate this command in favor of info report?
            app.merge_filter_args(&info.filter_args)?;
            ids.append_shorthands(info.filter_args.id, &app)?;

            let filters = filter::Filter {
                ids: &ids,
                query: &mut Default::default(),
            };
            let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

            let entries = if ids.check_ambiguity(&entries) {
                input::pick_prompt("Show", entries, &app)?
            } else {
                entries
            };

            for entry in &entries {
                display::show_entry(entry, &app)?;
            }
        }

        Some(Command::Add(a)) => {
            let mut entry = if a.copy {
                let filters = filter::Filter {
                    ids: &ids,
                    query: &mut Default::default(),
                };
                let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;
                let entries = if ids.check_ambiguity(&entries) {
                    input::pick_prompt("Copy from", entries, &app)?
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

            entry.apply_args(&a.entry, &app)?;

            if a.entry.edit || app.config.editor_on_add.unwrap_or_default() {
                let status = editor::edit_entry(&mut entry, &app)?;
                if !status {
                    return Ok(());
                }
            }

            entry.validate(&app)?;
            storage::add_entry(entry, &app)?;
        }

        Some(Command::_Copy) => {
            todo!()
            // TODO: P2: implement context copy command
        }

        Some(Command::Mod(e)) => {
            storage::modify_entries(&ids, &e.entry, &app)?;
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

            datecalc::parse::parse_exp(&expr, local, &mut output)?;
            let res =
                datecalc::eval::eval(&output, local, &mut op_stack, &entry::Entry::default())?;

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
            if !args.sync {
                repo::sync_repo(&app)?;
            }
        }
        Some(Command::Merge(merge)) => {
            merge::merge_driver(&merge)?;
        }

        Some(Command::Template(args)) => {
            let template = match args {
                args::TemplateCommand::List => {
                    templates::print_builtin_templates();
                    return Ok(());
                }
                args::TemplateCommand::Show(template) => template,
            };

            use templates::colors::{RESET, fg};
            let (color, reset) = if app.config.no_color() { ("", "") } else { (fg(10), RESET) };

            if let Some((id, content)) = &templates::builtin_template(&template.template) {
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
            if ids.enabled {
                let filters = filter::Filter {
                    ids: &ids,
                    query: &mut Default::default(),
                };
                let entries = storage::fetch_entries(&filters, IndexType::All, &app)?;

                let entries = if ids.check_ambiguity(&entries) {
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
fn setup_logging(no_color: bool, args: &Args) -> Result<(), log::SetLoggerError> {
    use log::LevelFilter::*;

    if args.quiet {
        return Ok(());
    }

    fern::Dispatch::new()
        .format(move |out, message, record| {
            use templates::colors::{RESET, fg};

            if no_color {
                out.finish(format_args!("{}: {}", record.level(), message))
            } else {
                let reset = RESET;
                let color = match record.level() {
                    Level::Info => fg(11),
                    Level::Debug => fg(13),
                    Level::Warn => fg(10),
                    Level::Error => fg(9),
                    Level::Trace => fg(12),
                };
                out.finish(format_args!("{color}●{reset} {message}",))
            }
        })
        .level(match args.verbose {
            0 => Info,
            1 => Debug,
            _ => Trace,
        })
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
