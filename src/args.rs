use std::fmt::Display;
use std::path::PathBuf;

use clap_derive::{Parser, Subcommand, ValueEnum};
use serde_derive::Deserialize;

/// Trackit command line arguments.
#[derive(Parser)]
#[command(author, version, about = None, long_about = None)]
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

    /// Remove specified entry.
    #[command(visible_aliases(["rm", "delete", "del"]))]
    Remove(ModArgs),

    /// Modify specified entry
    #[command(visible_aliases(["mod", "m"]))]
    Modify(ModArgs),

    /// Mark specified tasks as done.
    #[command(visible_aliases(["c", "comp", "close"]))]
    Complete(ModArgs),

    /// Mark specified tasks as started.
    Start(ModArgs),

    /// Set task status to the initial.
    #[command(visible_aliases(["stop", "open"]))]
    Reset(ModArgs),

    /// List active entries using set of filters
    #[command(visible_aliases(["ls"]))]
    List(ListArgs),

    /// Print current configuration values and comments about possible options.
    Config,

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

    /// Refresh the active entries index (in case if storage was edited manually).
    Refresh(RefreshArgs),

    /// Check data repository and VCS status.
    Check,

    /// If no built-in command was matched, consider one of reports defined
    /// in the config.
    #[command(external_subcommand)]
    #[allow(unused)]
    Report(Vec<String>),
}

impl Default for Command {
    fn default() -> Self {
        Self::List(ListArgs::default())
    }
}

#[derive(Parser, Deserialize, Default, Clone)]
pub struct FilterArgs {
    /// Filter entries containing the tag.
    #[arg(long, short)]
    pub tag: Vec<String>,

    /// Filter entries excluding the tag.
    #[arg(long, short = 'u')]
    pub notag: Vec<String>,

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
    /// Entry title
    #[arg(short('m'), visible_aliases(["message", "msg"]), long)]
    pub title: Option<String>,

    /// Entry due date string.
    #[arg(short, long)]
    pub due: Option<String>,

    /// Entry end date string.
    #[arg(long, short)]
    pub end: Option<String>,

    /// Entry status
    #[arg(short, long)]
    pub status: Option<String>,

    /// Tag to apply to the record.
    #[clap(short, long)]
    pub tag: Vec<String>,

    /// Remove tag from the record.
    #[clap(short, long)]
    pub untag: Vec<String>,

    /// Repeat time specifier.
    #[arg(short, long)]
    pub repeat: Option<String>,
}

/// Args specific for entry creation.
#[derive(Parser, Default)]
pub struct AddArgs {
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
    pub format: i32,
}

#[derive(Parser)]
pub struct MergeArgs {
    /// Current file state in repo.
    pub ours: Box<str>,

    /// Incomfing changes.
    pub theirs: Box<str>,

    /// Merge output.
    pub output: Box<str>,
}

#[derive(Parser)]
pub struct InitArgs {
    /// Clone repository during init.
    pub clone: Option<Box<str>>,
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
