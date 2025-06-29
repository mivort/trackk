use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::{env, fs};

use serde_derive::{Deserialize, Serialize};

use crate::args::{Args, ColorMode};
use crate::templates::colors;
use crate::{expansion, prelude::*};

/// Configuration file name to look in config directories.
pub const CONFIG_FILE: &str = "config.json5";

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct Config {
    #[serde(flatten)]
    local: ConfigLocal,

    /// Editor used for entry input.
    #[serde(default)]
    editor: Option<Box<str>>,

    /// Open editor when new entry is added.
    #[serde(default)]
    pub editor_on_add: Option<bool>,

    /// Color mode used during output.
    #[serde(default)]
    pub color_mode: ColorMode,

    /// User-defined fields.
    #[serde(default)]
    pub _fields: HashMap<String, FieldType>, // TODO: P2: perform custom fields resolution

    /// Color highlight values.
    #[serde(default)]
    pub colors: HashMap<String, ColorConfig>,

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

    /// Index of available reports.
    #[serde(default)]
    pub reports: HashMap<String, ReportConfig>, // TODO: P2: handle custom reports

    /// Date formats which can be used by 'datefmt' filter.
    #[serde(default)]
    pub date_formats: HashMap<String, String>,

    /// Built-in expansion style.
    #[serde(default)]
    pub expansion_style: Option<ExpansionStyle>,

    /// Aliases which provide regex-based input argument expansion rules.
    #[serde(default)]
    #[allow(unused)]
    pub expansions: Vec<ExpansionConfig>,
}

/// Config entries which should not be taken from data storage config.
#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ConfigLocal {
    /// Data directory.
    #[serde(default)]
    data_path: Box<str>,

    /// Data directory base path.
    #[serde(default)]
    data_prefix: PrefixType,

    /// Use sub-directory in provided data path/repo.
    #[serde(default)]
    _storage_prefix: Box<str>,
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct DefaultsConfig {
    /// Default status to assign upon creation.
    #[serde(default)]
    status_initial: Box<str>,

    /// Default time string to assign as 'when'.
    #[serde(default)]
    _when: Box<str>, // TODO: P2: support default when value

    /// Default time string to assign as 'due'.
    #[serde(default)]
    _due: Box<str>, // TODO: P2: support default due value
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ValuesConfig {
    /// List of statuses which are considered as 'active'.
    #[serde(default)]
    pub active_status: HashSet<String>,

    /// When task is marked this status, check if it should be repeated.
    #[serde(default)]
    pub repeat_status: Vec<Box<str>>,

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
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
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
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct SyncConfig {
    /// Select one of the supported sync drivers.
    pub driver: SyncDriverMode, // TODO: P1: support multiple vcs drivers
}

impl Config {
    /// Override values from arguments.
    pub fn override_from_args(&mut self, args: &Args) {
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
    pub fn default_values(mut self) -> Self {
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

        self
    }

    /// Provide default editor value.
    pub fn editor(&self) -> Cow<str> {
        unwrap_none_or!(&self.editor, editor, { return Cow::Borrowed(editor) });

        const ENV_VAR: &str = concat!(env!("CARGO_PKG_NAME"), "_EDITOR");
        unwrap_err_or!(env::var(ENV_VAR), editor, { return editor.into() });
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
    pub fn report_next(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![
                SectionConfig {
                    title: "Backlog".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "(when or someday) >= 365d and (due or someday) >= 365d and status != started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Upcoming".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "((when >= 3d and when < 365d and due:false) or (due >= 3d and due < 365d)) and status != started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Current".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "((when < 3d and due:false) or (due >= now and due < 3d)) and status != started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Overdue".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "due < now and status != started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Started".into(),
                    index: IndexType::Active,
                    sorting: "urgency+".into(),
                    _grouping: "".into(),
                    filter: "status == started".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
                SectionConfig {
                    title: "Done today".into(),
                    index: IndexType::All,
                    sorting: "end+".into(),
                    _grouping: "".into(),
                    filter: "end >= today and status == completed".into(),
                    header: "header".into(),
                    template: "next".into(),
                },
            ],
        }
    }

    /// Report which displays all entries.
    pub fn report_all(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                title: "All entries".into(),
                index: IndexType::All,
                sorting: "end+ created+".into(),
                _grouping: "".into(),
                filter: "".into(),
                header: "header".into(),
                template: "all".into(),
            }],
        }
    }

    /// Report which displays all entries.
    pub fn report_recent(&self) -> ReportConfig {
        ReportConfig {
            sections: vec![SectionConfig {
                title: "All entries".into(),
                index: IndexType::All,
                sorting: "modified+".into(),
                _grouping: "".into(),
                filter: "modified > -14d".into(),
                header: "header".into(),
                template: "all".into(),
            }],
        }
    }

    /// Produce data path prefix.
    pub fn data_path_base(&self) -> Result<PathBuf> {
        let prefix = match self.local.data_prefix {
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
        let mut path = self.data_path()?;
        path.push("entries");
        Ok(path)
    }

    /// Data path default value.
    fn data_path_fallback(&self) -> &str {
        if self.local.data_path.is_empty() {
            env!("CARGO_PKG_NAME")
        } else {
            &self.local.data_path
        }
    }
}

impl ValuesConfig {
    /// Default urgency formula string.
    pub fn urgency_formula(&self) -> &str {
        if self.urgency_formula.is_empty() {
            return concat!(
                "(",
                "sig((now - (due or someday)) / 10mil) * 10",
                " + sig((now - (when or someday)) / 10mil) * 2.5 * (has(due) and 0 or 1)",
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

    /// Produce list of statuses which will trigger the repeat property to produce a copy.
    pub fn repeat_status(&self) -> Cow<Vec<Box<str>>> {
        if self.repeat_status.is_empty() {
            Cow::Owned(vec!["completed".into()])
        } else {
            Cow::Borrowed(&self.repeat_status)
        }
    }
}

impl DefaultsConfig {
    /// Status which is assigned by default when entry is created.
    pub fn status_initial(&self) -> &str {
        if self.status_initial.is_empty() { "pending" } else { &self.status_initial }
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
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct ReportConfig {
    pub sections: Vec<SectionConfig>,
}

/// Report section defined by filter and template.
#[derive(Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
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
#[cfg_attr(test, derive(Debug, Clone))]
pub enum FieldType {
    String,
    Number,
    Date,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
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
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub enum SyncDriverMode {
    #[default]
    #[serde(rename = "git")]
    Git,

    #[serde(rename = "custom")]
    Custom,
}

#[derive(Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
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

/// Expansion config entry.
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone, Default))]
pub struct ExpansionConfig {
    /// Regular expression to match on argument.
    pub expr: Box<str>,

    /// Replace command argument with one or more values.
    /// Capture groups can be accessed with '$1', '$2' etc.,
    /// '$$' is replaced with literal '$'.
    pub replace: Vec<String>,

    /// Command context to use the expansion in.
    /// If not specified, it will be used in root context.
    #[serde(default)]
    pub context: expansion::CmdContext, // TODO: P2: allow to apply rule to multiple contexts
}

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(unused)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub enum ColorConfig {
    Options(ColorOptions),
    Custom(Box<str>),
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct ColorOptions {
    pub fg: Option<u8>,
    pub bg: Option<u8>,
    _bold: bool,
    _italic: bool,
    _underscore: bool,
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub enum ExpansionStyle {
    #[default]
    #[serde(rename = "taskwarrior")]
    Taskwarrior,
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
        data_path_prefix = json5::to_string(&config.local.data_prefix)?,
        editor = &config.editor(),
        date_formats = json5::to_string(&config.date_formats)?,
        active_status = json5::to_string(&config.values.active_status)?,
        permit_status = json5::to_string(&config.values.permit_status)?,
        urgency_formula = config.values.urgency_formula(),
        status_initial = config.defaults.status_initial(),
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

/// Read config from storage directory (if there's any) and merge it with config
/// from config directory (so the config directory takes precedence).
///
/// If TRACKK_CONFIG env variable is defined, use it as the main config.
pub fn read_config_chain() -> Result<Config> {
    const ENV_CONFIG: &str = "TRACKK_CONFIG";
    const ENV_DATA: &str = "TRACKK_DATA";

    let path = &unwrap_ok_or!(env::var(ENV_CONFIG).map(PathBuf::from), _, {
        let mut dir = dirs::config_dir().context("Unable to find config directory")?;
        dir.push(env!("CARGO_PKG_NAME"));
        dir.push(CONFIG_FILE);
        dir
    });

    let mut local_config = read_config(&path)?;

    let mut data_path = 'data_path: {
        if let Ok(env_path) = std::env::var(ENV_DATA) {
            let path = PathBuf::from(&env_path);
            local_config.local.data_path = env_path.into();
            local_config.local.data_prefix = PrefixType::None;
            break 'data_path path;
        }
        local_config.data_path()?
    };
    data_path.push(CONFIG_FILE);
    if data_path == *path {
        return Ok(local_config.default_values());
    }

    let data_config = read_config(&path)?;
    merge_config(&mut local_config, data_config);

    Ok(local_config.default_values())
}

/// Read JSON5 config from file.
fn read_config(path: &impl AsRef<Path>) -> Result<Config> {
    Ok(match fs::read_to_string(path) {
        Ok(data) => json5::from_str(data.as_str())?,
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Config::default(),
            _ => bail!("Unable to read config: {}", path.as_ref().to_string_lossy()),
        },
    })
}

/// Stitch two configs together.
/// NOTE: Data path/prefix are purposedely not taken from 'source',
///       as it should be only defined in 'local' config.
fn merge_config(target: &mut Config, source: Config) {
    /// Overwrite target option if it's 'none'.
    fn merge_option<T>(target: &mut Option<T>, source: Option<T>) {
        if target.is_none() {
            *target = source
        }
    }

    /// Check if value is non-default and update otherwise.
    fn merge_non_default<T>(target: &mut T, source: T)
    where
        T: Default + Eq,
    {
        if *target == T::default() {
            *target = source
        }
    }

    /// Check vecs avoiding allocation if target is empty.
    fn merge_vecs<T>(target: &mut Vec<T>, mut source: Vec<T>) {
        if target.is_empty() {
            *target = source;
            return;
        }
        target.append(&mut source);
    }

    merge_option(&mut target.editor, source.editor);
    merge_option(&mut target.editor_on_add, source.editor_on_add);
    merge_option(&mut target.expansion_style, source.expansion_style);
    merge_non_default(&mut target.color_mode, source.color_mode);

    merge_vecs(&mut target.expansions, source.expansions);
}

/// Ensure all fields which require merging are merged.
#[test]
fn ensure_all_merged() {
    let mut a = Config::default();
    let b = Config {
        local: Default::default(),
        editor: Some("".into()),
        editor_on_add: Some(true),
        expansion_style: Some(ExpansionStyle::None),
        color_mode: ColorMode::Always,
        _fields: Default::default(),
        colors: Default::default(),
        date_formats: Default::default(),
        defaults: Default::default(),
        expansions: vec![ExpansionConfig::default()],
        reports: Default::default(),
        sync: Default::default(),
        templates: Default::default(),
        values: Default::default(),
    };

    merge_config(&mut a, b.clone());
    assert_eq!(a, b);
}
