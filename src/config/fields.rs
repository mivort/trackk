use std::collections::BTreeMap;

use serde_derive::Deserialize;

use super::Config;

/// Custom field type.
#[derive(Hash, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(Debug, Clone))]
pub enum FieldType {
    /// String field value.
    String,

    /// Store as numeric, display as number.
    Number,

    /// Store as numeric, display as duration.
    Duration,

    /// Store as numeric (UNIX timestamp), display as date.
    Date,
}

impl Config {
    /// Check specified field type.
    pub fn _field_type(&self, _field: &str) -> Option<FieldType> {
        None
    }

    /// List of custom field metadata values.
    pub fn fields_map(&self) -> BTreeMap<String, FieldType> {
        BTreeMap::new()
    }
}
