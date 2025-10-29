pub mod colors;
pub mod fields;
pub mod query;
pub mod reports;
pub mod values;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{self, IsTerminal, stdout};
use std::path::{Path, PathBuf};
use std::{env, fs};

use serde_derive::{Deserialize, Serialize};

use crate::args::{Args, ColorMode};
use crate::templates::colors::{RESET, fg};
use crate::{expansion, prelude::*};

use colors::ColorConfig;
use fields::FieldType;
use query::QueryConfig;
use reports::ReportConfig;
use values::ValuesConfig;

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
    pub fields: HashMap<String, FieldType>, // TODO: P2: perform custom fields resolution

    /// Entry values config.
    #[serde(default)]
    pub values: ValuesConfig,

    /// Templates used for non-report command outputs.
    #[serde(default)]
    pub templates: TemplatesConfig,

    /// Options related to VCS used with the storage.
    #[serde(default)]
    pub sync: SyncConfig,

    /// Named queries which can be used in reports and for filtering.
    #[serde(default)]
    pub queries: HashMap<String, QueryConfig>, // TODO: P2: support named queries

    /// Index of available reports.
    #[serde(default)]
    pub reports: HashMap<String, ReportConfig>, // TODO: P2: handle custom reports

    /// Date formats which can be used by 'datefmt' filter.
    #[serde(default)]
    pub date_formats: HashMap<String, String>,

    /// Built-in expansion style.
    #[serde(default)]
    pub macros_style: Option<ExpansionStyle>,

    /// Aliases which provide regex-based input argument expansion rules.
    #[serde(default)]
    #[allow(unused)]
    pub macros: Vec<ExpansionConfig>,
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

    #[serde(skip)]
    config_path: PathBuf,
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct TemplatesConfig {
    /// Template name for single entry view.
    #[serde(default)]
    entry: Box<str>, // TODO: P2: remove this option in favor of 'info' report

    /// Template used to display entry changes.
    #[serde(default)]
    picker: Box<str>,

    /// Template used to display entry changes.
    #[serde(default)]
    diff: Box<str>, // TODO: P2: support custom diff display

    /// Color highlight values. When colors are disabled, those values are ignored.
    #[serde(default)]
    pub colors: HashMap<String, ColorConfig>,

    /// Color highlight values. When colors are disabled, those values are ignored.
    #[serde(default)]
    pub tags: HashMap<String, ColorConfig>, // TODO: P2: add tags coloring
    // TODO: P2: add template variables
    /// Shared templates to parse before rendering.
    #[serde(default)]
    preload: Option<Vec<Box<str>>>,
}

#[derive(Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
pub struct SyncConfig {
    /// Select one of the supported sync drivers.
    pub driver: SyncDriverMode, // TODO: P1: support multiple vcs drivers
}

impl Config {
    /// Override values from arguments and environment variables.
    ///
    /// Also, check if stdout if terminal and in case if color is 'auto',
    /// disable it.
    pub fn override_from_args(&mut self, args: &Args) -> Result<()> {
        for config in &args.config {
            let config: Config = serde_json5::from_str(config)?;
            merge_config(self, config);
        }

        if !matches!(args.color, ColorMode::Auto) {
            self.color_mode = args.color;
        } else {
            let no_color = std::env::var("NO_COLOR").unwrap_or_default();
            if no_color == "1" {
                self.color_mode = ColorMode::Never;
            }
        }

        if matches!(self.color_mode, ColorMode::Auto) && !stdout().is_terminal() {
            self.color_mode = ColorMode::Never;
        }

        Ok(())
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

    /// Current config path.
    pub fn config_path(&self) -> &PathBuf {
        &self.local.config_path
    }

    /// Provide default editor value.
    pub fn editor(&self) -> Cow<'_, str> {
        unwrap_none_or!(&self.editor, editor, { return Cow::Borrowed(editor) });

        const ENV_VAR: &str = concat!(env!("CARGO_PKG_NAME"), "_EDITOR");
        unwrap_err_or!(env::var(ENV_VAR), editor, { return editor.into() });
        unwrap_err_or!(env::var("EDITOR"), editor, { return editor.into() });

        "nano".into()
    }

    /// Check if output should be colorized.
    pub fn no_color(&self) -> bool {
        matches!(self.color_mode, ColorMode::Never)
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

impl TemplatesConfig {
    /// Picker template with default value.
    pub fn picker(&self) -> &str {
        if self.picker.is_empty() { "picker" } else { &self.picker }
    }

    /// Single entry template with default value.
    pub fn entry(&self) -> &str {
        if self.entry.is_empty() { "issue" } else { &self.entry }
    }

    /// Iterate over preload templates and apply the mapping method.
    /// If no templates were specified by users, use 'utils' template.
    pub fn preload(&self, mut load: impl FnMut(&str) -> Result<()>) -> Result<()> {
        if let Some(preload) = &self.preload {
            for p in preload.iter() {
                load(p)?;
            }
            Ok(())
        } else {
            load("utils")
        }
    }
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
    pub find: Box<str>,

    /// Replace command argument with one or more values.
    /// Capture groups can be accessed with '$1', '$2' etc.,
    /// '$$' is replaced with literal '$'.
    pub replace: Vec<String>,

    /// Command context to use the expansion in.
    /// If not specified, it will be used in root context.
    #[serde(default)]
    pub contexts: Vec<expansion::CmdContext>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
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
    let color = if config.no_color() { "" } else { fg(11) };
    let clear = if config.no_color() { "" } else { RESET };

    Ok(format!(
        include_str!("./example.txt"),
        pkg = env!("CARGO_PKG_NAME"),
        c = color,
        cl = clear,
        data_path = config.data_path_fallback(),
        data_path_prefix = serde_json5::to_string(&config.local.data_prefix)?,
        editor = &config.editor(),
        date_formats = serde_json5::to_string(&config.date_formats)?,
        active_status = serde_json5::to_string(&config.values.active_status)?,
        permit_status = serde_json5::to_string(&config.values.permit_status)?,
        urgency_formula = config.values.urgency_formula(),
        initial_status = config.values.initial_status(),
        picker = config.templates.picker(),
        entry = config.templates.entry(),
        macros_style = serde_json5::to_string(&config.macros_style.clone().unwrap_or_default())?,
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
    serde_json5::from_str::<'_, Config>(format.as_str()).unwrap();
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

    let mut local_config = read_config(&path).context("Unable to parse main config")?;
    local_config.local.config_path = path.parent().map(ToOwned::to_owned).unwrap_or_default();

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

    let data_config = read_config(&data_path).context("Unable to parse data directory config")?;
    merge_config(&mut local_config, data_config);

    Ok(local_config.default_values())
}

/// Read JSON5 config from file.
fn read_config(path: &impl AsRef<Path>) -> Result<Config> {
    Ok(match fs::read_to_string(path) {
        Ok(data) => serde_json5::from_str(data.as_str())?,
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

    /// Merge two hash maps.
    fn merge_maps<K, V>(target: &mut HashMap<K, V>, source: HashMap<K, V>)
    where
        K: Eq + std::hash::Hash,
    {
        if target.is_empty() {
            *target = source;
            return;
        }
        for (key, value) in source {
            target.entry(key).or_insert(value);
        }
    }

    merge_option(&mut target.editor, source.editor);
    merge_option(&mut target.editor_on_add, source.editor_on_add);
    merge_option(&mut target.macros_style, source.macros_style);
    merge_non_default(&mut target.color_mode, source.color_mode);

    merge_vecs(&mut target.macros, source.macros);
    merge_maps(&mut target.date_formats, source.date_formats);

    merge_maps(&mut target.queries, source.queries);
    merge_maps(&mut target.reports, source.reports);
    merge_maps(&mut target.fields, source.fields);

    merge_non_default(
        &mut target.values.urgency_formula,
        source.values.urgency_formula,
    );

    merge_non_default(&mut target.templates.entry, source.templates.entry);
    merge_non_default(&mut target.templates.picker, source.templates.picker);
    merge_non_default(&mut target.templates.diff, source.templates.diff);
    merge_non_default(&mut target.templates.preload, source.templates.preload);
    merge_maps(&mut target.templates.colors, source.templates.colors);
    merge_maps(&mut target.templates.tags, source.templates.tags);
}

/// Ensure all fields which require merging are merged.
#[test]
fn ensure_all_merged() {
    let mut a = Config::default();
    let b = Config {
        local: Default::default(),
        editor: Some("".into()),
        editor_on_add: Some(true),
        macros_style: Some(ExpansionStyle::None),
        color_mode: ColorMode::Always,
        fields: [("f1".into(), FieldType::String)].into(),
        date_formats: Default::default(),
        macros: vec![ExpansionConfig::default()],
        queries: HashMap::from([("test".into(), QueryConfig::default())]),
        reports: Default::default(),
        sync: Default::default(),
        templates: TemplatesConfig {
            entry: "test".into(),
            picker: "test".into(),
            diff: "test".into(),
            colors: [("a".into(), ColorConfig::Custom("b".into()))].into(),
            tags: [("t1".into(), ColorConfig::Custom("t2".into()))].into(),
            preload: Some(vec!["test".into()]),
        },
        values: Default::default(),
    };

    merge_config(&mut a, b.clone());
    assert_eq!(a, b);
}
