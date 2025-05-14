use serde_derive::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    /// Data directory.
    pub data: String,

    /// New issue default values.
    pub defaults: DefaultsConfig,
}

#[derive(Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default status to assign upon creation.
    pub status: String,
}

impl Config {
    /// Override data directory.
    pub fn set_data_directory(&mut self, data: Option<String>) {
        if let Some(data) = data {
            self.data = data;
        }
    }

    /// Fill the empty values with default ones.
    pub fn fallback_values(&mut self) {
        if self.data.is_empty() {
            self.data = "data".into();
        }

        if self.defaults.status.is_empty() {
            self.defaults.status = "pending".into();
        }
    }
}
