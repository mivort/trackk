use clap_derive::{Parser, Subcommand};
use serde_derive::Deserialize;

/// Trackit command line arguments.
#[derive(Parser)]
#[command(author, version, about = None, long_about = None)]
#[command(args_override_self = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to an external configuration file.
    #[arg(short, long)]
    pub config: Option<String>,

    /// Path to data storage.
    #[arg(long)]
    pub data: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create new entry
    #[command(visible_aliases(["a"]))]
    Add(AddArgs),

    /// Remove specified entry
    #[command(visible_aliases(["rem", "rm", "r", "delete", "del", "d"]))]
    Remove(FilterArgs),

    /// Modify specified entry
    #[command(visible_aliases(["mod", "m"]))]
    Modify(ModArgs),

    #[command(visible_aliases(["complete"]))]
    Done(FilterArgs),

    /// List entries using set of filters
    #[command(visible_aliases(["ls", "l"]))]
    List(FilterArgs),

    /// Show info about specified entry
    #[command(visible_aliases(["inf", "i"]))]
    Info(FilterArgs),

    /// Edit using default editor program.
    #[command(visible_aliases(["e"]))]
    Edit(FilterArgs),

    /// Merge two JSON storage buckets.
    Merge(MergeArgs),

    /// Init the storage and VCS repo.
    Init,

    /// Check data repository and VCS status.
    Check,
}

impl Default for Command {
    fn default() -> Self {
        Self::List(FilterArgs::default())
    }
}

#[derive(Parser, Deserialize, Default, Clone)]
pub struct FilterArgs {
    /// Entry reference (UUID or shorthand).
    pub id: Option<String>,

    /// List both active and inactive entries.
    #[arg(long, short)]
    pub all: bool,

    /// Filter by entry title content.
    #[arg(long)]
    pub contains: Option<String>,

    /// Filter by max due date.
    #[arg(long)]
    pub due_before: Option<String>,

    /// Filter by min due date.
    #[arg(long)]
    pub due_after: Option<String>,

    /// Filter by one status values.
    #[arg(long)]
    pub has_status: Vec<String>,

    /// Filter by tag.
    #[arg(long)]
    pub has_tag: Vec<String>,
}

#[derive(Parser, Default)]
pub struct EntryArgs {
    /// Entry title
    #[arg(short('m'), visible_aliases(["message", "msg"]), long)]
    pub title: Option<String>,

    /// Entry due reference
    #[arg(short, long)]
    pub due: Option<String>,

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
    pub no_editor: bool,

    #[command(flatten)]
    pub entry: EntryArgs,
}

#[derive(Parser, Default)]
pub struct ModArgs {
    #[command(flatten)]
    pub filter: FilterArgs,

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
