use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::rc::Rc;

use serde_derive::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::{app::App, prelude::*};

/// Storage bucket which groups several entries in a single file.
#[derive(Serialize, Deserialize, Clone)]
pub struct Bucket {
    /// Storage bucket schema version.
    pub version: i64,

    /// List of bucket entries.
    #[serde(default)]
    pub entries: Vec<Entry>,
}

impl Bucket {
    /// Bucket entry schema version.
    pub const VERSION: i64 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            entries: Default::default(),
        }
    }

    /// Open file from the provided path and parse as bucket.
    pub fn from_path(path: impl AsRef<Path>, app: &App) -> Result<Self> {
        let mut full_path = app.config.entries_path()?;
        full_path.push(path);
        Self::from_full_path(full_path)
    }

    /// Open file from the provided full path and parse as bucket.
    pub fn from_full_path(path: impl AsRef<Path>) -> Result<Self> {
        let data = fs::read_to_string(&path).with_context(|| {
            format!(
                "Unable to open the bucket: {}",
                path.as_ref().to_string_lossy()
            )
        })?;
        Self::from_data(&data, &path)
    }

    /// Check cache and read from file system if not yet cached.
    pub fn from_cache(
        path: &str,
        cache: &mut HashMap<String, Rc<Self>>,
        app: &App,
    ) -> Result<Rc<Self>> {
        let bucket = unwrap_some_or!(cache.get(path), {
            &(|| -> Result<_> {
                let bucket = Rc::new(Bucket::from_path(path, app)?);

                trace!("Reading bucket to cache: {path}");

                cache.insert(path.to_owned(), bucket.clone());
                Ok(bucket)
            })()?
        });

        Ok(bucket.clone())
    }

    /// Open file from the provided path and parse as bucket. If file doesn't
    /// exist return the empty bucket.
    pub fn from_full_path_or_default(path: impl AsRef<Path>) -> Result<Self> {
        let data = fs::read_to_string(&path);
        let data = unwrap_ok_or!(data, e, {
            match e.kind() {
                ErrorKind::NotFound => return Ok(Bucket::new()),
                _ => {
                    bail!("Unable to read bucket: {}", path.as_ref().to_string_lossy())
                }
            }
        });
        Self::from_data(&data, path)
    }

    /// Insert new entry at the sorted position.
    pub fn insert(&mut self, insert: Entry) -> Option<usize> {
        if let Some(pos) = self.entries.iter().position(|e| insert.id <= e.id) {
            if self.entries[pos].id == insert.id {
                return Some(pos);
            }
            self.entries.insert(pos, insert);
        } else {
            self.entries.push(insert);
        };
        None
    }

    /// Fetch the reference to a bucket entry.
    pub fn find_by_id(&self, id: &str) -> Option<&Entry> {
        // TODO: P1: bucket is sorted by id in most cases - attempt to find the issue
        // with a binary search.

        self.entries.iter().find(|&issue| issue.id.starts_with(id))
    }

    /// Fetch the mutable reference to a bucket entry.
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut Entry> {
        self.entries
            .iter_mut()
            .find(|issue| issue.id.starts_with(id))
    }
}

impl Bucket {
    /// Read bucket data and deserialize.
    fn from_data(data: &str, path: impl AsRef<Path>) -> Result<Self> {
        let bucket: Self = serde_json::from_str(data).with_context(|| {
            format!(
                "Unable to parse bucket: {}",
                path.as_ref().to_string_lossy()
            )
        })?;

        if bucket.version > Self::VERSION {
            warn!("Bucket is using a newer version ({})", bucket.version);
        }

        // TODO: P1: add fallback in case if parsing has failed

        Ok(bucket)
    }
}
