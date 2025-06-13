use std::collections::{HashMap, HashSet};
use std::env;

use serde_derive::Deserialize;

use crate::prelude::*;

#[derive(Deserialize, Default)]
pub struct Config {
    /// Data directory.
    pub data_path: Box<str>,

    /// Issues sub-directory.
    pub issues_path: Box<str>,

    /// Editor used for entry input.
    pub editor: Box<str>,

    /// User-defined fields.
    pub fields: HashMap<String, FieldType>,

    /// New issue default values.
    pub defaults: DefaultsConfig,

    /// Issue values config.
    pub values: ValuesConfig,

    /// Options related to VCS used with the storage.
    pub sync: SyncConfig,

    /// Index of available reports.
    pub reports: HashMap<String, ReportConfig>,
}

#[derive(Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default status to assign upon creation.
    pub status_initial: Box<str>,

    /// Status which is applied when 'done' command is called.
    pub status_complete: Box<str>,

    /// Status which is applied upon entry removal.
    pub status_deleted: Box<str>,

    /// Default time string to assign as 'due'.
    pub due: Box<str>,
}

#[derive(Deserialize, Default)]
pub struct ValuesConfig {
    /// List of statuses which are considered as 'active'.
    pub active_status: HashSet<String>,

    /// Only allow to assign tags from this list. Allow any tag if empty.
    pub permit_tags: HashSet<String>,

    /// Only allow one of the provided statuses. Don't check status if empty.
    pub permit_status: HashSet<String>,
}

#[derive(Deserialize, Default)]
pub struct SyncConfig {
    /// Select one of the supported sync drivers.
    pub driver: SyncDriver,
}

impl Config {
    /// Override data directory.
    pub fn set_data_directory(&mut self, data: Option<String>) {
        if let Some(data) = data {
            self.data_path = data.into();
        }
    }

    /// Fill the empty values with default ones.
    pub fn fallback_values(&mut self) {
        if self.data_path.is_empty() {
            self.data_path = "data".into(); // TODO: change to .local/share/appname
        }

        if self.issues_path.is_empty() {
            self.issues_path = "issues".into();
        }

        if self.editor.is_empty() {
            self.editor = self.fallback_editor().into();
        }

        if self.defaults.status_initial.is_empty() {
            self.defaults.status_initial = "pending".into();
        }

        if self.defaults.status_complete.is_empty() {
            self.defaults.status_complete = "complete".into();
        }

        if self.defaults.status_deleted.is_empty() {
            self.defaults.status_deleted = "deleted".into();
        }

        if self.values.active_status.is_empty() {
            self.values.active_status = hash_set(&["pending", "started", "blocked"]);
        }

        if self.values.permit_status.is_empty() {
            self.values.permit_status =
                hash_set(&["pending", "started", "blocked", "complete", "deleted"]);
        }
    }

    /// Provide default editor value.
    fn fallback_editor(&self) -> String {
        unwrap_err_or!(env::var("TRACKIT_EDITOR"), editor, { return editor });
        unwrap_err_or!(env::var("EDITOR"), editor, { return editor });

        "nano".into()
    }
}

/// Report configuration which contains array of report sections.
#[derive(Deserialize, Default)]
pub struct ReportConfig {
    _sections: Vec<SectionConfig>,
}

/// Report section defined by filter and template.
#[derive(Deserialize, Default)]
pub struct SectionConfig {
    /// Name of tera template file used for section output.
    _template: Box<str>,

    /// Index to use when report is produced.
    _index: IndexType,

    /// Sorting direction.
    _sorting: Box<str>,

    /// Section filter parameters.
    _filter: Box<str>,
}

/// Custom field type.
#[derive(Hash, PartialEq, Eq, Deserialize)]
pub enum FieldType {
    String,
    Number,
    Date,
}

#[derive(Deserialize, Default)]
pub enum IndexType {
    #[default]
    Active,
    Recent,
    All,
}

#[derive(Deserialize, Default)]
pub enum SyncDriver {
    #[default]
    Git,
}

/// Produce a hash set from slice.
#[inline]
fn hash_set(items: &[&str]) -> HashSet<String> {
    items
        .into_iter()
        .map(|v| v.to_string())
        .collect::<HashSet<String>>()
}
