use std::collections::BTreeMap;

use serde_derive::Deserialize;
use serde_json::Value;

use super::Config;
use crate::prelude::*;

/// Custom field type.
#[derive(Hash, PartialEq, Eq, Deserialize, Clone, Copy)]
#[cfg_attr(test, derive(Debug))]
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

/// Fields which are defined by default.
mod defaults {
    pub(super) const PROJECT: &str = "project";
    pub(super) const PRIORITY: &str = "priority";
}

impl Config {
    /// Check specified field type.
    pub fn field_type(&self, field: &str) -> Option<FieldType> {
        if let Some(field_type) = self.fields.get(field) {
            return Some(*field_type);
        }

        match field {
            defaults::PROJECT => Some(FieldType::String),
            defaults::PRIORITY => Some(FieldType::Number),
            _ => None,
        }
    }

    /// List of custom field metadata values. It gets expanded with built-in fields
    /// unless 'no default fields' option is set.
    pub fn fields_map(&self) -> BTreeMap<String, FieldType> {
        let mut out = BTreeMap::from_iter(self.fields.iter().map(|(k, &v)| (k.clone(), v)));

        if self.values.no_default_fields {
            return out;
        }

        out.insert(defaults::PROJECT.into(), FieldType::String);
        out.insert(defaults::PRIORITY.into(), FieldType::Number);

        out
    }
}

impl FieldType {
    /// Format provided JSON value depending on the field type.
    pub fn format_value(&self, value: &Value) -> Option<String> {
        match self {
            Self::Number => Some(unwrap_some_or!(value.as_f64(), { return None }).to_string()),
            _ => None,
        }
    }

    /// Based of field type, produce json_serde Value to store in metadata map.
    pub fn parse_value(&self, _value: &str) -> Result<Value> {
        // TODO: P3: parse custom field value according to field type
        match self {
            Self::Number => Ok(Value::Null),
            _ => Ok(Value::Null),
        }
    }
}
