use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use time::{OffsetDateTime, UtcDateTime};
use uuid::Uuid;

use crate::args::EntryArgs;
use crate::config::Config;
use crate::dateexp::{eval, parse_date};
use crate::token::Token;
use crate::{app::App, prelude::*};

/// Base entry storage with ID, title text and date properties.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Issue {
    /// Entry unique ID used for merging.
    pub id: Box<str>,

    /// Numeric shorthand.
    #[serde(skip)]
    pub sid: Option<usize>,

    /// Issue main title.
    #[serde(default)]
    pub title: String,

    /// List of issue's tags.
    #[serde(default)]
    pub tags: BTreeSet<String>,

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

    /// Formula-based urgency value.
    #[serde(skip)]
    pub urgency: f64,

    /// Custom field values.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub meta: HashMap<String, FieldValue>,
}

impl Issue {
    /// Create new entry using provided arguments.
    pub fn new(entry: &EntryArgs, app: &App) -> Result<Self> {
        let new_uuid = Uuid::new_v4().to_string();

        let mut new = Self {
            id: new_uuid.into(),
            created: app.ts,
            status: app.config.defaults.status_initial().to_string(),
            ..Default::default()
        };

        new.apply_args(entry, app)?;
        Ok(new)
    }

    /// Apply list of description arguments, merging them into a single line
    /// if list is not empty.
    pub fn apply_description(&mut self, description: &[Box<str>]) {
        if description.is_empty() {
            return;
        }
        self.title = description.join(" ");
    }

    /// Take values from provided arguments and apply to the issue. Also,
    /// update the modified timestamp.
    pub fn apply_args(&mut self, args: &EntryArgs, app: &App) -> Result<()> {
        if !args.desc.is_empty() {
            self.title = args.desc.join(" ");
        }
        for arg in &args.append {
            // TODO: P2: append to the first line
            self.title.push(' ');
            self.title.push_str(arg);
        }

        if let Some(status) = &args.status {
            self.status = status.clone();
            self.update_end(&app.config);
        }

        let due = if let Some(due) = &args.due {
            let res = parse_date(due, app, self)
                .with_context(|| format!("Unable to parse the due date: '{}'", due))?;
            Some(res)
        } else {
            self.due
        };
        let end = if let Some(end) = &args.end {
            let res = parse_date(end, app, self)
                .with_context(|| format!("Unable to parse the end date: '{}'", end))?;
            Some(res)
        } else {
            self.end
        };

        for tag in &args.tag {
            let tag = tag.replace(" ", "_");
            if let Some(tag) = tag.strip_prefix("-") {
                self.tags.remove(tag);
            } else {
                self.tags.insert(tag);
            }
        }
        if let Some(repeat) = &args.repeat {
            self.repeat = if repeat.is_empty() { None } else { Some(repeat.clone()) };
        }

        self.due = due;
        self.end = end;

        self.update_ts();
        Ok(())
    }

    /// Evaluate urgency expression and assign the result.
    pub fn calculate_urgency(
        &mut self,
        stack: &mut Vec<Token>,
        ts: OffsetDateTime,
        urgency: &[Token],
    ) -> Result<()> {
        let res = eval(urgency, ts, stack, self)?;
        if let Token::Duration(urg) = res {
            self.urgency = urg;
        }

        Ok(())
    }

    /// Update entry end timestamp if it's empty and status is not in active list.
    /// If status is updated to one of the active states, clear the timestamp.
    pub fn update_end(&mut self, config: &Config) {
        if self.end.is_some() {
            return;
        }
        if !config.values.active_status.contains(&self.status) {
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
        new.sid = Some(short);
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

        // TODO: P2: compare meta

        false
    }

    /// Check issue validity and produce error message in case if required data is missing.
    /// If possible, fix the status value.
    pub fn validate(&mut self, app: &App) -> Result<()> {
        if self.title.is_empty() {
            bail!("Entry title should not be empty");
        }

        let config = &app.config.values;
        let permit_status = &config.permit_status;

        let no_status = 'no_match: {
            let mut filter = permit_status.iter().filter(|s| s.starts_with(&self.status));
            let approx = filter.next();
            let approx = unwrap_some_or!(approx, { break 'no_match true });
            if filter.count() > 0 {
                break 'no_match true;
            }

            self.status = approx.to_string();
            false
        };
        if no_status {
            bail!(
                "Entry status should be one of: {}. Update config to allow more statuses.",
                app.config.values.permit_status.join(", ")
            );
        }

        Ok(())
    }
}

/// Build-in issue field reference.
#[derive(Debug, Clone, Copy)]
pub enum FieldRef {
    Title,
    Desc,
    Status,
    Tag,
    Created,
    Modified,
    Due,
    End,
}

impl FieldRef {
    /// Convert field reference to token value. Nulls (None) are converted to 'false'.
    /// If token is not cheaply copyable (e.g. string or set), keep the reference for now.
    pub fn as_token(&self, issue: &Issue) -> Token {
        match self {
            Self::Created => Token::Date(issue.created),
            Self::Modified => Token::Date(issue.modified),
            Self::Due => issue.due.map(Token::Date).unwrap_or(Token::Bool(false)),
            Self::End => issue.end.map(Token::Date).unwrap_or(Token::Bool(false)),
            _ => Token::Reference(*self),
        }
    }

    /// Compare referenced value to provided token.
    pub fn fuzzy_eq(&self, token: &Token, issue: &Issue) -> Result<bool> {
        // TODO: P3: support other operand types
        match (self, token) {
            (Self::Title, Token::String(rhs)) => {
                Ok(issue.title.lines().next().unwrap_or("").contains(&**rhs))
            }
            (Self::Desc, Token::String(rhs)) => Ok(issue.title.contains(&**rhs)),
            (Self::Tag, Token::String(rhs)) => Ok(issue.tags.contains(&**rhs)),
            (Self::Status, Token::String(rhs)) => Ok(issue.status == **rhs),
            _ => bail!("Unable to compare the value with field reference"),
        }
    }

    /// Calculate the length of referenced value.
    pub fn length(&self, entry: &Issue) -> f64 {
        match self {
            Self::Desc => entry.title.len() as f64,
            Self::Tag => entry.tags.len() as f64,
            Self::Status => entry.status.len() as f64,
            _ => 0.,
        }
    }

    /// Check if referenced value is 'not empty'
    pub fn has(&self, entry: &Issue) -> bool {
        match self {
            Self::Desc => !entry.title.is_empty(),
            Self::Tag => !entry.tags.is_empty(),
            Self::Status => !entry.status.is_empty(),
            Self::Due => entry.due.is_some(),
            Self::End => entry.end.is_some(),
            _ => false,
        }
    }
}

/// Custom field value. The end data representation will depend the specific field settings.
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum FieldValue {
    String(String),
    F64(f64),
    I64(i64),
}
