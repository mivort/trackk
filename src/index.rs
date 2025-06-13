use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::Config;
use crate::issue::Issue;
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Index {
    /// List of currently active entries.
    active: Vec<String>,

    /// Index last modify time.
    _mtime: SystemTime,

    /// Path to index.
    index_path: PathBuf,
}

impl Index {
    pub fn load(config: &Config) -> Result<Self> {
        let index_path = Path::new(&config.data_dir).join("active");
        let (active, mtime) = match File::open(&index_path) {
            Ok(data) => {
                let mtime = data.metadata()?.modified()?;
                let reader = BufReader::new(data);
                let mut active = Vec::<String>::new();
                for line in reader.lines() {
                    active.push(line?);
                }
                (active, mtime)
            }
            Err(_) => (Default::default(), SystemTime::UNIX_EPOCH),
        };

        Ok(Self { active, _mtime: mtime, index_path })
    }

    /// Append entry to active/shorthand storage.
    pub fn update_status(&mut self, config: &Config, path: &str, issue: &Issue) {
        let id = &issue.id;
        let status = &issue.status;

        let entry = format!("{path}/{id}");

        if config.values.active_status.contains(status) {
            if self.active.contains(&entry) {
                return;
            }
            self.active.push(entry);
            return;
        }

        let position = self.active.iter().position(|e| e == &entry);
        if let Some(position) = position {
            self.active.remove(position);
        }
    }

    /// Write index back to storage.
    pub fn write(&self) -> Result<()> {
        let file = File::create(&self.index_path)?;
        let mut writer = BufWriter::new(file);

        for s in self.active() {
            write!(writer, "{}\n", s)?;
        }

        Ok(())
    }

    #[inline]
    pub fn active(&self) -> &[String] {
        self.active.as_slice()
    }

    /// Find shorthand for the provided ID.
    pub fn find_id(&self, id: &str) -> Option<usize> {
        self.active
            .iter()
            .position(|a| a.ends_with(id))
            .map(|v| v + 1)
    }
}
