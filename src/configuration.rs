use log::*;
use serde::Deserialize;
use std::{path::PathBuf, str};

/// Use default config if no path is provided
pub fn get_configuration(config: Option<PathBuf>) -> Result<AppConfig, anyhow::Error> {
    let mut settings = config::Config::default();

    if let Some(config) = config {
        info!("Using configuration from {:?}", config);
        settings.merge(config::File::with_name(
            config
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Failed to convert path"))?,
        ))?;
    } else {
        info!("Using default configuration");
        settings
            .merge(config::File::with_name("configuration/settings"))?
            .merge(config::File::with_name("configuration/dev_settings"))?;
    }

    settings.merge(config::Environment::with_prefix("APP"))?;

    Ok(settings.try_into()?)
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub mqtt: MqttConfig,
    pub discord: DiscordConfig,
    pub home: HomeSettings,
}

// weird serde default thing
const DEFAULT_MQTT_PORT: u16 = 1883;

const fn default_mqtt_port() -> u16 {
    DEFAULT_MQTT_PORT
}

#[derive(Deserialize, Debug, Clone)]
pub struct MqttConfig {
    pub base_route: String,
    pub broker_host: String,
    #[serde(default = "default_mqtt_port")]
    pub broker_port: u16,
    pub client_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HomeSettings {
    pub notification_discord_channel: u64,
}
