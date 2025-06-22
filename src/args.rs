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
#[command(args_override_self = true, allow_external_subcommands = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to an external configuration file.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to data storage.
    #[arg(long)]
    pub data: Option<Box<str>>,

    #[command(flatten)]
    pub filter_args: FilterArgs,

    /// Enable verbose output.
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Set color mode.
    #[arg(long, global = true, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create new entry.
    #[command(visible_aliases(["a"]))]
    Add(AddArgs),

    /// Create new entry and mark it as complete.
    #[command(visible_aliases(["l"]))]
    Log(AddArgs),

    /// Duplicate entire entry.
    // TODO: P2: implement duplicate command
    #[command(skip)]
    _Dup,

    /// Copy task context from one entry to another.
    // TODO: P2: implement copy command
    #[command(skip)]
    _Copy,

    /// Remove specified entry.
    #[command(visible_aliases(["rm"]))]
    Remove(ModArgs),

    /// Modify specified entry
    #[command(visible_aliases(["mod", "m"]))]
    Modify(ModArgs),

    /// Mark specified tasks as done.
    #[command(visible_aliases(["c"]))]
    Complete(ModArgs),

    /// Mark specified tasks as started.
    Start(ModArgs),

    /// Set task status to the initial.
    #[command(visible_aliases(["stop"]))]
    Reset(ModArgs),

    /// List active entries using set of filters
    #[command(visible_aliases(["ls"]))]
    List(ListArgs),

    /// Print current configuration values and comments about possible options.
    Config,

    /// Evaluate provided expression and print the result.
    Calc(CalcArgs),

    /// List all entries using set of filters
    All,

    /// Show info about specified entry
    #[command(visible_aliases(["inf", "i"]))]
    Info(InfoArgs),

    /// Edit using default editor program.
    #[command(visible_aliases(["e"]))]
    Edit(ModArgs),

    /// Show one of the built-in or config-defined report templates.
    Template(TemplateArgs),

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

    /// Refresh the active entries index (in case if storage was edited manually).
    Refresh(RefreshArgs),

    /// Check data repository and VCS status.
    Check,

    /// If no built-in command was matched, try to match with one of the aliases.
    /// Otherwise, fallback to 'info' command.
    #[command(external_subcommand)]
    Alias(Vec<String>),
}

impl Default for Command {
    fn default() -> Self {
        Self::List(ListArgs::default())
    }
}

#[derive(Parser, Deserialize, Default, Clone)]
pub struct FilterArgs {
    /// Filter entries containing the tag (add '-' to tag name to exclude).
    #[arg(long, short, allow_hyphen_values = true)]
    pub tag: Vec<String>,

    /// Filter entries by due date.
    #[arg(long, short)]
    pub due: Vec<String>,

    /// Filter entries by end date.
    #[arg(long, short)]
    pub end: Vec<String>,

    /// Filter entries by created date.
    #[arg(long, short)]
    pub created: Vec<String>,

    /// Filter entries by title.
    #[arg(long, short = 'm')]
    pub title: Vec<String>,

    /// Filter entries by description.
    #[arg(long, short = 'M')]
    pub desc: Vec<String>,

    /// Filter by description using regular expression.
    #[arg(long, short)]
    pub glob: Vec<String>,

    /// Filter query to apply to the results.
    #[arg(long, short)]
    pub filter: Vec<String>,

    /// Sort by provided sorting rule, overriding report sorting.
    #[arg(long, short)]
    pub sort: Option<Box<str>>,
}

/// Args to apply changes to the selected entries.
#[derive(Parser, Default)]
pub struct EntryArgs {
    /// Set entry title message and description.
    #[arg(short('m'), long)]
    pub desc: Vec<String>,

    /// Append text at the end of the entry title.
    #[arg(short, long)]
    pub append: Vec<String>,

    /// Set entry due date string.
    #[arg(short, long, allow_hyphen_values = true)]
    pub due: Option<String>,

    /// Set entry end date string.
    #[arg(long, short, allow_hyphen_values = true)]
    pub end: Option<String>,

    /// Set entry status
    #[arg(short, long)]
    pub status: Option<String>,

    /// Set entry tag (add '-' to tag name to remove).
    #[clap(short, long, allow_hyphen_values = true)]
    pub tag: Vec<String>,

    /// Set task recurrence query.
    #[arg(short, long)]
    pub repeat: Option<String>,
}

/// Args specific for entry creation.
#[derive(Parser, Default)]
pub struct AddArgs {
    /// Entry title message and description.
    pub description: Vec<Box<str>>,

    /// Don't use interactive input via default editor.
    #[arg(long)]
    pub no_edit: bool,

    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Parser, Default)]
pub struct InfoArgs {
    /// List of IDs to display.
    pub ids: Vec<String>,
}

#[derive(Parser, Default)]
pub struct RefreshArgs {
    /// Ignore modify time and re-parse all storage files.
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Parser, Default)]
pub struct ListArgs {
    /// List all entries.
    #[arg(long, short)]
    pub all: bool,

    /// Output in JSON format.
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Default)]
pub struct CalcArgs {
    /// Evaluate the expression.
    pub expr: Vec<String>,

    /// Issue ID used as expression context.
    #[arg(long)]
    pub context: Option<Box<str>>,
}

#[derive(Parser, Default)]
pub struct ModArgs {
    /// List of IDs to apply changes to.
    pub ids: Vec<String>,

    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Parser)]
pub struct TemplateArgs {
    pub template: Box<str>,
}

#[derive(Parser)]
pub struct ImportArgs {
    /// One of the supported import formats.
    #[arg(long)]
    pub format: ImportMode,

    /// Input file (read from stdin if not specified).
    pub input: Option<Box<str>>,
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

#[derive(ValueEnum, Default, Clone, Copy, Deserialize)]
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
    TaskwarriorV2,
}
