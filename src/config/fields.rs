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
    #[serde(rename = "string")]
    String,

    /// Store as integer number.
    #[serde(rename = "integer")]
    Integer,

    /// Store as numeric (f64), display as number.
    #[serde(rename = "float")]
    Float,

    /// Store as numeric (in seconds), display as duration.
    #[serde(rename = "duration")]
    Duration,

    /// Store as numeric (UNIX timestamp), display as date.
    #[serde(rename = "date")]
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
            defaults::PRIORITY => Some(FieldType::Float),
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
        out.insert(defaults::PRIORITY.into(), FieldType::Float);

        out
    }
}

impl FieldType {
    /// Format provided JSON value depending on the field type.
    pub fn format_value(&self, value: &Value) -> Option<String> {
        match self {
            Self::Float => Some(unwrap_some_or!(value.as_f64(), { return None }).to_string()),
            Self::String => value.as_str().map(|v| v.into()),
            // TODO: P3: format dates and durations and dates
            _ => None,
        }
    }

    /// Based of field type, produce json_serde Value to store in metadata map.
    pub fn parse_value(&self, value: &str) -> Result<Value> {
        // TODO: P3: parse custom field value according to field type
        match self {
            Self::Float => {
                let value = value.parse::<f64>()?;
                serde_json::Number::from_f64(value)
                    .with_context(|| format!("Unable to store value: {}", value))
                    .map(Value::Number)
            }
            Self::String => Ok(Value::String(value.into())),
            _ => Ok(Value::Null),
        }
    }
}
