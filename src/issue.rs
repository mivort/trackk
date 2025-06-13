use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use time::UtcDateTime;
use uuid::Uuid;

use crate::args::EntryArgs;
use crate::config::Config;
use crate::dateexp::parse_date;
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

    /// Parent issue ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub parent: Option<String>,

    /// Repeat string which is applied to task copy upon completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub repeat: Option<String>,

    /// Creation date/time.
    #[serde(default)]
    pub created: i64,

    /// Last modify timestamp.
    #[serde(default)]
    pub modified: i64,

    /// Due date/time.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub due: Option<i64>,

    /// Last status change timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub end: Option<i64>,
}

impl Issue {
    /// Create new entry using provided arguments.
    pub fn new(entry: &EntryArgs, app: &App) -> Result<Self> {
        let new_uuid = Uuid::new_v4().to_string();

        let mut new = Self {
            id: new_uuid,
            created: app.ts,
            status: app.config.defaults.status_initial.clone(),
            ..Default::default()
        };

        new.apply_args(entry, app)?;
        Ok(new)
    }

    /// Take values from provided arguments and apply to the issue. Also,
    /// update the modified timestamp.
    pub fn apply_args(&mut self, args: &EntryArgs, app: &App) -> Result<()> {
        if let Some(title) = &args.title {
            self.title = title.clone();
        }
        if let Some(status) = &args.status {
            self.status = status.clone();
            self.update_end(&app.config);
        }
        if let Some(due) = &args.due {
            self.due = Some(parse_date(due, app).context("Unable to parse the due date")?);
        }
        if let Some(end) = &args.end {
            self.end = Some(parse_date(end, app).context("Unable to parse the end date")?);
        }
        for tag in &args.tag {
            self.tags.insert(tag.clone());
        }
        for untag in &args.untag {
            self.tags.remove(untag);
        }
        if let Some(repeat) = &args.repeat {
            self.repeat = if repeat.is_empty() {
                None
            } else {
                Some(repeat.clone())
            };
        }

        self.update_ts();
        Ok(())
    }

    /// Update entry end timestamp if it's empty and status is not in active list.
    /// If status is updated to one of the active states, clear the timestamp.
    pub fn update_end(&mut self, config: &Config) {
        if self.end.is_none() && !config.values.active_status.contains(&self.status) {
            self.end = Some(UtcDateTime::now().unix_timestamp());
        } else {
            self.end = None;
        }
    }

    /// Update timestamp to the current time.
    pub fn update_ts(&mut self) {
        self.modified = UtcDateTime::now().unix_timestamp();
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
