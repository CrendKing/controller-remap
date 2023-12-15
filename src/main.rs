mod config;
mod input;

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread::sleep;

use sdl2::controller::Axis;
use sdl2::event::Event;

use crate::config::Config;
#[cfg(target_os = "linux")]
use crate::input::windows::LinuxInput as PlatformInput;
#[cfg(target_os = "windows")]
use crate::input::windows::WindowsInput as PlatformInput;
use crate::input::KeyPressType;

static CONFIG_LOCK: OnceLock<Config> = OnceLock::new();
static IS_ALTERNATIVE_ACTIVE: AtomicBool = AtomicBool::new(false);
static LEFT_STICK_COORD: (AtomicI32, AtomicI32) = (AtomicI32::new(0), AtomicI32::new(0));
static RIGHT_STICK_COORD: (AtomicI32, AtomicI32) = (AtomicI32::new(0), AtomicI32::new(0));
static REPEAT_KEY_SIGNAL: Mutex<Option<SyncSender<bool>>> = Mutex::new(None);

fn press(name: &str, is_press_down: bool) {
    let config = CONFIG_LOCK.get().unwrap();

    if let Some(alt_input) = &config.alternative_activation {
        if name == alt_input {
            IS_ALTERNATIVE_ACTIVE.store(is_press_down, Ordering::Relaxed);
            return;
        }
    }

    if let Some(remap) = config.get_remap(name, IS_ALTERNATIVE_ACTIVE.load(Ordering::Relaxed)) {
        match remap {
            crate::config::Remap::keys(seq) => {
                if is_press_down {
                    PlatformInput::press(&seq, KeyPressType::Down);
                } else {
                    PlatformInput::press(&seq.iter().rev().collect::<Vec<_>>(), KeyPressType::Up);
                }
            }
            crate::config::Remap::repeat(key) => {
                let mut signal_lock = REPEAT_KEY_SIGNAL.lock().unwrap();

                if let Some(signal) = signal_lock.as_ref() {
                    signal.send(false).unwrap();
                }

                *signal_lock = if is_press_down {
                    let (tx, rx) = sync_channel(0);

                    std::thread::spawn(move || {
                        PlatformInput::press(&[key], KeyPressType::DownAndUp);

                        sleep(config.key_initial_delay_multi);

                        while rx.try_recv().is_err() {
                            PlatformInput::press(&[key], KeyPressType::DownAndUp);
                            sleep(config.key_repeat_delay);
                        }
                    });

                    Some(tx)
                } else {
                    None
                };
            }
        }
    }
}

fn left_stick() {
    let config = CONFIG_LOCK.get().unwrap();
    let mut curr_mouse_acceleration = config.initial_mouse_speed_div;

    loop {
        let x = LEFT_STICK_COORD.0.load(Ordering::Relaxed);
        let y: i32 = LEFT_STICK_COORD.1.load(Ordering::Relaxed);
        let distance_to_origin = ((x * x + y * y) as f32).sqrt();
        let dead_zone_shrink_ratio = (1. - (config.left_stick_dead_zone) / distance_to_origin).max(0.);
        let delta_x = ((x as f32) * dead_zone_shrink_ratio / curr_mouse_acceleration) as i32;
        let delta_y = ((y as f32) * dead_zone_shrink_ratio / curr_mouse_acceleration) as i32;

        if delta_x != 0 || delta_y != 0 {
            PlatformInput::move_mouse(delta_x, delta_y);
            curr_mouse_acceleration = (curr_mouse_acceleration - config.mouse_acceleration_rate).max(config.max_mouse_speed_div);
        } else {
            curr_mouse_acceleration = config.initial_mouse_speed_div;
        }

        sleep(config.left_stick_poll_interval);
    }
}

fn right_stick() {
    let config = CONFIG_LOCK.get().unwrap();
    let trigger_angle_precompute = [
        std::f32::consts::FRAC_PI_8,
        3. * std::f32::consts::FRAC_PI_8,
        5. * std::f32::consts::FRAC_PI_8,
        7. * std::f32::consts::FRAC_PI_8,
    ];
    let mut pressed_input_name: Option<String> = None;

    loop {
        let x = RIGHT_STICK_COORD.0.load(Ordering::Relaxed);
        let y = RIGHT_STICK_COORD.1.load(Ordering::Relaxed);
        let distance_to_origin = ((x * x + y * y) as f32).sqrt();

        if distance_to_origin <= config.right_stick_dead_zone {
            if let Some(input_name) = pressed_input_name.as_ref() {
                press(input_name, true);
                pressed_input_name = None;
            }
        } else if distance_to_origin >= config.right_stick_trigger_zone && pressed_input_name.is_none() {
            let stick_angle = (y as f32).atan2(x as f32);

            pressed_input_name = if stick_angle >= -trigger_angle_precompute[2] && stick_angle <= -trigger_angle_precompute[1] {
                // up
                Some(sdl2::controller::Button::Paddle1.string())
            } else if stick_angle >= trigger_angle_precompute[1] && stick_angle <= trigger_angle_precompute[2] {
                // down
                Some(sdl2::controller::Button::Paddle2.string())
            } else if stick_angle >= trigger_angle_precompute[3] || stick_angle <= -trigger_angle_precompute[3] {
                // left
                Some(sdl2::controller::Button::Paddle3.string())
            } else if stick_angle >= -trigger_angle_precompute[0] && stick_angle <= trigger_angle_precompute[0] {
                // right
                Some(sdl2::controller::Button::Paddle4.string())
            } else {
                None
            };

            if let Some(input_name) = pressed_input_name.as_ref() {
                press(input_name, true);
            }
        }

        sleep(config.right_stick_poll_interval);
    }
}

fn main() {
    let config_path = std::env::current_exe().unwrap().with_extension("toml");
    let config_str = std::fs::read_to_string(&config_path).unwrap_or_else(|_| panic!("Unable to open config file at {}", config_path.display()));
    let config = toml::from_str::<Config>(&config_str).unwrap();
    config.check_validity().unwrap();
    CONFIG_LOCK.set(config).unwrap();

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let controller_sys = sdl_context.game_controller().unwrap();
    let mut opt_controller = None;

    std::thread::scope(|s| {
        s.spawn(left_stick);
        s.spawn(right_stick);

        loop {
            for event in event_pump.wait_iter() {
                match event {
                    Event::ControllerDeviceAdded { which, .. } => opt_controller = Some(controller_sys.open(which).unwrap()),
                    Event::ControllerDeviceRemoved { .. } => {
                        if CONFIG_LOCK.get().unwrap().run_once {
                            std::process::exit(0);
                        } else {
                            let mut signal_lock = REPEAT_KEY_SIGNAL.lock().unwrap();
                            if let Some(signal) = signal_lock.as_ref() {
                                signal.send(false).unwrap();
                                *signal_lock = None;
                            }

                            opt_controller = None;
                        }
                    }
                    Event::ControllerButtonDown { button, .. } => press(&button.string(), true),
                    Event::ControllerButtonUp { button, .. } => press(&button.string(), false),
                    Event::ControllerAxisMotion { axis, value, .. } => match axis {
                        Axis::LeftX => LEFT_STICK_COORD.0.store(value as _, Ordering::Relaxed),
                        Axis::LeftY => LEFT_STICK_COORD.1.store(value as _, Ordering::Relaxed),
                        Axis::RightX => RIGHT_STICK_COORD.0.store(value as _, Ordering::Relaxed),
                        Axis::RightY => RIGHT_STICK_COORD.1.store(value as _, Ordering::Relaxed),
                        Axis::TriggerLeft | Axis::TriggerRight => press(&axis.string(), value > 0),
                    },
                    _ => (),
                }
            }
        }
    });
}
