use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use time::UtcDateTime;
use uuid::Uuid;

use crate::args::EntryArgs;
use crate::config::Config;
use crate::{App, prelude::*};

/// Base entry storage with ID, title text and date properties.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Issue {
    /// Entry unique ID used for merging.
    pub id: String,

    /// Numeric shorthand.
    #[serde(skip)]
    pub short: Option<usize>,

    /// Issue main title.
    #[serde(default)]
    pub title: String,

    /// List of issue's tags.
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Entry status string.
    #[serde(default)]
    pub status: String,

    /// Repeat string which is applied to task copy upon completion.
    #[serde(default)]
    pub repeat: Option<String>,

    /// Creation date/time.
    #[serde(default)]
    pub created: i64,

    /// Last modify timestamp.
    #[serde(default)]
    pub modified: i64,

    /// Due date/time.
    #[serde(default)]
    pub due: Option<i64>,

    /// Last status change timestamp.
    #[serde(default)]
    pub end: Option<i64>,
}

impl Issue {
    /// Create new entry using provided arguments.
    pub fn new(entry: &EntryArgs, app: &App) -> Self {
        let new_uuid = Uuid::new_v4().to_string();

        let ts = UtcDateTime::now().unix_timestamp();

        Self {
            id: new_uuid,
            title: entry.title.clone().unwrap_or_default(),
            status: unwrap_some_or!(&entry.status, { &app.config.defaults.status_initial }).clone(),
            created: ts,
            modified: ts,
            ..Default::default()
        }
    }

    /// Take values from provided arguments and apply to the issue. Also,
    /// update the modified timestamp.
    pub fn apply_args(&mut self, args: &EntryArgs, config: &Config) {
        if let Some(title) = &args.title {
            self.title = title.clone();
        }
        if let Some(status) = &args.status {
            self.status = status.clone();
            self.update_status(args.end.is_none(), config);
        }
        if let Some(_due) = &args.due {
            // TODO: parse and apply due date
        }
        if let Some(_end) = &args.end {
            // TODO: parse and apply end date
        }
        if let Some(repeat) = &args.repeat {
            self.repeat = if repeat.is_empty() {
                None
            } else {
                Some(repeat.clone())
            };
        }

        self.update_ts();
    }

    /// Update entry status (and end timestamp in case if 'set_end' is true and
    /// status is not in active list).
    pub fn update_status(&mut self, set_end: bool, config: &Config) {
        if set_end && !config.values.active_status.contains(&self.status) {
            self.update_end_ts();
        }
    }

    /// Update timestamp to the current time.
    pub fn update_ts(&mut self) {
        self.modified = UtcDateTime::now().unix_timestamp();
    }

    /// Update status timestamp to the current time.
    pub fn update_end_ts(&mut self) {
        self.end = Some(UtcDateTime::now().unix_timestamp());
    }

    /// Provide cloned entry with shorthand.
    pub fn with_shorthand(&self, short: usize) -> Self {
        let mut new = self.clone();
        new.short = Some(short);
        new
    }

    /// Determine if modified entry has any differences.
    pub fn differs(&self, other: &Self) -> bool {
        debug_assert_eq!(self.id, other.id, "compared entry ids should match");

        if self.title != other.title {
            return true;
        }
        if self.due != other.due {
            return true;
        }
        if self.end != other.end {
            return true;
        }
        if self.tags != other.tags {
            return true;
        }
        if self.status != other.status {
            return true;
        }
        if self.repeat != other.repeat {
            return true;
        }

        false
    }
}
