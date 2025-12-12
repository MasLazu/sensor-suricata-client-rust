use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    pub file: String,
    pub server: String,
    pub port: u16,
    pub insecure: bool,
    pub interval: u64, // Duration in seconds
    pub sensor_id: String,
    pub testing_mode: bool,
    pub max_clients: Option<usize>,
    pub max_message_size: usize,
    pub verbose: usize,
}

impl ClientConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start with default values
            .set_default("file", "/var/run/suricata.sock")?
            .set_default("server", "localhost")?
            .set_default("port", 50051)?
            .set_default("insecure", true)?
            .set_default("interval", 1)?
            .set_default("sensor_id", "sensor1")?
            .set_default("testing_mode", false)?
            // max_clients default handled in main.rs
            .set_default("max_message_size", 100)?
            .set_default("verbose", 0)?
            // Add in settings from the environment (with a prefix of MES_CLIENT)
            .add_source(Environment::with_prefix("MES_CLIENT"))
            .build()?;

        s.try_deserialize()
    }
}
