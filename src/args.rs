use clap_derive::{Parser, Subcommand};
use serde_derive::Deserialize;

use crate::index::Index;
use crate::prelude::*;

/// Trackit command line arguments.
#[derive(Parser)]
#[command(author, version, about = None, long_about = None)]
#[command(args_override_self = true, subcommand_precedence_over_arg = true)]
pub struct Args {
    /// List of filter rules.
    /// Supported rules: tag:, created:, modified:, due:, end:.
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
pub struct FilterArgs {
    /// Entry reference (UUID or shorthand).
    #[arg(skip)]
    pub id: Vec<String>,

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

    /// Filter by max end date.
    #[arg(long)]
    pub end_before: Option<String>,

    /// Filter by min end date.
    #[arg(long)]
    pub end_after: Option<String>,

    /// Filter by one status values.
    #[arg(long, short)]
    pub status: Vec<String>,

    /// Filter by tag.
    #[arg(long, short)]
    pub tag: Vec<String>,
}

#[derive(Parser, Default)]
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

impl FilterArgs {
    /// Iterate over IDs list provided with filter and resolve shortcuts using
    /// 'active' index.
    ///
    /// If entry ID is porvided as shortcut, but it goes outside of index range,
    /// it gets removed from the vec.
    ///
    /// Return `false` if all shorthands were outside of index range.
    pub fn resolve_shorthands(&mut self, index: &Index) -> bool {
        if self.id.is_empty() {
            return true;
        }

        let mut unresolved = false;

        self.id.retain_mut(|id| {
            let shorthand = unwrap_ok_or!(id.parse::<usize>(), _e, {
                unresolved = true;
                return true;
            });
            if shorthand > 999999 {
                unresolved = true;
                return true;
            }
            let pointer = unwrap_some_or!(index.active().get(shorthand - 1), {
                return false;
            });
            let (_, resolved) = unwrap_some_or!(pointer.rsplit_once("/"), {
                return false;
            });
            *id = resolved.to_string();

            true
        });

        if self.id.is_empty() {
            return false;
        }

        if unresolved {
            self.all = true;
        }

        true
    }
}
