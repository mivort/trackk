use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

use crate::args::{Args, ColorMode};
use crate::prelude::*;

#[derive(Deserialize, Default)]
pub struct Config {
    /// Data directory.
    #[serde(default)]
    data_path: Box<str>,

    /// Data directory base path.
    #[serde(default)]
    data_prefix: PrefixType,

    /// Issues sub-directory.
    #[serde(default)]
    issues_path: Box<str>,

    /// Editor used for entry input.
    #[serde(default)]
    editor: Box<str>,

    /// Color mode used during output.
    #[serde(default)]
    pub color_mode: ColorMode,

    /// User-defined fields.
    #[serde(default)]
    pub fields: HashMap<String, FieldType>,

    /// New issue default values.
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// Issue values config.
    #[serde(default)]
    pub values: ValuesConfig,

    /// Options related to VCS used with the storage.
    #[serde(default)]
    pub sync: SyncConfig,

    /// Default output report.
    #[serde(default)]
    pub report_next: ReportConfig,

    /// All entries report.
    #[serde(default)]
    pub report_all: ReportConfig,

    /// Index of available reports.
    #[serde(default)]
    pub custom_reports: HashMap<String, ReportConfig>,
}

#[derive(Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default status to assign upon creation.
    status_initial: Box<str>,

    /// Status which is applied when 'done' command is called.
    status_complete: Box<str>,

    /// Status which is applied upon entry removal.
    status_deleted: Box<str>,

    /// Default time string to assign as 'due'.
    due: Box<str>,
}

#[derive(Deserialize, Default)]
pub struct ValuesConfig {
    /// List of statuses which are considered as 'active'.
    pub active_status: HashSet<String>,

    /// Only allow to assign tags from this list. Allow any tag if empty.
    pub permit_tags: HashSet<String>,

    /// Only allow one of the provided statuses. Don't check status if empty.
    pub permit_status: HashSet<String>,
}

#[derive(Deserialize, Default)]
pub struct SyncConfig {
    /// Select one of the supported sync drivers.
    pub driver: SyncDriver,
}

impl Config {
    /// Override values from arguments.
    pub fn override_from_args(&mut self, args: &Args) {
        if let Some(data) = &args.data {
            self.data_path = data.clone();
            self.data_prefix = PrefixType::None;
        }

        if !matches!(args.color, ColorMode::Auto) {
            self.color_mode = args.color;
        } else {
            let no_color = std::env::var("NO_COLOR").unwrap_or_default();
            if no_color == "1" {
                self.color_mode = ColorMode::Never;
            }
        }
    }

    /// Fill the empty values with default ones.
    pub fn fallback_values(&mut self) {
        if self.values.active_status.is_empty() {
            self.values.active_status = hash_set(&["pending", "started", "blocked"]);
        }

        if self.values.permit_status.is_empty() {
            self.values.permit_status =
                hash_set(&["pending", "started", "blocked", "complete", "deleted"]);
        }
    }

    /// Provide default editor value.
    pub fn editor(&self) -> Cow<str> {
        if !self.editor.is_empty() {
            return Cow::Borrowed(&*self.editor);
        }

        unwrap_err_or!(env::var("TRACKIT_EDITOR"), editor, { return editor.into() });
        unwrap_err_or!(env::var("EDITOR"), editor, { return editor.into() });

        "nano".into()
    }

    /// Default report format.
    pub fn report_next(&self) -> Cow<ReportConfig> {
        if !self.report_next.sections.is_empty() {
            return Cow::Borrowed(&self.report_next);
        }

        Cow::Owned(ReportConfig {
            sections: vec![SectionConfig {
                index: IndexType::Active,
                _sorting: "+urgency".into(),
                _grouping: "".into(),
                _filter: "".into(),
                template: "next".into(),
            }],
        })
    }

    /// Report which display all entries.
    pub fn report_all(&self) -> Cow<ReportConfig> {
        if !self.report_all.sections.is_empty() {
            return Cow::Borrowed(&self.report_all);
        }

        Cow::Owned(ReportConfig {
            sections: vec![SectionConfig {
                index: IndexType::All,
                _sorting: "+created".into(),
                _grouping: "".into(),
                _filter: "".into(),
                template: "all".into(),
            }],
        })
    }

    /// Produce data path prefix.
    pub fn data_path_base(&self) -> Result<PathBuf> {
        let prefix = match self.data_prefix {
            PrefixType::DataDir => dirs::data_dir(),
            PrefixType::DataLocalDir => dirs::data_local_dir(),
            PrefixType::ConfigDir => dirs::config_dir(),
            PrefixType::ConfigLocalDir => dirs::config_local_dir(),
            PrefixType::HomeDir => dirs::home_dir(),
            PrefixType::None => Some(PathBuf::new()),
        };

        prefix.context("Unable to locate data prefix directory")
    }

    /// Produce full data path with default fallback.
    pub fn data_path(&self) -> Result<PathBuf> {
        let data_path = self.data_path_fallback();
        let mut path = self.data_path_base()?;
        path.push(data_path);
        Ok(path)
    }

    /// Produce full path to issues storage.
    pub fn issues_path(&self) -> Result<PathBuf> {
        let issues_path = self.issues_path_fallback();
        let mut path = self.data_path()?;
        path.push(issues_path);
        Ok(path)
    }

    /// Data path default value.
    fn data_path_fallback(&self) -> &str {
        if self.data_path.is_empty() {
            env!("CARGO_PKG_NAME")
        } else {
            &self.data_path
        }
    }

    /// Issues path default value.
    fn issues_path_fallback(&self) -> &str {
        if self.issues_path.is_empty() {
            "issues"
        } else {
            &self.issues_path
        }
    }
}

impl DefaultsConfig {
    /// Status which is assigned by default when entry is created.
    pub fn status_initial(&self) -> &str {
        if self.status_initial.is_empty() {
            return "pending";
        }
        &self.status_initial
    }

    /// Status which is assigned when entry is marked as done.
    pub fn status_complete(&self) -> &str {
        if self.status_complete.is_empty() {
            return "complete";
        }
        &self.status_complete
    }

    /// Status which is assigned when entry is deleted.
    pub fn status_deleted(&self) -> &str {
        if self.status_deleted.is_empty() {
            return "deleted";
        }
        &self.status_deleted
    }

    /// Default due date expression.
    pub fn due(&self) -> &str {
        &self.due
    }
}

/// Report configuration which contains array of report sections.
#[derive(Deserialize, Default, Clone)]
pub struct ReportConfig {
    pub sections: Vec<SectionConfig>,
}

/// Report section defined by filter and template.
#[derive(Deserialize, Default, Clone)]
pub struct SectionConfig {
    /// Name of tera template file used for section output.
    pub template: Box<str>,

    /// Index to use when report is produced.
    pub index: IndexType,

    /// Sorting direction.
    _sorting: Box<str>,

    /// Grouping field.
    _grouping: Box<str>,

    /// Section filter parameters.
    _filter: Box<str>,
}

/// Custom field type.
#[derive(Hash, PartialEq, Eq, Deserialize)]
pub enum FieldType {
    String,
    Number,
    Date,
}

#[derive(Deserialize, Default, Clone, Copy)]
pub enum IndexType {
    #[default]
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "recent")]
    Recent,
    #[serde(rename = "all")]
    All,
}

#[derive(Deserialize, Default)]
pub enum SyncDriver {
    #[default]
    #[serde(rename = "git")]
    Git,
}

#[derive(Serialize, Deserialize, Default)]
pub enum PrefixType {
    #[default]
    #[serde(rename = "data_dir")]
    DataDir,
    #[serde(rename = "data_local_dir")]
    DataLocalDir,
    #[serde(rename = "home_dir")]
    HomeDir,
    #[serde(rename = "config_dir")]
    ConfigDir,
    #[serde(rename = "config_local_dir")]
    ConfigLocalDir,
    #[serde(rename = "none")]
    None,
}

/// Print all configuration values along with documentation.
pub fn print_config(config: &Config) -> Result<()> {
    println!("{}", format_config(config)?);
    Ok(())
}

/// Produce example config with current values.
fn format_config(config: &Config) -> Result<String> {
    Ok(format!(
        concat!(
            "{{\n",
            "  // Relative or absolute path to data directory.\n",
            "  data_path: \"{data_path}\",\n",
            "\n",
            "  // Prefix which is added to the data path.\n",
            "  // Possible values: data_dir, config_dir, home_dir, none.\n",
            "  data_path_prefix: {data_path_prefix},\n",
            "\n",
            "  // Sub-path to issue files in data directory.\n",
            "  issues_path: \"{issues_path}\",\n",
            "}}",
        ),
        data_path = config.data_path_fallback(),
        data_path_prefix = json5::to_string(&config.data_prefix)?,
        issues_path = config.issues_path_fallback(),
    ))
}

/// Produce a hash set from slice.
#[inline]
fn hash_set(items: &[&str]) -> HashSet<String> {
    items
        .iter()
        .map(|v| v.to_string())
        .collect::<HashSet<String>>()
}

#[test]
fn config_doc_is_sane() {
    let config = Config::default();
    let format = format_config(&config).unwrap();
    json5::from_str::<'_, Config>(format.as_str()).unwrap();
}
