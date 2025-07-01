#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod atomic_f32;
mod config;

use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use enigo::{Direction, Enigo, Keyboard, Mouse};
use gilrs::{Axis, Event, EventType, Gilrs};

use crate::atomic_f32::*;
use crate::config::*;

struct Coordinate {
    x: AtomicF32,
    y: AtomicF32,
}

impl Coordinate {
    const fn new() -> Self {
        Self {
            x: AtomicF32::new(),
            y: AtomicF32::new(),
        }
    }

    fn reset(&self) {
        self.x.reset();
        self.y.reset();
    }
}

const INPUT_LOOP_TIMEOUT: Duration = Duration::from_secs(3);

static IS_ALTERNATIVE_ACTIVE: AtomicBool = AtomicBool::new(false);
static LEFT_STICK_COORD: Coordinate = Coordinate::new();
static RIGHT_STICK_COORD: Coordinate = Coordinate::new();

static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::try_new().unwrap());
static ENIGO: LazyLock<std::sync::Mutex<Enigo>> = LazyLock::new(|| std::sync::Mutex::new(Enigo::new(&enigo::Settings::default()).unwrap()));
static REPEAT_KEY_ABORT_HANDLE: std::sync::Mutex<Option<tokio::task::AbortHandle>> = std::sync::Mutex::new(None);

fn press_input(input_name: &str, is_press_down: bool) {
    if let Some(activator) = &CONFIG.alternative_activator
        && input_name == activator.to_lowercase()
    {
        IS_ALTERNATIVE_ACTIVE.store(is_press_down, Ordering::Relaxed);
        return;
    }

    if let Some(remap) = CONFIG.get_remap(input_name, IS_ALTERNATIVE_ACTIVE.load(Ordering::Relaxed)) {
        match remap {
            Remap::Seq(seq) => {
                if is_press_down {
                    let mut enigo = ENIGO.lock().unwrap();

                    for key in seq.iter() {
                        enigo.key(*key, Direction::Press).unwrap();
                    }
                    for key in seq.iter().rev() {
                        enigo.key(*key, Direction::Release).unwrap();
                    }
                }
            }
            Remap::Sync(seq) => {
                let mut enigo = ENIGO.lock().unwrap();

                if is_press_down {
                    for key in seq.iter() {
                        enigo.key(*key, Direction::Press).unwrap();
                    }
                } else {
                    for key in seq.iter().rev() {
                        enigo.key(*key, Direction::Release).unwrap();
                    }
                }
            }
            Remap::Repeat(key) => {
                let mut abort_handle_lock = REPEAT_KEY_ABORT_HANDLE.lock().unwrap();

                if let Some(handle) = &*abort_handle_lock {
                    handle.abort();
                }

                if is_press_down {
                    ENIGO.lock().unwrap().key(*key, Direction::Click).unwrap();

                    *abort_handle_lock = Some(
                        tokio::spawn(async {
                            tokio::time::sleep(CONFIG.key_repeat_initial_delay).await;

                            loop {
                                ENIGO.lock().unwrap().key(*key, Direction::Click).unwrap();
                                tokio::time::sleep(CONFIG.key_repeat_sub_delay).await;
                            }
                        })
                        .abort_handle(),
                    );
                }
            }
            Remap::Mouse(button) => {
                ENIGO
                    .lock()
                    .unwrap()
                    .button(*button, if is_press_down { Direction::Press } else { Direction::Release })
                    .unwrap();
            }
            Remap::Command(cmdline) => {
                if is_press_down
                    && let Some(components) = shlex::split(cmdline)
                    && !components.is_empty()
                {
                    std::process::Command::new(&components[0]).args(&components[1..]).spawn().ok();
                }
            }
        }
    }
}

async fn left_stick() {
    let mouse_acceleration = (CONFIG.mouse_max_speed - CONFIG.mouse_initial_speed) / CONFIG.mouse_ticks_to_reach_max_speed;
    let mut curr_mouse_speed = CONFIG.mouse_initial_speed;

    loop {
        let x = LEFT_STICK_COORD.x.load();
        let y = LEFT_STICK_COORD.y.load();
        let distance_to_origin = (x * x + y * y).sqrt();
        let dead_zone_shrink_ratio = (1.0 - (CONFIG.left_stick_dead_zone) / distance_to_origin).max(0.0);
        let delta_x = x * dead_zone_shrink_ratio * curr_mouse_speed;
        let delta_y = y * dead_zone_shrink_ratio * curr_mouse_speed;

        if delta_x != 0.0 || delta_y != 0.0 {
            ENIGO.lock().unwrap().move_mouse(delta_x as i32, -delta_y as i32, enigo::Coordinate::Rel).unwrap();
            curr_mouse_speed = (curr_mouse_speed + mouse_acceleration).min(CONFIG.mouse_max_speed);
        } else {
            curr_mouse_speed = CONFIG.mouse_initial_speed;
        }

        tokio::time::sleep(CONFIG.left_stick_poll_interval).await;
    }
}

async fn right_stick() {
    const TRIGGER_ANGLES: [f32; 4] = [
        1.0 * std::f32::consts::FRAC_PI_8,
        3.0 * std::f32::consts::FRAC_PI_8,
        5.0 * std::f32::consts::FRAC_PI_8,
        7.0 * std::f32::consts::FRAC_PI_8,
    ];
    let mut pressed_input_name = None;

    loop {
        let x = RIGHT_STICK_COORD.x.load();
        let y = RIGHT_STICK_COORD.y.load();
        let distance_to_origin = (x * x + y * y).sqrt();

        if distance_to_origin <= CONFIG.right_stick_dead_zone {
            if let Some(input_name) = pressed_input_name {
                press_input(input_name, false);
                pressed_input_name = None;
            }
        } else if distance_to_origin >= CONFIG.right_stick_trigger_zone && pressed_input_name.is_none() {
            let stick_angle = y.atan2(x);
            let abs_stick_angle = stick_angle.abs();

            pressed_input_name = if (TRIGGER_ANGLES[1]..=TRIGGER_ANGLES[2]).contains(&abs_stick_angle) {
                if stick_angle > 0.0 { Some("right_stick_up") } else { Some("right_stick_down") }
            } else if abs_stick_angle >= TRIGGER_ANGLES[3] {
                Some("right_stick_left")
            } else if abs_stick_angle <= TRIGGER_ANGLES[0] {
                Some("right_stick_right")
            } else {
                None
            };

            if let Some(input_name) = pressed_input_name {
                press_input(input_name, true);
            }
        }

        tokio::time::sleep(CONFIG.right_stick_poll_interval).await;
    }
}

fn get_button_input_name(button: gilrs::Button) -> Option<&'static str> {
    match button {
        gilrs::Button::North => Some("north"),
        gilrs::Button::South => Some("south"),
        gilrs::Button::West => Some("west"),
        gilrs::Button::East => Some("east"),
        gilrs::Button::LeftTrigger => Some("left_bumper"),
        gilrs::Button::RightTrigger => Some("right_bumper"),
        gilrs::Button::LeftTrigger2 => Some("left_trigger"),
        gilrs::Button::RightTrigger2 => Some("right_trigger"),
        gilrs::Button::Select => Some("select"),
        gilrs::Button::Start => Some("start"),
        gilrs::Button::Mode => Some("mode"),
        gilrs::Button::LeftThumb => Some("left_thumb"),
        gilrs::Button::RightThumb => Some("right_thumb"),
        gilrs::Button::DPadUp => Some("dpad_up"),
        gilrs::Button::DPadDown => Some("dpad_down"),
        gilrs::Button::DPadLeft => Some("dpad_left"),
        gilrs::Button::DPadRight => Some("dpad_right"),
        _ => None,
    }
}

#[tokio::main(worker_threads = 3)]
async fn main() {
    std::mem::forget(singleton_process::SingletonProcess::try_new(None, true).unwrap());

    // we don't care about the terminal state of the user command processes, and don't want them to become zombies
    // set SA_NOCLDWAIT to the SIGCHLD signal
    #[cfg(target_os = "linux")]
    unsafe {
        use nix::sys::signal::*;
        sigaction(Signal::SIGCHLD, &SigAction::new(SigHandler::SigDfl, SaFlags::SA_NOCLDWAIT, SigSet::empty())).unwrap();
    }

    tokio::spawn(left_stick());
    tokio::spawn(right_stick());

    loop {
        std::panic::catch_unwind(|| {
            let mut gilrs = Gilrs::new().unwrap();
            loop {
                if let Some(Event { event, .. }) = gilrs.next_event_blocking(Some(INPUT_LOOP_TIMEOUT)) {
                    match event {
                        EventType::Disconnected => {
                            IS_ALTERNATIVE_ACTIVE.store(false, Ordering::Relaxed);
                            LEFT_STICK_COORD.reset();
                            RIGHT_STICK_COORD.reset();

                            let mut enigo = ENIGO.lock().unwrap();

                            for held_key in enigo.held().0 {
                                enigo.key(held_key, Direction::Release).unwrap();
                            }
                        }
                        EventType::ButtonPressed(button, ..) => {
                            if let Some(input_name) = get_button_input_name(button) {
                                press_input(input_name, true);
                            }
                        }
                        EventType::ButtonReleased(button, ..) => {
                            if let Some(input_name) = get_button_input_name(button) {
                                press_input(input_name, false);
                            }
                        }
                        EventType::AxisChanged(axis, value, ..) => match axis {
                            Axis::LeftStickX => LEFT_STICK_COORD.x.store(value),
                            Axis::LeftStickY => LEFT_STICK_COORD.y.store(value),
                            Axis::RightStickX => RIGHT_STICK_COORD.x.store(value),
                            Axis::RightStickY => RIGHT_STICK_COORD.y.store(value),
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }
        })
        .ok();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_baseline() {
        press_input("north", true);

        tokio::time::timeout(Duration::from_secs(1), left_stick()).await.ok();
        tokio::time::timeout(Duration::from_secs(1), right_stick()).await.ok();
    }
}
