use std::collections::HashMap;
use std::time::Duration;

use duration_str::deserialize_duration;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Remap {
    Seq(Vec<enigo::Key>),
    Sync(Vec<enigo::Key>),
    Repeat(enigo::Key),
    Mouse(enigo::Button),
    Command(String),
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_duration", default = "Config::default_key_repeat_initial_delay")]
    pub key_repeat_initial_delay: Duration,
    #[serde(deserialize_with = "deserialize_duration", default = "Config::default_key_repeat_sub_delay")]
    pub key_repeat_sub_delay: Duration,

    #[serde(deserialize_with = "deserialize_duration", default = "Config::default_left_stick_poll_interval")]
    pub left_stick_poll_interval: Duration,
    #[serde(default = "Config::default_left_stick_dead_zone")]
    pub left_stick_dead_zone: f32,

    #[serde(default = "Config::default_mouse_initial_speed")]
    pub mouse_initial_speed: f32,
    #[serde(default = "Config::default_mouse_max_speed")]
    pub mouse_max_speed: f32,
    #[serde(default = "Config::default_mouse_ticks_to_reach_max_speed")]
    pub mouse_ticks_to_reach_max_speed: f32,

    #[serde(deserialize_with = "deserialize_duration", default = "Config::default_right_stick_poll_interval")]
    pub right_stick_poll_interval: Duration,
    #[serde(default = "Config::default_right_stick_trigger_zone")]
    pub right_stick_trigger_zone: f32,
    #[serde(default = "Config::default_right_stick_dead_zone")]
    pub right_stick_dead_zone: f32,

    pub alternative_activator: Option<String>,

    pub main: HashMap<String, Remap>,
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

    fn default_key_repeat_initial_delay() -> Duration {
        Duration::from_millis(400)
    }

    fn default_key_repeat_sub_delay() -> Duration {
        Duration::from_millis(40)
    }

    fn default_left_stick_poll_interval() -> Duration {
        Duration::from_millis(10)
    }

    fn default_left_stick_dead_zone() -> f32 {
        0.05
    }

    fn default_mouse_initial_speed() -> f32 {
        10.0
    }

    fn default_mouse_max_speed() -> f32 {
        20.0
    }

    fn default_mouse_ticks_to_reach_max_speed() -> f32 {
        30.0
    }

    fn default_right_stick_poll_interval() -> Duration {
        Duration::from_millis(50)
    }

    fn default_right_stick_trigger_zone() -> f32 {
        0.3
    }

    fn default_right_stick_dead_zone() -> f32 {
        0.1
    }
}
