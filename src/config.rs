use std::collections::HashMap;
use std::time::Duration;

use duration_str::deserialize_duration;
use serde_inline_default::serde_inline_default;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Remap {
    Seq(Vec<enigo::Key>),
    Sync(Vec<enigo::Key>),
    Repeat(enigo::Key),
    Mouse(enigo::Button),
    Command(String),
}

// TODO: Remove when serde-rs/serde#368 resolves
#[serde_inline_default]
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde_inline_default(Duration::from_millis(400))]
    pub key_repeat_initial_delay: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde_inline_default(Duration::from_millis(40))]
    pub key_repeat_sub_delay: Duration,

    #[serde(deserialize_with = "deserialize_duration")]
    #[serde_inline_default(Duration::from_millis(10))]
    pub left_stick_poll_interval: Duration,
    #[serde_inline_default(0.05)]
    pub left_stick_dead_zone: f32,

    #[serde_inline_default(10.0)]
    pub mouse_initial_speed: f32,
    #[serde_inline_default(20.0)]
    pub mouse_max_speed: f32,
    #[serde_inline_default(30.0)]
    pub mouse_ticks_to_reach_max_speed: f32,

    #[serde(deserialize_with = "deserialize_duration")]
    #[serde_inline_default(Duration::from_millis(50))]
    pub right_stick_poll_interval: Duration,
    #[serde_inline_default(0.3)]
    pub right_stick_trigger_zone: f32,
    #[serde_inline_default(0.1)]
    pub right_stick_dead_zone: f32,

    #[serde(default)]
    pub alternative_activator: Option<String>,

    #[serde(default)]
    pub main: HashMap<String, Remap>,
    #[serde(default)]
    pub alt: HashMap<String, Remap>,
}

impl Config {
    pub fn try_new() -> std::result::Result<Self, &'static str> {
        let config_path = std::env::current_exe().unwrap().with_extension("toml");
        let config_str = std::fs::read_to_string(&config_path).unwrap_or_default();
        let config_obj = toml::from_str::<Config>(&config_str).expect("Unable to parse the config file");

        if config_obj.left_stick_dead_zone <= 0. || config_obj.right_stick_trigger_zone <= 0. || config_obj.right_stick_dead_zone <= 0. {
            return Err("Negative zone size");
        }

        if config_obj.right_stick_trigger_zone < config_obj.right_stick_dead_zone {
            return Err("Trigger zone smaller than dead zone");
        }

        if let Some(activator) = &config_obj.alternative_activator
            && config_obj.main.contains_key(activator)
        {
            return Err("Activator for alternative set is remapped");
        }

        Ok(config_obj)
    }

    pub fn get_remap(&self, input: &str, is_alternative: bool) -> Option<&Remap> {
        if !is_alternative { &self.main } else { &self.alt }.get(input)
    }
}
