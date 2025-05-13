use clap_derive::{Parser, Subcommand};

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
}

#[derive(Subcommand)]
pub enum Command {
    /// Create new entry
    #[command(visible_aliases(["a"]))]
    Add(EntryArgs),

    /// Remove specified entry
    #[command(visible_aliases(["rem", "rm", "r", "delete", "del", "d"]))]
    Remove(FilterArgs),

    /// Modify specified entry
    #[command(visible_aliases(["mod", "m"]))]
    Modify(ModArgs),

    /// List entries using set of filters
    #[command(visible_aliases(["ls", "l"]))]
    List(FilterArgs),

    /// Show info about specified entry
    #[command(visible_aliases(["inf", "i"]))]
    Info(FilterArgs),
}

impl Default for Command {
    fn default() -> Self {
        Self::List(FilterArgs::default())
    }
}

#[derive(Parser, Default)]
pub struct FilterArgs {
    /// Entry reference (UUID, number, latest)
    pub id: Option<String>,

    /// Filter by entry title content
    #[arg(long)]
    pub contains: Option<String>,

    /// Filter by max due date
    #[arg(long)]
    pub due_before: Option<String>,

    /// Filter by min due date
    #[arg(long)]
    pub due_after: Option<String>,

    /// Filter by status
    #[arg(long)]
    pub has_status: Option<String>,
}

#[derive(Parser)]
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
}

#[derive(Parser)]
pub struct ModArgs {
    #[command(flatten)]
    pub filter: FilterArgs,

    #[command(flatten)]
    pub entry: EntryArgs,
}
