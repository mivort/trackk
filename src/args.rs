use clap_derive::{Parser, Subcommand};
use serde_derive::Deserialize;

/// Trackit command line arguments.
#[derive(Parser)]
#[command(author, version, about = None, long_about = None)]
#[command(args_override_self = true, subcommand_precedence_over_arg = true)]
pub struct Args {
    /// List of filter rules.
    /// Supported rules:
    /// @[tag], status:, created:, modified:, due:, end:.
    /// Multiple conditions can be provided separated by comma (,) to use 'OR' matching.
    #[arg(skip)]
    pub filter: Vec<String>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to an external configuration file.
    #[arg(short, long)]
    pub config: Option<String>,

    /// Path to data storage.
    #[arg(long)]
    pub data: Option<String>,

    #[command(flatten)]
    pub filter_args: FilterArgs,
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
    #[command(visible_aliases(["rem", "rm", "r", "delete", "del", "d"]))]
    Remove,

    /// Modify specified entry
    #[command(visible_aliases(["mod", "m"]))]
    Modify(ModArgs),

    /// Mark specified tasks as done.
    #[command(visible_aliases(["complete"]))]
    Done,

    /// List entries using set of filters
    #[command(visible_aliases(["ls"]))]
    List,

    /// Show info about specified entry
    #[command(visible_aliases(["inf", "i"]))]
    Info,

    /// Edit using default editor program.
    #[command(visible_aliases(["e"]))]
    Edit,

    /// Merge two JSON storage buckets.
    Merge(MergeArgs),

    /// Init the storage and VCS repo.
    Init,

    /// Check data repository and VCS status.
    Check,
}

impl Default for Command {
    fn default() -> Self {
        Self::List
    }
}

#[derive(Parser, Deserialize, Default, Clone)]
#[command(allow_hyphen_values = true)]
pub struct FilterArgs {
    /// Exclude entries which match the provided rule.
    #[arg(skip)]
    pub exclude: Vec<String>,

    /// Filter entries containing the tag.
    #[arg(long, short = 'u')]
    pub tag: Vec<String>,

    /// Filter entries containing the tag.
    #[arg(long, short)]
    pub notag: Vec<String>,

    /// Filter entries by due date.
    #[arg(long, short)]
    pub due: Vec<String>,

    /// Filter entries by created date.
    #[arg(long)]
    pub created: Vec<String>,

    /// Filter entries by title.
    #[arg(long, short = 'm')]
    pub title: Vec<String>,

    /// Filter entries by description.
    #[arg(long)]
    pub desc: Vec<String>,

    /// Filter query to apply to the results.
    #[arg(long, short)]
    pub filter: Vec<String>,

    // TODO: deprecate separate filter flags in favor of rules
    /// List both active and inactive entries.
    #[arg(long, short)]
    pub all: bool,
}

#[derive(Parser, Default)]
#[command(allow_hyphen_values = true)]
pub struct EntryArgs {
    /// Entry title
    #[arg(short('m'), visible_aliases(["message", "msg"]), long)]
    pub title: Option<String>,

    /// Entry due date string.
    #[arg(short, long)]
    pub due: Option<String>,

    /// Entry end date string.
    #[arg(long)]
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

#[derive(Parser, Default)]
pub struct AddArgs {
    /// Don't use interactive input via default editor.
    #[arg(long)]
    pub no_edit: bool,

    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Parser, Default)]
pub struct ModArgs {
    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Parser)]
pub struct MergeArgs {
    /// Current file state in repo.
    pub ours: String,

    /// Incomfing changes.
    pub theirs: String,

    /// Merge output.
    pub output: String,
}
