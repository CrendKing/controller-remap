#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod atomic_f32;
mod config;

use std::f32::consts::FRAC_PI_8;
use std::sync::atomic::{AtomicBool, Ordering};

use enigo::{Direction, Enigo, Keyboard as _, Mouse as _};
use gilrs::{Axis, Event, EventType, Gilrs};

use crate::atomic_f32::AtomicF32;
use crate::config::{Config, Remap};

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

struct Context {
    is_alternative_active: AtomicBool,
    left_stick_coord: Coordinate,
    right_stick_coord: Coordinate,
    config: Config,
    enigo: tokio::sync::Mutex<Enigo>,
    repeat_key_abort_handle: tokio::sync::Mutex<Option<tokio::task::AbortHandle>>,
}

impl Context {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            is_alternative_active: AtomicBool::new(false),
            left_stick_coord: Coordinate::new(),
            right_stick_coord: Coordinate::new(),
            config: Config::new()?,
            enigo: tokio::sync::Mutex::new(Enigo::new(&enigo::Settings::default())?),
            repeat_key_abort_handle: tokio::sync::Mutex::new(None),
        })
    }

    async fn reset(&self) -> anyhow::Result<()> {
        self.is_alternative_active.store(false, Ordering::Relaxed);
        self.left_stick_coord.reset();
        self.right_stick_coord.reset();

        let mut enigo_guard = self.enigo.lock().await;

        for held_key in enigo_guard.held().0 {
            enigo_guard.key(held_key, Direction::Release)?;
        }

        Ok(())
    }
}

static CONTEXT: std::sync::OnceLock<Context> = std::sync::OnceLock::new();

fn get_context() -> &'static Context {
    CONTEXT.get_or_init(|| Context::new().unwrap())
}

async fn press_input(input_name: &str, is_press_down: bool) -> anyhow::Result<()> {
    if let Some(activator) = &get_context().config.alternative_activator
        && input_name == activator.to_lowercase()
    {
        get_context().is_alternative_active.store(is_press_down, Ordering::Relaxed);
        return Ok(());
    }

    if let Some(remap) = get_context().config.get_remap(input_name, get_context().is_alternative_active.load(Ordering::Relaxed)) {
        match remap {
            Remap::Seq(seq) => {
                if is_press_down {
                    let mut enigo_guard = get_context().enigo.lock().await;

                    for key in seq {
                        enigo_guard.key(*key, Direction::Press)?;
                    }
                    for key in seq.iter().rev() {
                        enigo_guard.key(*key, Direction::Release)?;
                    }
                }
            }
            Remap::Sync(seq) => {
                let mut enigo_guard = get_context().enigo.lock().await;

                if is_press_down {
                    for key in seq {
                        enigo_guard.key(*key, Direction::Press)?;
                    }
                } else {
                    for key in seq.iter().rev() {
                        enigo_guard.key(*key, Direction::Release)?;
                    }
                }
            }
            Remap::Repeat(key) => {
                let mut abort_handle_lock_guard = get_context().repeat_key_abort_handle.lock().await;

                if let Some(handle) = &*abort_handle_lock_guard {
                    handle.abort();
                }

                if is_press_down {
                    /*
                        Tokio task is ideal tool for the repeat key use case:
                        - Small blocking code running on a separate thread
                        - Can be cancelled at any time
                    */
                    let repeat_task = tokio::task::spawn(async move {
                        get_context().enigo.lock().await.key(*key, Direction::Click).unwrap();
                        tokio::time::sleep(get_context().config.key_repeat_initial_delay).await;

                        loop {
                            get_context().enigo.lock().await.key(*key, Direction::Click).unwrap();
                            tokio::time::sleep(get_context().config.key_repeat_sub_delay).await;
                        }
                    });
                    *abort_handle_lock_guard = Some(repeat_task.abort_handle());
                }
            }
            Remap::Mouse(button) => {
                get_context()
                    .enigo
                    .lock()
                    .await
                    .button(*button, if is_press_down { Direction::Press } else { Direction::Release })?;
            }
            Remap::Command(cmdline) => {
                if is_press_down
                    && let Some(components) = shlex::split(cmdline)
                    && !components.is_empty()
                {
                    let _ = std::process::Command::new(&components[0]).args(&components[1..]).spawn();
                }
            }
        }
    }

    Ok(())
}

async fn left_stick() -> anyhow::Result<!> {
    let mouse_acceleration = (get_context().config.mouse_max_speed - get_context().config.mouse_initial_speed) / get_context().config.mouse_ticks_to_reach_max_speed;
    let mut curr_mouse_speed = get_context().config.mouse_initial_speed;
    let dead_zone_sq = get_context().config.left_stick_dead_zone * get_context().config.left_stick_dead_zone;

    loop {
        let x = get_context().left_stick_coord.x.load();
        let y = get_context().left_stick_coord.y.load();

        let distance_to_origin_sq = x * x + y * y;
        let is_mouse_moved = if distance_to_origin_sq > dead_zone_sq {
            let dead_zone_shrink_ratio = 1.0 - get_context().config.left_stick_dead_zone / distance_to_origin_sq.sqrt();
            let delta_x = (x * dead_zone_shrink_ratio * curr_mouse_speed).round() as i32;
            let delta_y = (y * dead_zone_shrink_ratio * curr_mouse_speed).round() as i32;

            if delta_x != 0 || delta_y != 0 {
                get_context().enigo.lock().await.move_mouse(delta_x, -delta_y, enigo::Coordinate::Rel)?;
                true
            } else {
                false
            }
        } else {
            false
        };

        curr_mouse_speed = if is_mouse_moved {
            (curr_mouse_speed + mouse_acceleration).min(get_context().config.mouse_max_speed)
        } else {
            get_context().config.mouse_initial_speed
        };

        tokio::time::sleep(get_context().config.left_stick_poll_interval).await;
    }
}

async fn right_stick() -> anyhow::Result<!> {
    const TRIGGER_ANGLES: [f32; 4] = [1.0 * FRAC_PI_8, 3.0 * FRAC_PI_8, 5.0 * FRAC_PI_8, 7.0 * FRAC_PI_8];
    let mut pressed_input_name = None;

    loop {
        let x = get_context().right_stick_coord.x.load();
        let y = get_context().right_stick_coord.y.load();
        let distance_to_origin = (x * x + y * y).sqrt();

        if distance_to_origin <= get_context().config.right_stick_dead_zone {
            if let Some(input_name) = pressed_input_name {
                press_input(input_name, false).await?;
                pressed_input_name = None;
            }
        } else if distance_to_origin >= get_context().config.right_stick_trigger_zone && pressed_input_name.is_none() {
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
                press_input(input_name, true).await?;
            }
        }

        tokio::time::sleep(get_context().config.right_stick_poll_interval).await;
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

#[tokio::main(worker_threads = 1)]
async fn main() -> anyhow::Result<!> {
    std::mem::forget(singleton_process::SingletonProcess::try_new(None, true)?);

    std::panic::set_hook(Box::new(|panic_info| {
        println!("{panic_info}\n\nStack trace:\n{}", std::backtrace::Backtrace::force_capture());
    }));

    // we don't care about the terminal state of the user command processes, and don't want them to become zombies
    // set SA_NOCLDWAIT to the SIGCHLD signal
    #[cfg(target_os = "linux")]
    unsafe {
        use nix::sys::signal::*;
        sigaction(Signal::SIGCHLD, &SigAction::new(SigHandler::SigDfl, SaFlags::SA_NOCLDWAIT, SigSet::empty()))
    }?;

    tokio::task::spawn(left_stick());
    tokio::task::spawn(right_stick());

    let mut gilrs = Gilrs::new().unwrap();

    loop {
        if let Some(Event { event, .. }) = gilrs.next_event_blocking(None) {
            match event {
                EventType::Disconnected => get_context().reset().await?,
                EventType::ButtonPressed(button, ..) | EventType::ButtonReleased(button, ..) => {
                    let is_press = matches!(event, EventType::ButtonPressed(..));
                    if let Some(input_name) = get_button_input_name(button) {
                        press_input(input_name, is_press).await?;
                    }
                }
                EventType::AxisChanged(axis, value, ..) => match axis {
                    Axis::LeftStickX => get_context().left_stick_coord.x.store(value),
                    Axis::LeftStickY => get_context().left_stick_coord.y.store(value),
                    Axis::RightStickX => get_context().right_stick_coord.x.store(value),
                    Axis::RightStickY => get_context().right_stick_coord.y.store(value),
                    _ => (),
                },
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn basic() -> anyhow::Result<()> {
        press_input("north", true).await?;

        tokio::task::spawn(left_stick());
        tokio::task::spawn(right_stick());

        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }
}
