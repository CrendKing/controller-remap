use std::time::Duration;

#[allow(non_camel_case_types)]
#[derive(Debug, serde::Deserialize)]
pub enum Remap {
    keys(Vec<String>),
    repeat(String),
}

#[derive(Debug, serde::Deserialize)]
pub struct Set {
    a: Option<Remap>,
    b: Option<Remap>,
    x: Option<Remap>,
    y: Option<Remap>,
    select: Option<Remap>,
    start: Option<Remap>,
    right_stick_up: Option<Remap>,
    right_stick_down: Option<Remap>,
    right_stick_left: Option<Remap>,
    right_stick_right: Option<Remap>,
    left_stick_button: Option<Remap>,
    right_stick_button: Option<Remap>,
    left_bumper: Option<Remap>,
    right_bumper: Option<Remap>,
    left_trigger: Option<Remap>,
    right_trigger: Option<Remap>,
    dpad_up: Option<Remap>,
    dpad_down: Option<Remap>,
    dpad_left: Option<Remap>,
    dpad_right: Option<Remap>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub run_once: bool,

    #[serde(default = "Config::default_key_repeat_delay")]
    pub key_repeat_delay: Duration,

    #[serde(default = "Config::default_key_initial_delay_multi")]
    pub key_initial_delay_multi: Duration,

    #[serde(default = "Config::default_left_stick_poll_interval")]
    pub left_stick_poll_interval: Duration,

    #[serde(default = "Config::default_left_stick_dead_zone")]
    pub left_stick_dead_zone: f32,

    #[serde(default = "Config::default_initial_mouse_speed_div")]
    pub initial_mouse_speed_div: f32,

    #[serde(default = "Config::default_mouse_acceleration_rate")]
    pub mouse_acceleration_rate: f32,

    #[serde(default = "Config::default_max_mouse_speed_div")]
    pub max_mouse_speed_div: f32,

    #[serde(default = "Config::default_right_stick_poll_interval")]
    pub right_stick_poll_interval: Duration,

    #[serde(default = "Config::default_right_stick_trigger_zone")]
    pub right_stick_trigger_zone: f32,

    #[serde(default = "Config::default_right_stick_dead_zone")]
    pub right_stick_dead_zone: f32,

    pub alternative_activation: Option<String>,

    pub main: Set,
    pub alt: Set,
}

impl Config {
    fn default_key_repeat_delay() -> Duration {
        Duration::from_millis(40)
    }

    fn default_key_initial_delay_multi() -> Duration {
        Duration::from_millis(400)
    }

    fn default_left_stick_poll_interval() -> Duration {
        Duration::from_millis(10)
    }

    fn default_left_stick_dead_zone() -> f32 {
        2048.
    }

    fn default_initial_mouse_speed_div() -> f32 {
        4096.
    }

    fn default_mouse_acceleration_rate() -> f32 {
        64.
    }

    fn default_max_mouse_speed_div() -> f32 {
        2048.
    }

    fn default_right_stick_poll_interval() -> Duration {
        Duration::from_millis(50)
    }

    fn default_right_stick_trigger_zone() -> f32 {
        24576.
    }

    fn default_right_stick_dead_zone() -> f32 {
        4096.
    }

    pub fn check_validity(&self) -> std::result::Result<&Self, String> {
        if self.left_stick_dead_zone <= 0. || self.right_stick_trigger_zone <= 0. || self.right_stick_dead_zone <= 0. {
            return Err(String::from("Negative zone size"));
        }
        if self.right_stick_trigger_zone < self.right_stick_dead_zone {
            return Err(String::from("Trigger zone smaller than dead zone"));
        }
        if let Some(alt_input) = &self.alternative_activation {
            if self.get_remap(alt_input, false).is_some() {
                return Err(String::from("Alternative activation input is remapped"));
            }
        }

        Ok(self)
    }

    pub fn get_remap(&self, input: &str, is_alt: bool) -> &Option<Remap> {
        let active_set = if !is_alt { &self.main } else { &self.alt };

        if let Some(button) = sdl2::controller::Button::from_string(input) {
            match button {
                sdl2::controller::Button::A => &active_set.a,
                sdl2::controller::Button::B => &active_set.b,
                sdl2::controller::Button::X => &active_set.x,
                sdl2::controller::Button::Y => &active_set.y,
                sdl2::controller::Button::Back => &active_set.select,
                sdl2::controller::Button::Start => &active_set.start,
                sdl2::controller::Button::LeftStick => &active_set.left_stick_button,
                sdl2::controller::Button::RightStick => &active_set.right_stick_button,
                sdl2::controller::Button::LeftShoulder => &active_set.left_bumper,
                sdl2::controller::Button::RightShoulder => &active_set.right_bumper,
                sdl2::controller::Button::DPadUp => &active_set.dpad_up,
                sdl2::controller::Button::DPadDown => &active_set.dpad_down,
                sdl2::controller::Button::DPadLeft => &active_set.dpad_left,
                sdl2::controller::Button::DPadRight => &active_set.dpad_right,
                sdl2::controller::Button::Paddle1 => &active_set.right_stick_up,
                sdl2::controller::Button::Paddle2 => &active_set.right_stick_down,
                sdl2::controller::Button::Paddle3 => &active_set.right_stick_left,
                sdl2::controller::Button::Paddle4 => &active_set.right_stick_right,
                _ => &None,
            }
        } else if let Some(axis) = sdl2::controller::Axis::from_string(input) {
            match axis {
                sdl2::controller::Axis::TriggerLeft => &active_set.left_trigger,
                sdl2::controller::Axis::TriggerRight => &active_set.right_trigger,
                _ => &None,
            }
        } else {
            &None
        }
    }
}
