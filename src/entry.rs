use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use time::{OffsetDateTime, UtcDateTime};
use uuid::Uuid;

use crate::args::EntryArgs;
use crate::config::Config;
use crate::config::fields::FieldType;
use crate::datecalc::token::Token;
use crate::datecalc::{eval::eval, parse::parse_date};
use crate::templates::dates;
use crate::{app::App, prelude::*};

/// Tuple containing entry and path to its bucket.
pub type EntryPath = (Entry, Rc<str>);

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
    pub linked: Vec<Box<str>>, // TODO: P1: support issue linking

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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub meta: BTreeMap<String, Value>,
}

impl Entry {
    /// Create new entry using provided arguments.
    pub fn new(entry: &EntryArgs, app: &App) -> Result<Self> {
        let new_uuid = Uuid::new_v4().to_string();

        let mut new = Self {
            id: new_uuid.into(),
            created: app.ts,
            status: app.config.values.initial_status().to_string(),
            ..Default::default()
        };

        new.apply_args(entry, app)?;
        Ok(new)
    }

    /// Convert entry into a copy with new UUID.
    pub fn copy(&mut self, app: &App) {
        let new_uuid = Uuid::new_v4().to_string();

        self.id = new_uuid.into();
        self.created = app.ts;
    }

    /// Take values from provided arguments and apply to the issue. Also,
    /// update the modified timestamp.
    pub fn apply_args(&mut self, args: &EntryArgs, app: &App) -> Result<()> {
        if !args.description.is_empty() {
            self.desc = args.description.join(" ");
        }

        for arg in &args.append {
            append_title(&mut self.desc, arg);
        }

        for arg in &args.annotate {
            annotate_desc(&mut self.desc, arg);
        }

        if let Some(status) = &args.status {
            self.update_status(status, app)?;
            self.update_end(&app.config);
        }

        let when = if let Some(when) = &args.when {
            if when.is_empty() {
                None
            } else {
                parse_date(when, app, self)
                    .with_context(|| format!("Unable to parse the planned date: '{}'", when))?
            }
        } else {
            self.when
        };

        let due = if let Some(due) = &args.due {
            if due.is_empty() {
                None
            } else {
                parse_date(due, app, self)
                    .with_context(|| format!("Unable to parse the due date: '{}'", due))?
            }
        } else {
            self.due
        };
        let end = if let Some(end) = &args.end {
            if end.is_empty() {
                None
            } else {
                parse_date(end, app, self)
                    .with_context(|| format!("Unable to parse the end date: '{}'", end))?
            }
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

        for meta in &args.meta {
            let (key, value) = meta.split_once("=").with_context(|| {
                format!("Meta should be provided in key=value format (got '{meta}')")
            })?;

            let field_type = app
                .config
                .field_type(key)
                .with_context(|| format!("Field '{key}' is not defined"))?;
            let parsed = field_type
                .parse_value(value, app, self)
                .with_context(|| format!("Unable to parse value for '{key}': '{value}'"))?;
            self.meta.insert(key.into(), parsed);
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
        app: &App,
    ) -> Result<()> {
        let res = eval(urgency, ts, stack, self, app)?;
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

    /// Check if entry status change should cause a repetition.
    /// If so, produce new entry with applied date shift.
    pub fn check_repeat(&self, app: &App) -> Result<Option<Self>> {
        let repeat = unwrap_some_or!(&self.repeat, { return Ok(None) });
        if repeat.is_empty() {
            return Ok(None);
        }

        let config = &app.config.values;
        if !config.repeat_status().iter().any(|s| **s == *self.status) {
            return Ok(None);
        }

        let date = parse_date(repeat, app, self)
            .with_context(|| format!("Unable to parse repeat expression: '{}'", repeat))?;
        let date = unwrap_some_or!(date, {
            info!("Task is not repeated: condition not met");
            return Ok(None);
        });

        let mut new_entry = self.clone();
        new_entry.copy(app);
        new_entry.status = app.config.values.initial_status().to_owned();
        new_entry.end = None;

        info!(
            "Task is set to repeat in {}",
            dates::longreldate(date, app.ts, None)
        );

        if let Some(due) = new_entry.due {
            new_entry.due = Some(date);
            if let Some(when) = new_entry.when {
                new_entry.when = Some(date + when - due);
            }
        } else if new_entry.when.is_some() {
            new_entry.when = Some(date);
        }

        Ok(Some(new_entry))
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
        if self.meta != other.meta {
            return true;
        }

        false
    }

    /// Check issue validity and produce error message in case if required data is missing.
    /// If possible, fix the status value.
    pub fn validate(&self, app: &App) -> Result<()> {
        if self.desc.is_empty() {
            bail!("Entry title should not be empty");
        }

        let is_active = app.config.values.active_status.contains(&self.status);

        if self.end.is_some() && is_active {
            bail!("End date should be only set for complete/deleted/inactive entries");
        }

        if let Some(repeat) = &self.repeat {
            let date = parse_date(repeat, app, self)
                .with_context(|| format!("Unable to parse repeat date: '{}'", repeat))?;
            if date.is_none() && is_active {
                warn!(
                    "Task is set to repeat, but it WON'T be repeated due to non-matching condition"
                );
            }
        }

        Ok(())
    }

    /// Return first line of the description.
    pub fn title(&self) -> &str {
        self.desc.lines().next().unwrap_or_default()
    }

    /// Produce a formatted string for the meta value.
    pub fn meta(&self, key: &str) -> Option<&Value> {
        self.meta.get(key)
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
    When,
    Due,
    End,
    Repeat,
}

impl FieldRef {
    /// Convert field reference to token value. Nulls (None) are converted to 'false'.
    /// If token is not cheaply copyable (e.g. string or set), keep the reference for now.
    pub fn as_token(&self, issue: &Entry) -> Token {
        match self {
            Self::Created => Token::Date(issue.created),
            Self::Modified => Token::Date(issue.modified),
            Self::When => issue.when.map(Token::Date).unwrap_or(Token::Bool(false)),
            Self::Due => issue.due.map(Token::Date).unwrap_or(Token::Bool(false)),
            Self::End => issue.end.map(Token::Date).unwrap_or(Token::Bool(false)),
            Self::Tag if issue.tags.is_empty() => Token::Bool(false),
            Self::Id | Self::Desc | Self::Status | Self::Title | Self::Tag | Self::Repeat => {
                Token::Reference(*self)
            }
        }
    }

    /// Try to convert custom field value token. If value is missing, convert it
    /// to 'false'.
    pub fn as_meta_token(key: Rc<str>, entry: &Entry, app: &App) -> Token {
        let value = entry.meta(&key);
        let value = unwrap_some_or!(value, { return Token::Bool(false) });

        match value {
            Value::Number(_) => {
                let field_type = app.config.field_type(&key).unwrap_or(FieldType::Float);
                match field_type {
                    FieldType::Duration | FieldType::Float | FieldType::Integer => Token::Duration(
                        value
                            .as_f64()
                            .expect("Value is expected to be convertable to f64"),
                    ),
                    FieldType::Date => Token::Date(
                        value
                            .as_i64()
                            .expect("Value is expected to be convertable to i64"),
                    ),
                    FieldType::String => Token::Bool(false),
                }
            }
            Value::String(value) => Token::String(Rc::from(value.as_str())),
            _ => Token::Bool(false),
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
            (Self::Repeat, Token::String(rhs)) => {
                Ok(issue.repeat.as_ref().map_or("", |r| r.as_str()) == rhs.as_ref())
            }
            (Self::Repeat, Token::Bool(rhs)) => Ok(issue.repeat.is_some() == *rhs),
            (Self::Tag, _) => bail!(
                "'==' got incompatible arguments (tags and {})",
                token.ttype()
            ),
            _ => bail!(
                "'==' got incompatible arguments (reference and {})",
                token.ttype()
            ),
        }
    }

    /// Perform negation. Only nullable fields can return 'true'.
    pub fn not(&self, entry: &Entry) -> bool {
        match self {
            Self::Repeat => entry.repeat.is_none(),
            _ => false,
        }
    }

    /// Check if provided value is contained in the field.
    pub fn contains(&self, token: &Token, issue: &Entry) -> Result<bool> {
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
    pub fn length(&self, entry: &Entry) -> usize {
        match self {
            Self::Desc => entry.desc.len(),
            Self::Tag => entry.tags.len(),
            Self::Status => entry.status.len(),
            _ => 0,
        }
    }
}

/// Insert space and string to entry's first line.
fn append_title(title: &mut String, append: &str) {
    let newline_pos = title.lines().next().unwrap_or_default().len();
    if !append.starts_with(' ') {
        title.insert(newline_pos, ' ');
    }
    title.insert_str(newline_pos + 1, append);
}

/// Append to entry description adding a new line.
fn annotate_desc(desc: &mut String, annotate: &str) {
    desc.push('\n');
    desc.push_str(annotate);
}
