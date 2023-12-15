pub enum KeyPressType {
    Down,
    Up,
    DownAndUp,
}

#[cfg(target_os = "windows")]
pub mod windows {
    use std::mem::{size_of, zeroed};

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    use crate::input::KeyPressType;

    pub struct WindowsInput;

    impl WindowsInput {
        unsafe fn create_mouse_input(mouse_button_name: &str, press_type: &KeyPressType) -> INPUT {
            let flag = match mouse_button_name {
                "LButton" => {
                    if matches!(press_type, KeyPressType::Up) {
                        MOUSEEVENTF_LEFTUP
                    } else {
                        MOUSEEVENTF_LEFTDOWN
                    }
                }
                "RButton" => {
                    if matches!(press_type, KeyPressType::Up) {
                        MOUSEEVENTF_RIGHTUP
                    } else {
                        MOUSEEVENTF_RIGHTDOWN
                    }
                }
                "MButton" => {
                    if matches!(press_type, KeyPressType::Up) {
                        MOUSEEVENTF_MIDDLEUP
                    } else {
                        MOUSEEVENTF_MIDDLEDOWN
                    }
                }
                _ => unimplemented!("Unrecognized mouse button name: {}", mouse_button_name),
            };

            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT { dwFlags: flag, ..zeroed() },
                },
            }
        }

        unsafe fn create_keyboard_input(key_name: &str, press_type: &KeyPressType) -> INPUT {
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        // https://github.com/libsdl-org/SDL/blob/SDL2/src/events/SDL_keyboard.c#L350
                        wVk: sdl2::keyboard::Keycode::from_name(key_name).unwrap() as _,
                        dwFlags: if matches!(press_type, KeyPressType::Up) { KEYEVENTF_KEYUP } else { 0 },
                        ..zeroed()
                    },
                },
            }
        }

        pub fn press(input_names: &[impl AsRef<str>], press_type: KeyPressType) {
            unsafe {
                let create_input_fn = if input_names[0].as_ref().ends_with("Button") {
                    Self::create_mouse_input
                } else {
                    // keyboard
                    Self::create_keyboard_input
                };

                let input_names_str = input_names.iter().map(AsRef::as_ref);
                let mut inputs: Vec<INPUT> = input_names_str.clone().map(|input_name| create_input_fn(input_name, &press_type)).collect::<Vec<_>>();

                if matches!(press_type, KeyPressType::DownAndUp) {
                    inputs.extend(input_names_str.rev().map(|input_name| create_input_fn(input_name, &KeyPressType::Up)));
                }

                SendInput(inputs.len() as _, inputs.as_ptr(), size_of::<INPUT>() as _);
            }
        }

        pub fn move_mouse(delta_x: i32, delta_y: i32) {
            unsafe {
                SendInput(
                    1,
                    &INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: delta_x,
                                dy: delta_y,
                                dwFlags: MOUSEEVENTF_MOVE,
                                ..zeroed()
                            },
                        },
                    },
                    size_of::<INPUT>() as _,
                );
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub mod linux {
    use std::sync::Arc;

    use uinput::event::controller::Controller::Mouse;
    use uinput::event::controller::Mouse::{Left, Middle, Right};
    use uinput::event::keyboard;
    use uinput::event::relative::Position::{X, Y};
    use uinput::event::relative::Relative::Position;
    use uinput::event::Event::{Controller, Relative};
    use uinput::Device;

    use crate::config::Config;
    use crate::input::{Input, KeyPressType};

    pub struct LinuxInput {
        config: Arc<Config>,
        device: Device,
    }

    impl LinuxInput {
        pub fn new(config: Arc<Config>) -> Self {
            let device = uinput::default()
                .unwrap()
                .name("Controller-Remap")
                .unwrap()
                .event(uinput::event::Keyboard::All)
                .unwrap()
                .event(Controller(Mouse(Left)))
                .unwrap()
                .event(Controller(Mouse(Right)))
                .unwrap()
                .event(Controller(Mouse(Middle)))
                .unwrap()
                .event(Relative(Position(X)))
                .unwrap()
                .event(Relative(Position(Y)))
                .unwrap()
                .create()
                .unwrap();
            Self { config, device }
        }

        pub fn repeat_key(&self, key: Keycode) -> SyncSender<bool> {
            Self::_repeat_key(self.config.clone(), key)
        }
    }

    impl Input for LinuxInput {
        fn press(&self, keys: &[Keycode], press_type: KeyPressType) {
            match press_type {
                KeyPressType::Down => self.device.press(&keyboard::Key::H).unwrap(),
                KeyPressType::Up => self.device.release(&keyboard::Key::H).unwrap(),
                KeyPressType::DownAndUp => self.device.click(&keyboard::Key::H).unwrap(),
            }

            device.synchronize().unwrap();
        }

        fn click_mouse(button: crate::input::MouseButton, press_type: KeyPressType) {
            match press_type {
                KeyPressType::Down => self.device.press(&controller::Mouse::Left).unwrap(),
                KeyPressType::Up => self.device.release(&controller::Mouse::Left).unwrap(),
                KeyPressType::DownAndUp => self.device.click(&controller::Mouse::Left).unwrap(),
            }

            device.synchronize().unwrap();
        }

        fn move_mouse(delta_x: i32, delta_y: i32) {
            device.send(X, delta_x).unwrap();
            device.send(Y, delta_y).unwrap();
            device.synchronize().unwrap();
        }
    }
}
