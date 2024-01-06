use std::collections::HashMap;
use std::time::Duration;

use duration_str::deserialize_duration;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Remap {
    Seq(Vec<enigo::Key>),
    Repeat(enigo::Key),
    Mouse(enigo::Button),
    Command(String),
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_duration")]
    pub key_repeat_initial_delay: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub key_repeat_sub_delay: Duration,

    #[serde(deserialize_with = "deserialize_duration")]
    pub left_stick_poll_interval: Duration,
    pub left_stick_dead_zone: f32,

    pub mouse_initial_speed: f32,
    pub mouse_max_speed: f32,
    pub mouse_ticks_to_reach_max_speed: f32,

    #[serde(deserialize_with = "deserialize_duration")]
    pub right_stick_poll_interval: Duration,
    pub right_stick_trigger_zone: f32,
    pub right_stick_dead_zone: f32,

    pub alternative_activator: Option<String>,

    pub main: HashMap<String, Remap>,
    pub alt: HashMap<String, Remap>,
}

impl Config {
    pub fn check_error(self) -> std::result::Result<Self, String> {
        if self.left_stick_dead_zone <= 0. || self.right_stick_trigger_zone <= 0. || self.right_stick_dead_zone <= 0. {
            return Err(String::from("Negative zone size"));
        }

        if self.right_stick_trigger_zone < self.right_stick_dead_zone {
            return Err(String::from("Trigger zone smaller than dead zone"));
        }

        if let Some(activator) = &self.alternative_activator {
            if self.main.contains_key(activator) {
                return Err(String::from("Activator for alternative set is remapped"));
            }
        }

        Ok(self)
    }

    pub fn get_remap(&self, input: &str, is_alternative: bool) -> Option<&Remap> {
        if !is_alternative { &self.main } else { &self.alt }.get(input)
    }
}
