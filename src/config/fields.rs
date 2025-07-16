use serde_derive::Deserialize;

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
