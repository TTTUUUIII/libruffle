use std::{
    collections::HashMap,
    fmt::Display,
    i32,
    sync::{
        Mutex, MutexGuard,
        atomic::{self, AtomicBool},
    },
};

use ndk::event::Keycode;
use once_cell::sync::Lazy;
use ruffle_core::{
    Player, PlayerEvent,
    events::{KeyDescriptor, KeyLocation, LogicalKey, MouseButton, NamedKey, PhysicalKey},
};

pub const RETRO_DEVICE_JOYPAD: i32 = 1;
pub const RETRO_DEVICE_POINTER: i32 = 6;

pub const RETRO_DEVICE_ID_JOYPAD_B: i32 = 0;
pub const RETRO_DEVICE_ID_JOYPAD_Y: i32 = 1;
pub const RETRO_DEVICE_ID_JOYPAD_SELECT: i32 = 2;
pub const RETRO_DEVICE_ID_JOYPAD_START: i32 = 3;
pub const RETRO_DEVICE_ID_JOYPAD_UP: i32 = 4;
pub const RETRO_DEVICE_ID_JOYPAD_DOWN: i32 = 5;
pub const RETRO_DEVICE_ID_JOYPAD_LEFT: i32 = 6;
pub const RETRO_DEVICE_ID_JOYPAD_RIGHT: i32 = 7;
pub const RETRO_DEVICE_ID_JOYPAD_A: i32 = 8;
pub const RETRO_DEVICE_ID_JOYPAD_X: i32 = 9;
pub const RETRO_DEVICE_ID_JOYPAD_L: i32 = 10;
pub const RETRO_DEVICE_ID_JOYPAD_R: i32 = 11;
// pub const RETRO_DEVICE_ID_JOYPAD_L2: i32        = 12;
// pub const RETRO_DEVICE_ID_JOYPAD_R2: i32        = 13;
// pub const RETRO_DEVICE_ID_JOYPAD_L3: i32        = 14;
// pub const RETRO_DEVICE_ID_JOYPAD_R3: i32        = 15;
pub const RETRO_DEVICE_ID_JOYPAD_MASK: i32 = 256;

pub const RETRO_DEVICE_ID_POINTER_X: i32 = 0;
pub const RETRO_DEVICE_ID_POINTER_Y: i32 = 1;
pub const RETRO_DEVICE_ID_POINTER_PRESSED: i32 = 2;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum KeyAction {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputSource {
    Keyboard,
    Gamepad,
}

impl Display for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if *self == KeyAction::Down {
                "Down"
            } else {
                "Up"
            }
        )
    }
}

impl From<i32> for KeyAction {
    fn from(action: i32) -> Self {
        if action == 0 {
            KeyAction::Down
        } else {
            KeyAction::Up
        }
    }
}

impl From<bool> for KeyAction {
    fn from(is_down: bool) -> Self {
        if is_down {
            KeyAction::Down
        } else {
            KeyAction::Up
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    port: i32,
    key: Keycode,
    action: KeyAction,
    source: InputSource,
}

impl Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KeyEvent: {{ key={}, action={} }}",
            keycode_as_string(&self.key),
            self.action
        )
    }
}

impl KeyEvent {
    pub fn new(key: Keycode, action: KeyAction) -> Self {
        Self {
            port: 0,
            key,
            action,
            source: InputSource::Keyboard,
        }
    }

    pub fn gamepad(port: i32, key: Keycode, action: KeyAction) -> Self {
        Self {
            port,
            key,
            action,
            source: InputSource::Gamepad,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TouchEvent {
    x: f64,
    y: f64,
    action: KeyAction,
}

impl TouchEvent {
    pub fn new(x: f64, y: f64, action: KeyAction) -> Self {
        Self { x, y, action }
    }
}

struct KeyState {
    action: KeyAction,
    descriptor: KeyDescriptor,
}

impl KeyState {
    fn new(descriptor: KeyDescriptor) -> Self {
        Self {
            action: KeyAction::Up,
            descriptor: descriptor,
        }
    }
}

static KEYCODE_DESCRIPTORS: Lazy<HashMap<i32, KeyDescriptor>> = Lazy::new(|| {
    let mut it = HashMap::new();
    for keycode in KEYCODE_ARRAY {
        it.insert(keycode.into(), keycode_as_descriptor(&keycode));
    }
    it
});

// for gamepad
static P0_ROUTE_TABLE: Lazy<Mutex<HashMap<i32, KeyState>>> =
    Lazy::new(|| Mutex::new(InputDispatcher::build_route_table(0)));
static P1_ROUTE_TABLE: Lazy<Mutex<HashMap<i32, KeyState>>> =
    Lazy::new(|| Mutex::new(InputDispatcher::build_route_table(1)));
static POINTER_ACTION: AtomicBool = AtomicBool::new(false);

pub struct InputDispatcher;
impl InputDispatcher {
    fn build_route_table(port: i32) -> HashMap<i32, KeyState> {
        let mut it = HashMap::new();

        if port == 1 {
            // Pad Left => A
            let mut key: i32 = Keycode::DpadLeft.into();
            let mut desc = Self::get_descriptor(Keycode::A).unwrap();
            it.insert(key, KeyState::new(desc));

            // Pad Right => D
            key = Keycode::DpadRight.into();
            desc = Self::get_descriptor(Keycode::D).unwrap();
            it.insert(key, KeyState::new(desc));

            // Pad Up => W
            key = Keycode::DpadUp.into();
            desc = Self::get_descriptor(Keycode::W).unwrap();
            it.insert(key, KeyState::new(desc));

            // Pad Down => S
            key = Keycode::DpadDown.into();
            desc = Self::get_descriptor(Keycode::S).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button A => G,
            key = Keycode::ButtonA.into();
            desc = Self::get_descriptor(Keycode::G).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button B => H
            key = Keycode::ButtonB.into();
            desc = Self::get_descriptor(Keycode::H).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button X => T
            key = Keycode::ButtonX.into();
            desc = Self::get_descriptor(Keycode::T).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button Y => Y
            key = Keycode::ButtonY.into();
            desc = Self::get_descriptor(Keycode::Y).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button Select => Tab
            key = Keycode::ButtonSelect.into();
            desc = Self::get_descriptor(Keycode::Tab).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button Start => Enter
            key = Keycode::ButtonStart.into();
            desc = Self::get_descriptor(Keycode::Enter).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button L1 => 7
            key = Keycode::ButtonL1.into();
            desc = Self::get_descriptor(Keycode::Keycode7).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button R1 => 8
            key = Keycode::ButtonR1.into();
            desc = Self::get_descriptor(Keycode::Keycode8).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button L2 => 9
            key = Keycode::ButtonL1.into();
            desc = Self::get_descriptor(Keycode::Keycode9).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button R2 => 0
            key = Keycode::ButtonR1.into();
            desc = Self::get_descriptor(Keycode::Keycode0).unwrap();
            it.insert(key, KeyState::new(desc));
        } else {
            // Pad Left => ArrowLeft
            let mut key: i32 = Keycode::DpadLeft.into();
            let mut desc = Self::get_descriptor(Keycode::DpadLeft).unwrap();
            it.insert(key, KeyState::new(desc));

            // Pad Right => ArrowRight
            key = Keycode::DpadRight.into();
            desc = Self::get_descriptor(Keycode::DpadRight).unwrap();
            it.insert(key, KeyState::new(desc));
            // Pad Up => ArrowUp
            key = Keycode::DpadUp.into();
            desc = Self::get_descriptor(Keycode::DpadUp).unwrap();
            it.insert(key, KeyState::new(desc));

            // Pad Down => ArrowDown
            key = Keycode::DpadDown.into();
            desc = Self::get_descriptor(Keycode::DpadDown).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button A => ,
            key = Keycode::ButtonA.into();
            desc = Self::get_descriptor(Keycode::Comma).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button B => .
            key = Keycode::ButtonB.into();
            desc = Self::get_descriptor(Keycode::Period).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button X => K
            key = Keycode::ButtonX.into();
            desc = Self::get_descriptor(Keycode::K).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button Y => L
            key = Keycode::ButtonY.into();
            desc = Self::get_descriptor(Keycode::L).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button Select => Tab
            key = Keycode::ButtonSelect.into();
            desc = Self::get_descriptor(Keycode::Tab).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button Start => Enter
            key = Keycode::ButtonStart.into();
            desc = Self::get_descriptor(Keycode::Enter).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button L1 => 1
            key = Keycode::ButtonL1.into();
            desc = Self::get_descriptor(Keycode::Keycode1).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button R1 => 2
            key = Keycode::ButtonR1.into();
            desc = Self::get_descriptor(Keycode::Keycode2).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button L2 => 3
            key = Keycode::ButtonL1.into();
            desc = Self::get_descriptor(Keycode::Keycode3).unwrap();
            it.insert(key, KeyState::new(desc));

            // Button R2 => 4
            key = Keycode::ButtonR1.into();
            desc = Self::get_descriptor(Keycode::Keycode4).unwrap();
            it.insert(key, KeyState::new(desc));
        }

        it
    }

    fn get_descriptor(keycode: Keycode) -> Option<KeyDescriptor> {
        let key: i32 = keycode.into();
        KEYCODE_DESCRIPTORS.get(&key).copied()
    }

    pub fn dispatch_touch_event<'a>(event: TouchEvent, player: &mut MutexGuard<'a, Player>) {
        let current_action = KeyAction::from(POINTER_ACTION.load(atomic::Ordering::Relaxed));
        if current_action != event.action {
            if event.action == KeyAction::Down {
                player.handle_event(PlayerEvent::MouseDown {
                    x: event.x,
                    y: event.y,
                    button: MouseButton::Left,
                    index: None,
                });
                POINTER_ACTION.store(true, atomic::Ordering::Relaxed);
            } else {
                player.handle_event(PlayerEvent::MouseUp {
                    x: event.x,
                    y: event.y,
                    button: MouseButton::Left,
                });
                POINTER_ACTION.store(false, atomic::Ordering::Relaxed);
            }
        } else if event.action == KeyAction::Down {
            player.handle_event(PlayerEvent::MouseMove {
                x: event.x,
                y: event.y,
            });
        }
    }

    pub fn dispatch_key_event<'a>(event: KeyEvent, player: &mut MutexGuard<'a, Player>) {
        if event.source == InputSource::Gamepad {
            let mut route_table = if event.port == 1 {
                P1_ROUTE_TABLE.lock().unwrap()
            } else {
                P0_ROUTE_TABLE.lock().unwrap()
            };
            if let Some(key_state) = route_table.get_mut(&event.key.into()) {
                if key_state.action != event.action {
                    if event.action == KeyAction::Down {
                        player.handle_event(PlayerEvent::KeyDown {
                            key: key_state.descriptor,
                        });
                    } else {
                        player.handle_event(PlayerEvent::KeyUp {
                            key: key_state.descriptor,
                        });
                    }
                    key_state.action = event.action;
                };
            }
        } else if let Some(descriptor) = KEYCODE_DESCRIPTORS.get(&event.key.into()) {
            if event.action == KeyAction::Down {
                player.handle_event(PlayerEvent::KeyDown { key: *descriptor });
            } else {
                player.handle_event(PlayerEvent::KeyUp { key: *descriptor });
            }
        }
    }
}

const KEYCODE_ARRAY: [Keycode; 108] = [
    Keycode::Home,
    Keycode::Keycode0,
    Keycode::Keycode1,
    Keycode::Keycode2,
    Keycode::Keycode3,
    Keycode::Keycode4,
    Keycode::Keycode5,
    Keycode::Keycode6,
    Keycode::Keycode7,
    Keycode::Keycode8,
    Keycode::Keycode9,
    Keycode::DpadUp,
    Keycode::DpadDown,
    Keycode::DpadLeft,
    Keycode::DpadRight,
    Keycode::A,
    Keycode::B,
    Keycode::C,
    Keycode::D,
    Keycode::E,
    Keycode::F,
    Keycode::G,
    Keycode::H,
    Keycode::I,
    Keycode::J,
    Keycode::K,
    Keycode::L,
    Keycode::M,
    Keycode::N,
    Keycode::O,
    Keycode::P,
    Keycode::Q,
    Keycode::R,
    Keycode::S,
    Keycode::T,
    Keycode::U,
    Keycode::V,
    Keycode::W,
    Keycode::X,
    Keycode::Y,
    Keycode::Z,
    Keycode::Comma,
    Keycode::Period,
    Keycode::AltLeft,
    Keycode::AltRight,
    Keycode::ShiftLeft,
    Keycode::ShiftRight,
    Keycode::Tab,
    Keycode::Space,
    Keycode::Enter,
    Keycode::Del,
    Keycode::Grave,
    Keycode::Minus,
    Keycode::Equals,
    Keycode::LeftBracket,
    Keycode::RightBracket,
    Keycode::Backslash,
    Keycode::Semicolon,
    Keycode::Apostrophe,
    Keycode::Slash,
    Keycode::PageUp,
    Keycode::PageDown,
    Keycode::Escape,
    Keycode::ForwardDel,
    Keycode::CtrlLeft,
    Keycode::CtrlRight,
    Keycode::CapsLock,
    Keycode::ScrollLock,
    Keycode::MetaLeft,
    Keycode::MetaRight,
    Keycode::Sysrq,
    Keycode::Break,
    Keycode::MoveHome,
    Keycode::MoveEnd,
    Keycode::Insert,
    Keycode::F1,
    Keycode::F2,
    Keycode::F3,
    Keycode::F4,
    Keycode::F5,
    Keycode::F6,
    Keycode::F7,
    Keycode::F8,
    Keycode::F9,
    Keycode::F10,
    Keycode::F11,
    Keycode::F12,
    Keycode::NumLock,
    Keycode::Numpad0,
    Keycode::Numpad1,
    Keycode::Numpad2,
    Keycode::Numpad3,
    Keycode::Numpad4,
    Keycode::Numpad5,
    Keycode::Numpad6,
    Keycode::Numpad7,
    Keycode::Numpad8,
    Keycode::Numpad9,
    Keycode::NumpadDivide,
    Keycode::NumpadMultiply,
    Keycode::NumpadSubtract,
    Keycode::NumpadAdd,
    Keycode::NumpadDot,
    Keycode::NumpadComma,
    Keycode::NumpadEnter,
    Keycode::NumpadEquals,
    Keycode::MediaPlay,
    Keycode::MediaPause,
];

fn keycode_as_string(keycode: &Keycode) -> &str {
    match keycode {
        Keycode::Home => "Home",
        Keycode::Keycode0 | Keycode::Numpad0 => "0",
        Keycode::Keycode1 | Keycode::Numpad1 => "1",
        Keycode::Keycode2 | Keycode::Numpad2 => "2",
        Keycode::Keycode3 | Keycode::Numpad3 => "3",
        Keycode::Keycode4 | Keycode::Numpad4 => "4",
        Keycode::Keycode5 | Keycode::Numpad5 => "5",
        Keycode::Keycode6 | Keycode::Numpad6 => "6",
        Keycode::Keycode7 | Keycode::Numpad7 => "7",
        Keycode::Keycode8 | Keycode::Numpad8 => "8",
        Keycode::Keycode9 | Keycode::Numpad9 => "9",
        Keycode::DpadUp => "ArrowUp",
        Keycode::DpadDown => "ArrowDown",
        Keycode::DpadLeft => "ArrowLeft",
        Keycode::DpadRight => "ArrowRight",
        Keycode::A => "A",
        Keycode::B => "B",
        Keycode::C => "C",
        Keycode::D => "D",
        Keycode::E => "E",
        Keycode::F => "F",
        Keycode::G => "G",
        Keycode::H => "H",
        Keycode::I => "I",
        Keycode::J => "J",
        Keycode::K => "K",
        Keycode::L => "L",
        Keycode::M => "M",
        Keycode::N => "N",
        Keycode::O => "O",
        Keycode::P => "P",
        Keycode::Q => "Q",
        Keycode::R => "R",
        Keycode::S => "S",
        Keycode::T => "T",
        Keycode::U => "U",
        Keycode::V => "V",
        Keycode::W => "W",
        Keycode::X => "X",
        Keycode::Y => "Y",
        Keycode::Z => "Z",
        Keycode::Comma | Keycode::NumpadComma => ",",
        Keycode::Period | Keycode::NumpadDot => ".",
        Keycode::AltLeft => "AltLeft",
        Keycode::AltRight => "AltRight",
        Keycode::ShiftLeft => "ShiftLeft",
        Keycode::ShiftRight => "ShiftRight",
        Keycode::Tab => "Tab",
        Keycode::Space => "Space",
        Keycode::Enter | Keycode::NumpadEnter => "Enter",
        Keycode::Del => "Del",
        Keycode::Grave => "`",
        Keycode::Minus | Keycode::NumpadSubtract => "-",
        Keycode::Equals | Keycode::NumpadEquals => "=",
        Keycode::LeftBracket => "[",
        Keycode::RightBracket => "]",
        Keycode::Backslash => "\\",
        Keycode::Semicolon => ";",
        Keycode::Apostrophe => "\'",
        Keycode::Slash | Keycode::NumpadDivide => "/",
        Keycode::PageUp => "PgDn",
        Keycode::PageDown => "PgUp",
        Keycode::Escape => "Escape",
        Keycode::ForwardDel => "Del",
        Keycode::CtrlLeft => "CtrlLeft",
        Keycode::CtrlRight => "CtrlRight",
        Keycode::CapsLock => "CapsLock",
        Keycode::ScrollLock => "ScrollLock",
        Keycode::MetaLeft => "MetaLeft",
        Keycode::MetaRight => "MetaRight",
        Keycode::Sysrq => "PrtSc",
        Keycode::Break => "Pause",
        Keycode::MoveHome => "Home",
        Keycode::MoveEnd => "End",
        Keycode::Insert => "Insert",
        Keycode::F1 => "F1",
        Keycode::F2 => "F2",
        Keycode::F3 => "F3",
        Keycode::F4 => "F4",
        Keycode::F5 => "F5",
        Keycode::F6 => "F6",
        Keycode::F7 => "F7",
        Keycode::F8 => "F8",
        Keycode::F9 => "F9",
        Keycode::F10 => "F10",
        Keycode::F11 => "F11",
        Keycode::F12 => "F12",
        Keycode::NumLock => "NumLock",
        Keycode::NumpadMultiply => "*",
        Keycode::NumpadAdd => "+",
        Keycode::MediaPlay => "MediaPlay",
        Keycode::MediaPause => "MediaPause",
        _ => "Unknow",
    }
}

fn keycode_as_descriptor(keycode: &Keycode) -> KeyDescriptor {
    match keycode {
        Keycode::Home => KeyDescriptor {
            physical_key: PhysicalKey::Home,
            logical_key: LogicalKey::Named(NamedKey::Home),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode0 => KeyDescriptor {
            physical_key: PhysicalKey::Digit0,
            logical_key: LogicalKey::Character('0'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode1 => KeyDescriptor {
            physical_key: PhysicalKey::Digit1,
            logical_key: LogicalKey::Character('1'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode2 => KeyDescriptor {
            physical_key: PhysicalKey::Digit2,
            logical_key: LogicalKey::Character('2'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode3 => KeyDescriptor {
            physical_key: PhysicalKey::Digit3,
            logical_key: LogicalKey::Character('3'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode4 => KeyDescriptor {
            physical_key: PhysicalKey::Digit4,
            logical_key: LogicalKey::Character('4'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode5 => KeyDescriptor {
            physical_key: PhysicalKey::Digit5,
            logical_key: LogicalKey::Character('5'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode6 => KeyDescriptor {
            physical_key: PhysicalKey::Digit6,
            logical_key: LogicalKey::Character('6'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode7 => KeyDescriptor {
            physical_key: PhysicalKey::Digit7,
            logical_key: LogicalKey::Character('7'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode8 => KeyDescriptor {
            physical_key: PhysicalKey::Digit8,
            logical_key: LogicalKey::Character('8'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Keycode9 => KeyDescriptor {
            physical_key: PhysicalKey::Digit9,
            logical_key: LogicalKey::Character('9'),
            key_location: KeyLocation::Standard,
        },
        Keycode::DpadUp => KeyDescriptor {
            physical_key: PhysicalKey::ArrowUp,
            logical_key: LogicalKey::Named(NamedKey::ArrowUp),
            key_location: KeyLocation::Standard,
        },
        Keycode::DpadDown => KeyDescriptor {
            physical_key: PhysicalKey::ArrowDown,
            logical_key: LogicalKey::Named(NamedKey::ArrowDown),
            key_location: KeyLocation::Standard,
        },
        Keycode::DpadLeft => KeyDescriptor {
            physical_key: PhysicalKey::ArrowLeft,
            logical_key: LogicalKey::Named(NamedKey::ArrowLeft),
            key_location: KeyLocation::Standard,
        },
        Keycode::DpadRight => KeyDescriptor {
            physical_key: PhysicalKey::ArrowRight,
            logical_key: LogicalKey::Named(NamedKey::ArrowRight),
            key_location: KeyLocation::Standard,
        },
        Keycode::A => KeyDescriptor {
            physical_key: PhysicalKey::KeyA,
            logical_key: LogicalKey::Character('a'),
            key_location: KeyLocation::Standard,
        },
        Keycode::B => KeyDescriptor {
            physical_key: PhysicalKey::KeyB,
            logical_key: LogicalKey::Character('b'),
            key_location: KeyLocation::Standard,
        },
        Keycode::C => KeyDescriptor {
            physical_key: PhysicalKey::KeyC,
            logical_key: LogicalKey::Character('c'),
            key_location: KeyLocation::Standard,
        },
        Keycode::D => KeyDescriptor {
            physical_key: PhysicalKey::KeyD,
            logical_key: LogicalKey::Character('d'),
            key_location: KeyLocation::Standard,
        },
        Keycode::E => KeyDescriptor {
            physical_key: PhysicalKey::KeyE,
            logical_key: LogicalKey::Character('e'),
            key_location: KeyLocation::Standard,
        },
        Keycode::F => KeyDescriptor {
            physical_key: PhysicalKey::KeyF,
            logical_key: LogicalKey::Character('f'),
            key_location: KeyLocation::Standard,
        },
        Keycode::G => KeyDescriptor {
            physical_key: PhysicalKey::KeyG,
            logical_key: LogicalKey::Character('g'),
            key_location: KeyLocation::Standard,
        },
        Keycode::H => KeyDescriptor {
            physical_key: PhysicalKey::KeyH,
            logical_key: LogicalKey::Character('h'),
            key_location: KeyLocation::Standard,
        },
        Keycode::I => KeyDescriptor {
            physical_key: PhysicalKey::KeyI,
            logical_key: LogicalKey::Character('i'),
            key_location: KeyLocation::Standard,
        },
        Keycode::J => KeyDescriptor {
            physical_key: PhysicalKey::KeyJ,
            logical_key: LogicalKey::Character('j'),
            key_location: KeyLocation::Standard,
        },
        Keycode::K => KeyDescriptor {
            physical_key: PhysicalKey::KeyK,
            logical_key: LogicalKey::Character('k'),
            key_location: KeyLocation::Standard,
        },
        Keycode::L => KeyDescriptor {
            physical_key: PhysicalKey::KeyL,
            logical_key: LogicalKey::Character('l'),
            key_location: KeyLocation::Standard,
        },
        Keycode::M => KeyDescriptor {
            physical_key: PhysicalKey::KeyM,
            logical_key: LogicalKey::Character('m'),
            key_location: KeyLocation::Standard,
        },
        Keycode::N => KeyDescriptor {
            physical_key: PhysicalKey::KeyN,
            logical_key: LogicalKey::Character('n'),
            key_location: KeyLocation::Standard,
        },
        Keycode::O => KeyDescriptor {
            physical_key: PhysicalKey::KeyO,
            logical_key: LogicalKey::Character('o'),
            key_location: KeyLocation::Standard,
        },
        Keycode::P => KeyDescriptor {
            physical_key: PhysicalKey::KeyP,
            logical_key: LogicalKey::Character('p'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Q => KeyDescriptor {
            physical_key: PhysicalKey::KeyQ,
            logical_key: LogicalKey::Character('q'),
            key_location: KeyLocation::Standard,
        },
        Keycode::R => KeyDescriptor {
            physical_key: PhysicalKey::KeyR,
            logical_key: LogicalKey::Character('r'),
            key_location: KeyLocation::Standard,
        },
        Keycode::S => KeyDescriptor {
            physical_key: PhysicalKey::KeyS,
            logical_key: LogicalKey::Character('s'),
            key_location: KeyLocation::Standard,
        },
        Keycode::T => KeyDescriptor {
            physical_key: PhysicalKey::KeyT,
            logical_key: LogicalKey::Character('t'),
            key_location: KeyLocation::Standard,
        },
        Keycode::U => KeyDescriptor {
            physical_key: PhysicalKey::KeyU,
            logical_key: LogicalKey::Character('u'),
            key_location: KeyLocation::Standard,
        },
        Keycode::V => KeyDescriptor {
            physical_key: PhysicalKey::KeyV,
            logical_key: LogicalKey::Character('v'),
            key_location: KeyLocation::Standard,
        },
        Keycode::W => KeyDescriptor {
            physical_key: PhysicalKey::KeyW,
            logical_key: LogicalKey::Character('w'),
            key_location: KeyLocation::Standard,
        },
        Keycode::X => KeyDescriptor {
            physical_key: PhysicalKey::KeyX,
            logical_key: LogicalKey::Character('x'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Y => KeyDescriptor {
            physical_key: PhysicalKey::KeyY,
            logical_key: LogicalKey::Character('y'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Z => KeyDescriptor {
            physical_key: PhysicalKey::KeyZ,
            logical_key: LogicalKey::Character('z'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Comma => KeyDescriptor {
            physical_key: PhysicalKey::Comma,
            logical_key: LogicalKey::Character(','),
            key_location: KeyLocation::Standard,
        },
        Keycode::Period => KeyDescriptor {
            physical_key: PhysicalKey::Period,
            logical_key: LogicalKey::Character('.'),
            key_location: KeyLocation::Standard,
        },
        Keycode::AltLeft => KeyDescriptor {
            physical_key: PhysicalKey::AltLeft,
            logical_key: LogicalKey::Named(NamedKey::Alt),
            key_location: KeyLocation::Left,
        },
        Keycode::AltRight => KeyDescriptor {
            physical_key: PhysicalKey::AltRight,
            logical_key: LogicalKey::Named(NamedKey::Alt),
            key_location: KeyLocation::Right,
        },
        Keycode::ShiftLeft => KeyDescriptor {
            physical_key: PhysicalKey::ShiftLeft,
            logical_key: LogicalKey::Named(NamedKey::Shift),
            key_location: KeyLocation::Left,
        },
        Keycode::ShiftRight => KeyDescriptor {
            physical_key: PhysicalKey::ShiftRight,
            logical_key: LogicalKey::Named(NamedKey::Shift),
            key_location: KeyLocation::Right,
        },
        Keycode::Tab => KeyDescriptor {
            physical_key: PhysicalKey::Tab,
            logical_key: LogicalKey::Named(NamedKey::Tab),
            key_location: KeyLocation::Standard,
        },
        Keycode::Space => KeyDescriptor {
            physical_key: PhysicalKey::Space,
            logical_key: LogicalKey::Character(' '),
            key_location: KeyLocation::Standard,
        },
        Keycode::Enter => KeyDescriptor {
            physical_key: PhysicalKey::Enter,
            logical_key: LogicalKey::Named(NamedKey::Enter),
            key_location: KeyLocation::Standard,
        },
        Keycode::Del => KeyDescriptor {
            physical_key: PhysicalKey::Delete,
            logical_key: LogicalKey::Named(NamedKey::Delete),
            key_location: KeyLocation::Standard,
        },
        Keycode::Grave => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Character('`'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Minus => KeyDescriptor {
            physical_key: PhysicalKey::Minus,
            logical_key: LogicalKey::Character('-'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Equals => KeyDescriptor {
            physical_key: PhysicalKey::Equal,
            logical_key: LogicalKey::Character('='),
            key_location: KeyLocation::Standard,
        },
        Keycode::LeftBracket => KeyDescriptor {
            physical_key: PhysicalKey::BracketLeft,
            logical_key: LogicalKey::Character('['),
            key_location: KeyLocation::Standard,
        },
        Keycode::RightBracket => KeyDescriptor {
            physical_key: PhysicalKey::BracketRight,
            logical_key: LogicalKey::Character(']'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Backslash => KeyDescriptor {
            physical_key: PhysicalKey::Backslash,
            logical_key: LogicalKey::Character('\\'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Semicolon => KeyDescriptor {
            physical_key: PhysicalKey::Semicolon,
            logical_key: LogicalKey::Character(';'),
            key_location: KeyLocation::Standard,
        },
        Keycode::Apostrophe => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Character('\''),
            key_location: KeyLocation::Standard,
        },
        Keycode::Slash => KeyDescriptor {
            physical_key: PhysicalKey::Slash,
            logical_key: LogicalKey::Character('/'),
            key_location: KeyLocation::Standard,
        },
        Keycode::PageUp => KeyDescriptor {
            physical_key: PhysicalKey::PageUp,
            logical_key: LogicalKey::Named(NamedKey::PageUp),
            key_location: KeyLocation::Standard,
        },
        Keycode::PageDown => KeyDescriptor {
            physical_key: PhysicalKey::PageDown,
            logical_key: LogicalKey::Named(NamedKey::PageDown),
            key_location: KeyLocation::Standard,
        },
        Keycode::Escape => KeyDescriptor {
            physical_key: PhysicalKey::Escape,
            logical_key: LogicalKey::Named(NamedKey::Escape),
            key_location: KeyLocation::Standard,
        },
        Keycode::ForwardDel => KeyDescriptor {
            physical_key: PhysicalKey::Delete,
            logical_key: LogicalKey::Named(NamedKey::Delete),
            key_location: KeyLocation::Standard,
        },
        Keycode::CtrlLeft => KeyDescriptor {
            physical_key: PhysicalKey::ControlLeft,
            logical_key: LogicalKey::Named(NamedKey::Control),
            key_location: KeyLocation::Left,
        },
        Keycode::CtrlRight => KeyDescriptor {
            physical_key: PhysicalKey::ControlRight,
            logical_key: LogicalKey::Named(NamedKey::Control),
            key_location: KeyLocation::Right,
        },
        Keycode::CapsLock => KeyDescriptor {
            physical_key: PhysicalKey::CapsLock,
            logical_key: LogicalKey::Named(NamedKey::CapsLock),
            key_location: KeyLocation::Standard,
        },
        Keycode::ScrollLock => KeyDescriptor {
            physical_key: PhysicalKey::ScrollLock,
            logical_key: LogicalKey::Named(NamedKey::ScrollLock),
            key_location: KeyLocation::Standard,
        },
        Keycode::MetaLeft => KeyDescriptor {
            physical_key: PhysicalKey::ContextMenu,
            logical_key: LogicalKey::Named(NamedKey::ContextMenu),
            key_location: KeyLocation::Left,
        },
        Keycode::MetaRight => KeyDescriptor {
            physical_key: PhysicalKey::ContextMenu,
            logical_key: LogicalKey::Named(NamedKey::ContextMenu),
            key_location: KeyLocation::Right,
        },
        Keycode::Sysrq => KeyDescriptor {
            physical_key: PhysicalKey::PrintScreen,
            logical_key: LogicalKey::Named(NamedKey::PrintScreen),
            key_location: KeyLocation::Standard,
        },
        Keycode::Break => KeyDescriptor {
            physical_key: PhysicalKey::Pause,
            logical_key: LogicalKey::Named(NamedKey::Pause),
            key_location: KeyLocation::Standard,
        },
        Keycode::MoveHome => KeyDescriptor {
            physical_key: PhysicalKey::Home,
            logical_key: LogicalKey::Named(NamedKey::Home),
            key_location: KeyLocation::Standard,
        },
        Keycode::MoveEnd => KeyDescriptor {
            physical_key: PhysicalKey::End,
            logical_key: LogicalKey::Named(NamedKey::End),
            key_location: KeyLocation::Standard,
        },
        Keycode::Insert => KeyDescriptor {
            physical_key: PhysicalKey::Insert,
            logical_key: LogicalKey::Named(NamedKey::Insert),
            key_location: KeyLocation::Standard,
        },
        Keycode::F1 => KeyDescriptor {
            physical_key: PhysicalKey::F1,
            logical_key: LogicalKey::Named(NamedKey::F1),
            key_location: KeyLocation::Standard,
        },
        Keycode::F2 => KeyDescriptor {
            physical_key: PhysicalKey::F2,
            logical_key: LogicalKey::Named(NamedKey::F2),
            key_location: KeyLocation::Standard,
        },
        Keycode::F3 => KeyDescriptor {
            physical_key: PhysicalKey::F3,
            logical_key: LogicalKey::Named(NamedKey::F3),
            key_location: KeyLocation::Standard,
        },
        Keycode::F4 => KeyDescriptor {
            physical_key: PhysicalKey::F4,
            logical_key: LogicalKey::Named(NamedKey::F4),
            key_location: KeyLocation::Standard,
        },
        Keycode::F5 => KeyDescriptor {
            physical_key: PhysicalKey::F5,
            logical_key: LogicalKey::Named(NamedKey::F5),
            key_location: KeyLocation::Standard,
        },
        Keycode::F6 => KeyDescriptor {
            physical_key: PhysicalKey::F6,
            logical_key: LogicalKey::Named(NamedKey::F6),
            key_location: KeyLocation::Standard,
        },
        Keycode::F7 => KeyDescriptor {
            physical_key: PhysicalKey::F7,
            logical_key: LogicalKey::Named(NamedKey::F7),
            key_location: KeyLocation::Standard,
        },
        Keycode::F8 => KeyDescriptor {
            physical_key: PhysicalKey::F8,
            logical_key: LogicalKey::Named(NamedKey::F8),
            key_location: KeyLocation::Standard,
        },
        Keycode::F9 => KeyDescriptor {
            physical_key: PhysicalKey::F9,
            logical_key: LogicalKey::Named(NamedKey::F9),
            key_location: KeyLocation::Standard,
        },
        Keycode::F10 => KeyDescriptor {
            physical_key: PhysicalKey::F10,
            logical_key: LogicalKey::Named(NamedKey::F10),
            key_location: KeyLocation::Standard,
        },
        Keycode::F11 => KeyDescriptor {
            physical_key: PhysicalKey::F11,
            logical_key: LogicalKey::Named(NamedKey::F11),
            key_location: KeyLocation::Standard,
        },
        Keycode::F12 => KeyDescriptor {
            physical_key: PhysicalKey::F12,
            logical_key: LogicalKey::Named(NamedKey::F12),
            key_location: KeyLocation::Standard,
        },
        Keycode::NumLock => KeyDescriptor {
            physical_key: PhysicalKey::NumLock,
            logical_key: LogicalKey::Named(NamedKey::NumLock),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad0 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad0,
            logical_key: LogicalKey::Character('0'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad1 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad1,
            logical_key: LogicalKey::Character('1'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad2 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad2,
            logical_key: LogicalKey::Character('2'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad3 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad3,
            logical_key: LogicalKey::Character('3'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad4 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad4,
            logical_key: LogicalKey::Character('4'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad5 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad5,
            logical_key: LogicalKey::Character('5'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad6 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad6,
            logical_key: LogicalKey::Character('6'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad7 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad7,
            logical_key: LogicalKey::Character('7'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad8 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad8,
            logical_key: LogicalKey::Character('8'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::Numpad9 => KeyDescriptor {
            physical_key: PhysicalKey::Numpad9,
            logical_key: LogicalKey::Character('9'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadDivide => KeyDescriptor {
            physical_key: PhysicalKey::NumpadDivide,
            logical_key: LogicalKey::Character('/'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadMultiply => KeyDescriptor {
            physical_key: PhysicalKey::NumpadMultiply,
            logical_key: LogicalKey::Character('*'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadSubtract => KeyDescriptor {
            physical_key: PhysicalKey::NumpadSubtract,
            logical_key: LogicalKey::Character('-'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadAdd => KeyDescriptor {
            physical_key: PhysicalKey::NumpadAdd,
            logical_key: LogicalKey::Character('+'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadDot => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Character('.'),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadComma => KeyDescriptor {
            physical_key: PhysicalKey::NumpadComma,
            logical_key: LogicalKey::Character(','),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadEnter => KeyDescriptor {
            physical_key: PhysicalKey::NumpadEnter,
            logical_key: LogicalKey::Named(NamedKey::Enter),
            key_location: KeyLocation::Numpad,
        },
        Keycode::NumpadEquals => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Character('='),
            key_location: KeyLocation::Numpad,
        },
        Keycode::MediaPlay => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Named(NamedKey::Play),
            key_location: KeyLocation::Standard,
        },
        Keycode::MediaPause => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Named(NamedKey::Pause),
            key_location: KeyLocation::Standard,
        },
        _ => KeyDescriptor {
            physical_key: PhysicalKey::Unknown,
            logical_key: LogicalKey::Unknown,
            key_location: KeyLocation::Standard,
        },
    }
}
