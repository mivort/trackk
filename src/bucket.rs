use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::Path;
use std::rc::Rc;

use serde_derive::{Deserialize, Serialize};

use crate::issue::Issue;
use crate::{App, prelude::*};

/// Storage bucket which groups several entries in a single file.
#[derive(Serialize, Deserialize, Clone)]
pub struct Bucket {
    /// Storage bucket schema version.
    pub version: i64,

    /// List of bucket entries.
    #[serde(default)]
    pub entries: Vec<Issue>,
}

impl Bucket {
    const VERSION: i64 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            entries: Default::default(),
        }
    }

    /// Open file from the provided path and parse as bucket.
    pub fn from_path(path: impl AsRef<Path>, app: &App) -> Result<Self> {
        let path = Path::new(&app.config.data_dir)
            .join(&app.config.issues_dir)
            .join(path);
        Self::from_full_path(path)
    }

    /// Open file from the provided full path and parse as bucket.
    pub fn from_full_path(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(&path).with_context(|| {
            format!(
                "Unable to open the bucket: {}",
                path.as_ref().to_string_lossy()
            )
        })?;
        Self::from_file(&file, &path)
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
                cache.insert(path.to_owned(), bucket.clone());
                Ok(bucket)
            })()?
        });

        Ok(bucket.clone())
    }

    /// Open file from the provided path and parse as bucket. If file doesn't
    /// exist return the empty bucket.
    pub fn from_path_or_default(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(&path);
        let file = unwrap_ok_or!(file, e, {
            match e.kind() {
                ErrorKind::NotFound => return Ok(Bucket::new()),
                _ => {
                    bail!("Unable to read bucket: {}", path.as_ref().to_string_lossy())
                }
            }
        });
        Self::from_file(&file, path)
    }

    /// Insert new entry at the sorted position.
    pub fn insert(&mut self, issue: Issue) {
        if let Some(pos) = self.entries.iter().position(|e| issue.id < e.id) {
            self.entries.insert(pos, issue);
        } else {
            self.entries.push(issue);
        };
    }

    /// Fetch the reference to a bucket entry.
    pub fn find_by_id(&self, id: &str) -> Option<&Issue> {
        // TODO: bucket is sorted by id in most cases - attempt to find the issue
        // with a binary search.

        self.entries.iter().find(|&issue| issue.id.starts_with(id))
    }

    /// Fetch the mutable reference to a bucket entry.
    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut Issue> {
        self.entries
            .iter_mut()
            .find(|issue| issue.id.starts_with(id))
    }
}

impl Bucket {
    /// Read bucket file and deserialize the data.
    fn from_file(file: &File, path: impl AsRef<Path>) -> Result<Self> {
        let reader = BufReader::new(file);

        serde_json::from_reader(reader).with_context(|| {
            format!(
                "Unable to parse bucket: {}",
                path.as_ref().to_string_lossy()
            )
        })
    }
}
