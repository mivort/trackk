use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::config::Config;
use crate::entry::Entry;
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Index {
    /// List of currently active entries.
    active: Vec<String>,

    /// Index last modify time.
    mtime: SystemTime,

    /// Path to index.
    path: PathBuf,
}

pub const ACTIVE_INDEX: &str = "active";

impl Default for Index {
    fn default() -> Self {
        Self {
            active: Default::default(),
            path: Default::default(),
            mtime: SystemTime::UNIX_EPOCH,
        }
    }
}

impl Index {
    pub fn load(&mut self, config: &Config) -> Result<()> {
        self.load_path(config)?;

        let data = unwrap_ok_or!(File::open(&self.path), _, { return Ok(()) });
        self.mtime = data.metadata()?.modified()?;

        let reader = BufReader::new(data);
        for line in reader.lines() {
            self.active.push(line?);
        }

        trace!("Active entry index loaded");
        Ok(())
    }

    #[inline]
    pub fn clear(&mut self) {
        self.active.clear();
        self.mtime = SystemTime::UNIX_EPOCH;
    }

    pub fn loaded(&self) -> bool {
        self.mtime > SystemTime::UNIX_EPOCH
    }

    /// Initialize active entry index path.
    pub fn load_path(&mut self, config: &Config) -> Result<()> {
        self.path = config.data_path()?;
        self.path.push(ACTIVE_INDEX);
        Ok(())
    }

    /// Append or remove entry to active/shorthand storage. Return index row number
    /// if it's present in index.
    pub fn update_status(&mut self, config: &Config, path: &str, issue: &Entry) -> Option<usize> {
        let id = &issue.id;
        let status = &issue.status;

        let entry = format!("{path}/{id}");

        if config.values.active_status.contains(status) {
            if let Some(position) = self.active.iter().position(|e| *e == entry) {
                return Some(position);
            }
            self.active.push(entry);
            return Some(self.active.len() - 1);
        }

        let position = self.active.iter().position(|e| e == &entry);
        if let Some(position) = position {
            self.active.remove(position);
        }
        None
    }

    /// Sort index alphabetically. Note that it may change shorthand IDs.
    #[inline]
    pub fn sort(&mut self) {
        self.active.sort();
    }

    /// Write index back to storage.
    pub fn write(&self) -> Result<()> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);

        for s in self.active() {
            writeln!(writer, "{}", s)?;
        }

        trace!("Active entry index updated");
        Ok(())
    }

    #[inline]
    pub fn active(&self) -> &[String] {
        self.active.as_slice()
    }

    #[inline]
    pub fn mtime(&self) -> SystemTime {
        self.mtime
    }

    /// Find shorthand for the provided ID.
    pub fn find_id(&self, id: &str) -> Option<usize> {
        self.active
            .iter()
            .position(|a| a.ends_with(id))
            .map(|v| v + 1)
    }
}
