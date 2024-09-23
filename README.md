Remap controller to keyboard and mouse inputs.

### Features

* Map controller left stick to mouse and right stick to keyboard inputs.
* Simulate key repeats by holding a controller button. The initial input is designed to always happen.
* Simulate sequence of key inputs by pressing a controller button. The keys are pressed down in sequence when the controller is pressed down, and back up in the reverse order when the controller is released.
* Run executable command by pressing a controller button.
* Two sets of mappings, triggered by an "activator" button.
* Supports both Windows and Linux.

### Config

The app looks for a `<executable_name>.toml` file as remap config. The config is a toml file with the following format:

#### Top-level configs

* `key_repeat_initial_delay`: Duration string. Delay between the initial input and subsequent repeats. Takes various "human" formats such as `400ms` thanks to [duration_str](https://docs.rs/duration-str/latest/duration_str/).
* `key_repeat_sub_delay`: Duration string. Delay between each key repeats. Same format as above.
* `left_stick_poll_interval`: Duration string. The left and right sticks are polled in interval.
* `left_stick_dead_zone`: Decimal. Stick "zone" values range between 0 and 1. Movements with the deadzone are ignored.
* `mouse_initial_speed`: Decimal. Mouse movements are simulated to have acceleration. This is the initial speed.
* `mouse_max_speed`: Decimal. Maximum mouse movement speed.
* `mouse_ticks_to_reach_max_speed`: Decimal. Ticks (number of left stick polls) to reach the maximum speed.
* `right_stick_poll_interval`: Duration string.
* `right_stick_trigger_zone`: Decimal. Movement within the trigger zone are ignored.
* `right_stick_dead_zone`: Decimal.
* `alternative_activator`: String. The controller button name to be held down to switch to the alternative set.

#### Mapping sets

There are two predefined sets: `main` and `alt`. Both are toml tables. Each set contains the mappings from controller inputs to keyboard and mouse outputs. By default the `main` set is used, unless the `alternative_activator` button is held down.

The controller input names are based on [gilrs' naming convention](https://docs.rs/gilrs/0.10.4/gilrs/ev/enum.Button.html#variants), while being the snake case. For example, `left_bumper` is used to trigger a left bumper input.
To map the right stick as 4-way keypad, `right_stick_up`, `right_stick_down`, `right_stick_left` and `right_stick_right` are also supported.

The keyboard output names are based on [enigo's naming convention](https://docs.rs/enigo/0.2.0-rc2/enigo/enum.Key.html#variants). For example, `Control` stands for the control key.

The mapping value is an one-entry toml table. These are the supported mapping keys:

* `seq`: Value is a list of strings, each being keyboard output name. Keys are immediately released in reverse order.
* `sync`: Value is a list of strings, each being keyboard output name. Key releases are synced with the controller button release.
* `repeat`: Value is a single string of keyboard output name. The key is repeatedly pressed.
* `mouse`: Value is either `Left`, `Right` or `Middle`.
* `command`: Value is path to an executable file to run.

### Example
```
key_repeat_initial_delay = '400ms'
key_repeat_sub_delay = '40ms'
left_stick_poll_interval = '10ms'
left_stick_dead_zone = 0.05
mouse_initial_speed = 10
mouse_max_speed = 20
mouse_ticks_to_reach_max_speed = 30
right_stick_poll_interval = '50ms'
right_stick_trigger_zone = 0.3
right_stick_dead_zone = 0.1
alternative_activator = 'select'

[main]
north = { repeat = 'PageUp' }
south = { repeat = 'Return' }
west = { repeat = 'PageDown' }
east = { repeat = 'Space' }
left_bumper = { mouse = 'Left' }
right_bumper = { sync = ['Control'] }
left_trigger = { mouse = 'Right' }
right_trigger = { mouse = 'Middle' }
start = { sync = ['Shift'] }
left_thumb = { seq = ['Control', 'Meta', 'O'] }
right_thumb = { seq = ['Control', 'W'] }
dpad_up = { repeat = 'UpArrow' }
dpad_down = { repeat = 'DownArrow' }
dpad_left = { repeat = 'LeftArrow' }
dpad_right = { repeat = 'RightArrow' }
right_stick_up = { seq = ['F5'] }
right_stick_down = { seq = ['Control', 'C'] }
right_stick_left = { seq = ['Control', 'V'] }
right_stick_right = { seq = ['Control', 'Alt', 'Tab'] }

[alt]
north = { seq = ['Home'] }
south = { seq = ['End'] }
west = { repeat = 'Backspace' }
east = { repeat = 'Delete' }
right_stick_up = { seq = ['Escape'] }
right_stick_down = { seq = ['Meta', 'D'] }
right_stick_left = { seq = ['Control', 'Alt', { Unicode = '1' }] }
right_stick_right = { seq = ['Meta', 'X'] }
```
