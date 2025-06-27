use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
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
pub struct Entry {
    /// Entry unique ID used for merging.
    pub id: Box<str>,

    /// Numeric shorthand.
    #[serde(skip)]
    pub sid: Option<usize>,

    /// Entry main title.
    #[serde(default)]
    pub desc: String,

    /// List of issue's tags.
    #[serde(default)]
    pub tags: BTreeSet<String>,

    /// Entry status string.
    #[serde(default)]
    pub status: String,

    /// IDs of linked issues.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub linked: Vec<Box<str>>,

    /// Repeat string which is applied to task copy upon completion.
    /// Will be applied to both 'due' and 'when' dates if those are set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub repeat: Option<String>,

    /// Creation date/time.
    #[serde(default)]
    pub created: i64,

    /// Last modify timestamp.
    #[serde(default)]
    pub modified: i64,

    /// Date of planned completion.
    #[serde(default)]
    pub when: Option<i64>,

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
    pub meta: HashMap<String, Value>,
}

impl Entry {
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
        self.desc = description.join(" ");
    }

    /// Take values from provided arguments and apply to the issue. Also,
    /// update the modified timestamp.
    pub fn apply_args(&mut self, args: &EntryArgs, app: &App) -> Result<()> {
        if !args.desc.is_empty() {
            self.desc = args.desc.join(" ");
        }
        for arg in &args.append {
            // TODO: P2: append to the first line
            self.desc.push(' ');
            self.desc.push_str(arg);
        }

        if let Some(status) = &args.status {
            self.update_status(status, app)?;
            self.update_end(&app.config);
        }

        let when = if let Some(when) = &args.when {
            let res = parse_date(when, app, self)
                .with_context(|| format!("Unable to parse the planned date: '{}'", when))?;
            Some(res)
        } else {
            self.when
        };

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

        self.when = when;
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

    /// Update status from user input matching one of the permitted values.
    pub fn update_status(&mut self, status: &str, app: &App) -> Result<()> {
        let permit_status = &app.config.values.permit_status;
        let no_status = 'no_match: {
            let mut filter = permit_status.iter().filter(|s| s.starts_with(status));
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

    /// Update entry end timestamp if it's empty and status is not in active list.
    /// If status is updated to one of the active states, clear the timestamp.
    pub fn update_end(&mut self, config: &Config) {
        if config.values.active_status.contains(&self.status) {
            self.end = None;
        } else {
            if self.end.is_some() {
                return;
            }
            self.end = Some(UtcDateTime::now().unix_timestamp());
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

        if self.desc != other.desc {
            return true;
        }
        if self.when != other.when || self.due != other.due {
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
    pub fn validate(&self, config: &Config) -> Result<()> {
        if self.desc.is_empty() {
            bail!("Entry title should not be empty");
        }

        if self.end.is_some() && config.values.active_status.contains(&self.status) {
            bail!("End date should be only set for complete/deleted/inactive entries");
        }

        // TODO: P2: in case if repeat is set, check if at least 'due' or 'when' is not empty.

        Ok(())
    }
}

/// Build-in issue field reference.
#[derive(Debug, Clone, Copy)]
pub enum FieldRef {
    Id,
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
    pub fn as_token(&self, issue: &Entry) -> Token {
        match self {
            Self::Created => Token::Date(issue.created),
            Self::Modified => Token::Date(issue.modified),
            Self::Due => issue.due.map(Token::Date).unwrap_or(Token::Bool(false)),
            Self::End => issue.end.map(Token::Date).unwrap_or(Token::Bool(false)),
            _ => Token::Reference(*self),
        }
    }

    /// Perform strict comparison.
    pub fn eq(&self, token: &Token, issue: &Entry) -> Result<bool> {
        match (self, token) {
            (Self::Title, Token::String(rhs)) => {
                Ok(issue.desc.lines().next().unwrap_or("") == rhs.as_ref())
            }
            (Self::Desc, Token::String(rhs)) => Ok(issue.desc == rhs.as_ref()),
            (Self::Status, Token::String(rhs)) => Ok(issue.status == rhs.as_ref()),
            (Self::Tag, _) => bail!(
                "':' got incompatible arguments (tags and {})",
                token.ttype()
            ),
            _ => bail!(
                "':' got incompatible arguments (reference and {})",
                token.ttype()
            ),
        }
    }

    /// Compare referenced value to provided token.
    pub fn fuzzy_eq(&self, token: &Token, issue: &Entry) -> Result<bool> {
        use Token::*;

        match (self, token) {
            (Self::Id, Token::String(rhs)) => Ok(issue.id.starts_with(&**rhs)),
            (Self::Title, Token::String(rhs)) => {
                Ok(issue.desc.lines().next().unwrap_or("").contains(&**rhs))
            }
            (Self::Title, Token::Regex(regex)) => {
                Ok(regex.is_match(issue.desc.lines().next().unwrap_or("")))
            }

            (Self::Desc, String(rhs)) => Ok(issue.desc.contains(&**rhs)),
            (Self::Desc, Regex(regex)) => Ok(regex.is_match(&issue.desc)),

            (Self::Tag, String(rhs)) => Ok(issue.tags.contains(&**rhs)),
            (Self::Tag, Regex(regex)) => Ok(issue.tags.iter().any(|t| regex.is_match(t))),

            (Self::Status, String(rhs)) => Ok(issue.status.starts_with(&**rhs)),
            (Self::Status, Regex(regex)) => Ok(regex.is_match(&issue.status)),

            // TODO: P2: support matching with custom fields
            _ => bail!(
                "':' got incompatible arguments (reference and {})",
                token.ttype()
            ),
        }
    }

    /// Calculate the length of referenced value.
    pub fn length(&self, entry: &Entry) -> f64 {
        match self {
            Self::Desc => entry.desc.len() as f64,
            Self::Tag => entry.tags.len() as f64,
            Self::Status => entry.status.len() as f64,
            _ => 0.,
        }
    }

    /// Check if referenced value is 'not empty'
    pub fn has(&self, entry: &Entry) -> bool {
        match self {
            Self::Desc => !entry.desc.is_empty(),
            Self::Tag => !entry.tags.is_empty(),
            Self::Status => !entry.status.is_empty(),
            Self::Due => entry.due.is_some(),
            Self::End => entry.end.is_some(),
            _ => false,
        }
    }
}
