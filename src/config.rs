use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

use crate::args::{Args, ColorMode};
use crate::prelude::*;
use crate::templates::colors;

#[derive(Deserialize, Default)]
pub struct Config {
    /// Data directory.
    #[serde(default)]
    data_path: Box<str>,

    /// Data directory base path.
    #[serde(default)]
    data_prefix: PrefixType,

    /// Entries sub-directory.
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
    pub _fields: HashMap<String, FieldType>, // TODO: P2: perform custom fields resolution

    /// New issue default values.
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// Entry values config.
    #[serde(default)]
    pub values: ValuesConfig,

    /// Templates used for non-report command outputs.
    #[serde(default)]
    pub templates: TemplatesConfig,

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
    pub _reports: HashMap<String, ReportConfig>, // TODO: P2: handle custom reports

    /// Date formats which can be used by 'datefmt' filter.
    #[serde(default)]
    pub date_formats: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default status to assign upon creation.
    #[serde(default)]
    status_initial: Box<str>,

    /// Status which is applied when 'done' command is called.
    #[serde(default)]
    status_complete: Box<str>,

    /// Status which is applied upon entry removal.
    #[serde(default)]
    status_deleted: Box<str>,

    /// Default time string to assign as 'due'.
    #[serde(default)]
    _due: Box<str>,
}

#[derive(Deserialize, Default)]
pub struct ValuesConfig {
    /// List of statuses which are considered as 'active'.
    #[serde(default)]
    pub active_status: HashSet<String>,

    /// Only allow to assign tags from this list. Allow any tag if empty.
    #[serde(default)]
    pub _permit_tags: HashSet<String>, // TODO: P1: support list of permitted tags

    /// Only allow one of the provided statuses. Don't check status if empty.
    #[serde(default)]
    pub permit_status: Vec<Box<str>>,

    /// Urgency formula to use on entries.
    #[serde(default)]
    pub urgency_formula: Box<str>,
}

#[derive(Deserialize, Default)]
pub struct TemplatesConfig {
    /// Template name for single entry view.
    #[serde(default)]
    entry: Box<str>,

    /// Template used to display entry changes.
    #[serde(default)]
    picker: Box<str>,

    /// Template used to display entry changes.
    #[serde(default)]
    _diff: Box<str>,
}

#[derive(Deserialize, Default)]
pub struct SyncConfig {
    /// Select one of the supported sync drivers.
    pub driver: SyncDriverMode, // TODO: P1: support multiple vcs drivers
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
            self.values.permit_status = vec![
                "pending".into(),
                "started".into(),
                "blocked".into(),
                "completed".into(),
                "deleted".into(),
            ];
        }
    }

    /// Provide default editor value.
    pub fn editor(&self) -> Cow<str> {
        if !self.editor.is_empty() {
            return Cow::Borrowed(&*self.editor);
        }

        unwrap_err_or!(env::var("TRACKK_EDITOR"), editor, { return editor.into() });
        unwrap_err_or!(env::var("EDITOR"), editor, { return editor.into() });

        "nano".into()
    }

    /// Single issue view template.
    pub fn issue_view(&self) -> &str {
        if !self.templates.entry.is_empty() {
            return &self.templates.entry;
        }

        "issue"
    }

    /// Check if output should be colorized.
    pub fn no_color(&self) -> bool {
        matches!(self.color_mode, ColorMode::Never)
    }

    /// Default report format.
    pub fn report_next(&self) -> Cow<ReportConfig> {
        if !self.report_next.sections.is_empty() {
            return Cow::Borrowed(&self.report_next);
        }

        Cow::Owned(ReportConfig {
            sections: vec![
                SectionConfig {
                    title: "Backlog".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "(due or someday) >= 365d and !status:started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Upcoming".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "due >= 14d and due < 365d and !status:started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Current".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "due >= now and due < today - 14d and !status:started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Overdue".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "due < now and !status:started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Started".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "status:started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
            ],
        })
    }

    /// Report which display all entries.
    pub fn report_all(&self) -> Cow<ReportConfig> {
        if !self.report_all.sections.is_empty() {
            return Cow::Borrowed(&self.report_all);
        }

        Cow::Owned(ReportConfig {
            sections: vec![SectionConfig {
                title: "All entries".into(),
                index: IndexType::All,
                sorting: "end+ created+".into(),
                _grouping: "".into(),
                filter: "".into(),
                header: "header".into(),
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
    pub fn entries_path(&self) -> Result<PathBuf> {
        let issues_path = self.issues_path_fallback();
        let mut path = self.data_path()?;
        path.push(issues_path);
        Ok(path)
    }

    /// Data path default value.
    fn data_path_fallback(&self) -> &str {
        if self.data_path.is_empty() { env!("CARGO_PKG_NAME") } else { &self.data_path }
    }

    /// Issues path default value.
    fn issues_path_fallback(&self) -> &str {
        if self.issues_path.is_empty() { "entries" } else { &self.issues_path }
    }
}

impl ValuesConfig {
    /// Default urgency formula string.
    pub fn urgency_formula(&self) -> &str {
        if self.urgency_formula.is_empty() {
            return concat!(
                "(",
                "sig((now - (due or someday)) / 10mil) * 10",
                " + sig((now - created) / 10mil) * 0.5",
                ")",
                " * (end:false and 1 or 0)", // Only apply due/created if end is not set
                "",
                " - (end:false and 0 or (sig((now - (end or now)) / 10mil) - 0.25) * 2)",
                " + (status == started and 1 or 0)",
                " + (status == blocked and -1 or 0)",
                " + (status == deleted and -20 or 0)",
            );
        }
        &self.urgency_formula
    }
}

impl DefaultsConfig {
    /// Status which is assigned by default when entry is created.
    pub fn status_initial(&self) -> &str {
        if self.status_initial.is_empty() { "pending" } else { &self.status_initial }
    }

    /// Status which is assigned when entry is marked as done.
    pub fn status_complete(&self) -> &str {
        if self.status_complete.is_empty() { "completed" } else { &self.status_complete }
    }

    /// Status which is assigned when entry is deleted.
    pub fn status_deleted(&self) -> &str {
        if self.status_deleted.is_empty() { "deleted" } else { &self.status_deleted }
    }

    /// Default due date expression.
    pub fn _due(&self) -> &str {
        // TODO: P2: assign default due date on creation
        &self._due
    }
}

impl TemplatesConfig {
    /// Picker template with default value.
    pub fn picker(&self) -> &str {
        if self.picker.is_empty() { "picker" } else { &self.picker }
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
    /// Section header template.
    #[serde(default)]
    pub header: Box<str>,

    /// Name of tera template file used for section output.
    #[serde(default)]
    pub template: Box<str>,

    /// Index to use when report is produced.
    #[serde(default)]
    pub index: IndexType,

    /// Sorting direction.
    #[serde(default)]
    pub sorting: Box<str>,

    /// Section filter parameters.
    #[serde(default)]
    pub filter: Box<str>,

    /// Section title.
    #[serde(default)]
    pub title: Box<str>,

    /// Grouping field.
    #[serde(default)]
    _grouping: Box<str>,
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
pub enum SyncDriverMode {
    #[default]
    #[serde(rename = "git")]
    Git,

    #[serde(rename = "custom")]
    Custom,
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
    print!("{}", format_config(config)?);
    Ok(())
}

/// Produce example config with current values.
fn format_config(config: &Config) -> Result<String> {
    let color = if config.no_color() { "" } else { colors::fg(11) };
    let clear = if config.no_color() { "" } else { colors::RESET };

    Ok(format!(
        include_str!("config/example.txt"),
        pkg = env!("CARGO_PKG_NAME"),
        c = color,
        cl = clear,
        data_path = config.data_path_fallback(),
        data_path_prefix = json5::to_string(&config.data_prefix)?,
        editor = &config.editor(),
        date_formats = json5::to_string(&config.date_formats)?,
        issues_path = config.issues_path_fallback(),
        active_status = json5::to_string(&config.values.active_status)?,
        permit_status = json5::to_string(&config.values.permit_status)?,
        urgency_formula = config.values.urgency_formula(),
        status_initial = config.defaults.status_initial(),
        status_complete = config.defaults.status_complete(),
        status_deleted = config.defaults.status_deleted(),
        picker = config.templates.picker(),
        entry = config.issue_view(),
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

/// Ensure that config produced by 'config' command is valid JSON5 and follows the schema.
#[test]
fn config_doc_is_sane() {
    let mut config = Config::default();
    config.color_mode = crate::config::ColorMode::Never;

    let format = format_config(&config).unwrap();
    json5::from_str::<'_, Config>(format.as_str()).unwrap();
}
