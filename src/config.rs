use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, serde::Deserialize)]
pub enum Remap {
    #[serde(alias = "seq")]
    Seq(Vec<enigo::Key>),
    #[serde(alias = "repeat")]
    Repeat(enigo::Key),
    #[serde(alias = "mouse")]
    Mouse(enigo::Button),
    #[serde(alias = "command")]
    Command(String),
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_key_repeat_initial_delay")]
    pub key_repeat_initial_delay: Duration,

    #[serde(default = "Config::default_key_repeat_sub_delay")]
    pub key_repeat_sub_delay: Duration,

    #[serde(default = "Config::default_left_stick_poll_interval")]
    pub left_stick_poll_interval: Duration,

    #[serde(default = "Config::default_left_stick_dead_zone")]
    pub left_stick_dead_zone: f32,

    #[serde(default = "Config::default_mouse_initial_speed")]
    pub mouse_initial_speed: f32,

    #[serde(default = "Config::default_mouse_max_speed")]
    pub mouse_max_speed: f32,

    #[serde(default = "Config::default_mouse_ticks_to_reach_max_speed")]
    pub mouse_ticks_to_reach_max_speed: u32,

    #[serde(default = "Config::default_right_stick_poll_interval")]
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
        10.
    }

    fn default_mouse_max_speed() -> f32 {
        20.
    }

    fn default_mouse_ticks_to_reach_max_speed() -> u32 {
        30
    }

    fn default_right_stick_poll_interval() -> Duration {
        Duration::from_millis(50)
    }

    fn default_right_stick_trigger_zone() -> f32 {
        0.5
    }

    fn default_right_stick_dead_zone() -> f32 {
        0.1
    }

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
        let active_set = if !is_alternative { &self.main } else { &self.alt };
        active_set.get(input)
    }
}
