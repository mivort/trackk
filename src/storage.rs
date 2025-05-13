use crate::args::{EntryArgs, ModArgs};

/// Access storage bucket if it exists and add new entry to it.
pub fn add_entry(_entry: &EntryArgs) {
    create_bucket();
}

/// Find entry using the filter and update its properties.
pub fn modify_entry(_entry: &ModArgs) {}

/// Create the storage bucket using the current date.
fn create_bucket() {}
