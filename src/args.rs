use std::fmt::Display;
use std::path::PathBuf;

use clap_derive::{Parser, Subcommand, ValueEnum};
use serde_derive::Deserialize;

const LOGO: &str = r#"
///  _                  _    _
/// | |_ _ __ __ _  ___| | _| | __
/// | __| '__/ _` |/ __| |/ / |/ /
/// | |_| | | (_| | (__|   <|   <
///  \__|_|  \__,_|\___|_|\_\_|\_\
///
/// Task, event and project tracker."#;

/// Trackk command line arguments.
#[derive(Parser)]
#[command(author, version, about = LOGO, long_about = None)]
#[command(args_override_self = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub filter_args: FilterArgs,

    /// Enable verbose output.
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Disable all logging messages.
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Set color mode.
    #[arg(long, global = true, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create new entry.
    Add(AddArgs),

    /// Duplicate entire entry.
    // TODO: P2: implement duplicate command
    #[command(skip)]
    _Dup,

    /// Copy task context from one entry to another.
    // TODO: P2: implement copy command
    #[command(skip)]
    _Copy,

    /// Modify specified entry
    Mod(ModArgs),

    /// List active entries using set of filters.
    List(ListArgs),

    /// Print current configuration values and comments about possible options.
    Config,

    /// Evaluate provided expression and print the result.
    Calc(CalcArgs),

    /// Show count of tasks with filter applied.
    Count,

    /// Show info about specified entry
    Info(InfoArgs),

    /// Show one of the built-in or config-defined report templates.
    #[command(subcommand)]
    Template(TemplateCommand),

    /// Import data from one of the supported formats.
    Import(ImportArgs),

    /// Merge two JSON storage buckets.
    Merge(MergeArgs),

    /// Init the storage and VCS repo.
    Init(InitArgs),

    /// Produce commit in data repository using selected VCS, but don't sync.
    Commit,

    /// Sync repository with remote repo.
    Sync,

    /// Provide shell completions.
    Completions(CompletionsArgs),

    /// Refresh the active entries index (in case if storage was edited manually).
    Refresh(RefreshArgs),

    /// Check data repository and VCS status.
    Check,
}

impl Default for Command {
    fn default() -> Self {
        Self::List(ListArgs::default())
    }
}

#[derive(Parser, Deserialize, Default, Clone)]
pub struct FilterArgs {
    /// Filter by entry ID.
    #[arg(long)]
    pub id: Vec<Box<str>>,

    /// Filter entries by title.
    #[arg(long)]
    pub title: Vec<String>,

    /// Filter entries by description.
    #[arg(long)]
    pub desc: Vec<String>,

    /// Filter entries by status.
    #[arg(long)]
    pub status: Vec<String>,

    /// Filter entries containing the tag (add '-' to tag name to exclude).
    #[arg(long)]
    pub tag: Vec<String>,

    /// Filter entries by due date.
    #[arg(long)]
    pub due: Vec<String>,

    /// Filter entries by end date.
    #[arg(long)]
    pub end: Vec<String>,

    /// Filter entries by created date.
    #[arg(long)]
    pub created: Vec<String>,

    /// Filter query to apply to the results.
    #[arg(long)]
    pub filter: Vec<String>,

    /// Sort by provided sorting rule, overriding report sorting.
    #[arg(long)]
    pub sort: Option<Box<str>>,

    /// Limit the output by the provided value.
    #[arg(long)]
    pub limit: Option<usize>,
}

/// Args to apply changes to the selected entries.
#[derive(Parser, Default)]
pub struct EntryArgs {
    /// Run editor to apply changes to the entry.
    #[arg(long)]
    pub edit: bool,

    /// Entry title message and description.
    pub description: Vec<Box<str>>,

    /// Add text to the entry title.
    #[arg(long)]
    pub append: Vec<String>,

    /// Append text after the last line of the description.
    #[arg(long)]
    pub annotate: Vec<String>,

    /// Set entry planned completion date.
    #[arg(long)]
    pub when: Option<String>,

    /// Set entry due date.
    #[arg(long, allow_hyphen_values = true)]
    pub due: Option<String>,

    /// Set entry end date.
    #[arg(long, allow_hyphen_values = true)]
    pub end: Option<String>,

    /// Set entry status.
    #[arg(long)]
    pub status: Option<String>,

    /// Set entry tag (add '-' to tag name to remove).
    #[clap(long, allow_hyphen_values = true)]
    pub tag: Vec<String>,

    /// Set task recurrence query.
    #[arg(long)]
    pub repeat: Option<String>,
}

/// Args specific for entry creation.
#[derive(Parser, Default)]
pub struct AddArgs {
    /// Copy task from the filter.
    #[arg(long)]
    pub copy: bool,

    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Parser, Default)]
pub struct InfoArgs {
    /// List of IDs to display.
    pub ids: Vec<Box<str>>,
}

#[derive(Parser)]
pub struct CompletionsArgs {
    pub shell: clap_complete::Shell,
}

#[derive(Parser, Default)]
pub struct RefreshArgs {
    /// Ignore modify time and re-parse all storage files.
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Parser, Default)]
pub struct ListArgs {
    /// Report type to display.
    pub report: Option<Box<str>>,

    /// Override output format with template string.
    #[arg(long)]
    pub format: Option<Box<str>>,

    /// Output in JSON format.
    #[arg(long)]
    pub json: bool,

    #[command(flatten)]
    pub filter_args: FilterArgs,
}

#[derive(Parser, Default)]
pub struct CalcArgs {
    /// Evaluate the expression.
    pub expr: Vec<String>,

    /// Entry ID used as expression context.
    #[arg(long)]
    pub context: Option<Box<str>>, // TODO: P2: allow to specify several contexts
}

#[derive(Parser, Default)]
pub struct ModArgs {
    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Subcommand)]
pub enum TemplateCommand {
    /// Show source code of selected template.
    Show(TemplateArgs),

    /// List available templates.
    List,
}

#[derive(Parser)]
pub struct TemplateArgs {
    /// Template name to display.
    pub template: Box<str>,
}

#[derive(Parser)]
pub struct ImportArgs {
    /// One of the supported import formats.
    #[arg(long)]
    pub format: ImportMode,

    /// Input file (read from stdin if not specified).
    pub input: PathBuf,
}

/// Merge driver arguments.
#[derive(Parser)]
pub struct MergeArgs {
    /// Ancestor of the current version.
    pub ancestor: PathBuf,

    /// Current version.
    pub ours: PathBuf,

    /// Incoming change.
    pub theirs: PathBuf,
}

#[derive(Parser)]
pub struct InitArgs {
    /// Clone repository during init.
    #[arg(long)]
    pub clone: Option<Box<str>>,

    /// User name to apply during setup.
    #[arg(long)]
    pub user: Option<Box<str>>,

    /// User e-mail to apply during setup.
    #[arg(long)]
    pub email: Option<Box<str>>,

    /// Don't setup VCS for remote sync.
    #[arg(long)]
    pub no_sync: bool,
}

#[derive(ValueEnum, Default, Clone, Copy, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Always => f.write_str("always"),
            Self::Never => f.write_str("never"),
        }
    }
}

#[derive(ValueEnum, Default, Clone, Copy)]
pub enum ImportMode {
    #[default]
    Native,
    Taskwarrior,
}
