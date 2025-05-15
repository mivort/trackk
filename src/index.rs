use std::fs::{self, File};
use std::io::BufReader;

use crate::config::Config;
use crate::issue::Issue;
use crate::prelude::*;

pub struct Index<'a> {
    /// List of currently active entries.
    active: Vec<String>,

    /// Path to index.
    index_path: String,

    /// Reference to current configuration.
    config: &'a Config,
}

impl<'a> Index<'a> {
    pub fn load(config: &'a Config) -> Result<Self> {
        let index_path = format!("{}/active.json", config.data);
        let active: Vec<String> = match File::open(&index_path) {
            Ok(data) => {
                let reader = BufReader::new(data);
                serde_json::from_reader(reader)?
            }
            Err(_) => Default::default(),
        };

        Ok(Self {
            active,
            index_path,
            config,
        })
    }

    /// Append entry to active/shorthand storage.
    pub fn update_status(&mut self, path: &str, issue: &Issue) {
        let id = &issue.id;
        let status = &issue.status;

        let entry = format!("{path}/{id}");

        if self.config.values.active_status.contains(status) {
            if self.active.contains(&entry) {
                return;
            }
            self.active.push(entry);
        } else {
            let position = self.active.iter().position(|e| e == status);
            if let Some(position) = position {
                self.active.remove(position);
            }
        }
    }

    /// Write index back to storage.
    pub fn write(self) -> Result<()> {
        fs::write(&self.index_path, serde_json::to_string(&self.active)?)?;

        Ok(())
    }

    #[inline]
    pub fn active(&self) -> &[String] {
        self.active.as_slice()
    }
}
