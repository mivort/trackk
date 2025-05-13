use serde_derive::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub data: String,
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
    }
}
