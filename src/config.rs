use std::collections::{HashMap, HashSet};

use serde_derive::Deserialize;

use crate::args::FilterArgs;

#[derive(Deserialize, Default)]
pub struct Config {
    /// Data directory.
    pub data: String,

    /// New issue default values.
    pub defaults: DefaultsConfig,

    /// Issue values config.
    pub values: ValuesConfig,

    /// Index of available reports.
    pub _reports: HashMap<String, ReportConfig>,
}

#[derive(Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default status to assign upon creation.
    pub status: String,

    /// Default time string to assign as 'due'.
    pub _due: String,
}

#[derive(Deserialize, Default)]
pub struct ValuesConfig {
    /// List of statuses which are considered as 'active'.
    pub active_status: HashSet<String>,

    /// Only allow to assign tags from this list. Allow any tag if empty.
    pub _permit_tags: HashSet<String>,

    /// Only allow one of the provided statuses.
    pub _permit_status: HashSet<String>,
}

impl Config {
    /// Override data directory.
    pub fn set_data_directory(&mut self, data: Option<String>) {
        if let Some(data) = data {
            self.data = data;
        }
    }

    /// Fill the empty values with default ones.
    pub fn fallback_values(&mut self) {
        if self.data.is_empty() {
            self.data = "data".into();
        }

        if self.defaults.status.is_empty() {
            self.defaults.status = "pending".into();
        }

        if self.values.active_status.is_empty() {
            self.values.active_status.insert("pending".into());
            self.values.active_status.insert("wip".into());
        }
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
    _template: String,

    /// Section filter parameters.
    _filter: FilterArgs,
}
