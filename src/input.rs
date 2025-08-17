use std::{collections::HashMap, sync::{atomic::{self, AtomicBool}, Mutex, MutexGuard}};

use ndk::event::Keycode;
use once_cell::sync::Lazy;
use ruffle_core::{events::{KeyDescriptor, KeyLocation, LogicalKey, MouseButton, NamedKey, PhysicalKey}, Player, PlayerEvent};

pub const RETRO_DEVICE_JOYPAD: i32              = 1;
pub const RETRO_DEVICE_POINTER: i32             = 6;

pub const RETRO_DEVICE_ID_JOYPAD_B: i32         = 0;
pub const RETRO_DEVICE_ID_JOYPAD_Y: i32         = 1;
pub const RETRO_DEVICE_ID_JOYPAD_SELECT: i32    = 2;
pub const RETRO_DEVICE_ID_JOYPAD_START: i32     = 3;
pub const RETRO_DEVICE_ID_JOYPAD_UP: i32        = 4;
pub const RETRO_DEVICE_ID_JOYPAD_DOWN: i32      = 5;
pub const RETRO_DEVICE_ID_JOYPAD_LEFT: i32      = 6;
pub const RETRO_DEVICE_ID_JOYPAD_RIGHT: i32     = 7;
pub const RETRO_DEVICE_ID_JOYPAD_A: i32         = 8;
pub const RETRO_DEVICE_ID_JOYPAD_X: i32         = 9;
pub const RETRO_DEVICE_ID_JOYPAD_L: i32         = 10;
pub const RETRO_DEVICE_ID_JOYPAD_R: i32         = 11;
// pub const RETRO_DEVICE_ID_JOYPAD_L2: i32        = 12;
// pub const RETRO_DEVICE_ID_JOYPAD_R2: i32        = 13;
// pub const RETRO_DEVICE_ID_JOYPAD_L3: i32        = 14;
// pub const RETRO_DEVICE_ID_JOYPAD_R3: i32        = 15;
pub const RETRO_DEVICE_ID_JOYPAD_MASK: i32      = 256;

pub const RETRO_DEVICE_ID_POINTER_X: i32        = 0;
pub const RETRO_DEVICE_ID_POINTER_Y: i32        = 1;
pub const RETRO_DEVICE_ID_POINTER_PRESSED: i32  = 2;

pub const KEY_UP: i32    = 0;
pub const KEY_DOWN: i32  = 1;

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    key: Keycode,
    state: i32
}

impl KeyEvent {
    pub fn new(key: Keycode, state: i32) -> Self {
        Self { 
            key, 
            state 
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PointerEvent {
    x: f64,
    y: f64,
    pressed: bool
}

impl PointerEvent {
    pub fn new(x: f64, y: f64, pressed: bool) -> Self {
        Self { 
            x,
            y,
            pressed
         }
    }
}

#[derive(Debug)]
struct KeyState {
    state: i32,
    descriptor: KeyDescriptor
}

impl KeyState {
    fn new(desc: KeyDescriptor) -> Self {
        Self { 
            state: KEY_UP, 
            descriptor: desc 
        }
    }
}

pub struct InputDispatcher;

static KEYBORAD: Lazy<HashMap<i32, KeyDescriptor>> = Lazy::new(|| InputDispatcher::keyboard());
static ROUTE_TABLE: Lazy<Mutex<HashMap<i32, KeyState>>> = Lazy::new(|| Mutex::new(InputDispatcher::route_table()));
static POINTER_PRESSED: AtomicBool = AtomicBool::new(false);

impl InputDispatcher {

    fn keyboard() -> HashMap<i32, KeyDescriptor> {
        let mut it = HashMap::new();
        let keys = [
            Keycode::F1, Keycode::F2, Keycode::F3, Keycode::F3, Keycode::F5, Keycode::F6,
            Keycode::F7, Keycode::F8, Keycode::F9, Keycode::F10, Keycode::F11, Keycode::F12,   
            Keycode::Keycode1, Keycode::Keycode2, Keycode::Keycode3, Keycode::Keycode4, 
            Keycode::Keycode5, Keycode::Keycode6, Keycode::Keycode7, Keycode::Keycode8, 
            Keycode::Keycode9, Keycode::Keycode0,
            Keycode::DpadLeft, Keycode::DpadRight, Keycode::DpadUp, Keycode::DpadDown,
            Keycode::Q, Keycode::W, Keycode::E, Keycode::R, Keycode::T, Keycode::Y, Keycode::U, 
            Keycode::I, Keycode::O, Keycode::P, Keycode::A, Keycode::S, Keycode::D, Keycode::F,
            Keycode::G, Keycode::H, Keycode::J, Keycode::K, Keycode::L, Keycode::Z, Keycode::X,
            Keycode::C, Keycode::V, Keycode::B, Keycode::N, Keycode::M,
            Keycode::ShiftLeft, Keycode::ShiftRight, Keycode::AltLeft, Keycode::AltRight,Keycode::CtrlLeft, 
            Keycode::LeftBracket, Keycode::RightBracket,
            Keycode::CtrlRight, Keycode::Tab, Keycode::Space, Keycode::Enter, Keycode::Back
        ];
        for key in keys {
            let physical = PhysicalKey::Unknown;
            let (logical, location) = match key {
                Keycode::Keycode0 => (LogicalKey::Character('0'), KeyLocation::Standard),
                Keycode::Keycode1 => (LogicalKey::Character('1'), KeyLocation::Standard),
                Keycode::Keycode2 => (LogicalKey::Character('2'), KeyLocation::Standard),
                Keycode::Keycode3 => (LogicalKey::Character('3'), KeyLocation::Standard),
                Keycode::Keycode4 => (LogicalKey::Character('4'), KeyLocation::Standard),
                Keycode::Keycode5 => (LogicalKey::Character('5'), KeyLocation::Standard),
                Keycode::Keycode6 => (LogicalKey::Character('6'), KeyLocation::Standard),
                Keycode::Keycode7 => (LogicalKey::Character('7'), KeyLocation::Standard),
                Keycode::Keycode8 => (LogicalKey::Character('8'), KeyLocation::Standard),
                Keycode::Keycode9 => (LogicalKey::Character('9'), KeyLocation::Standard),
                Keycode::A => (LogicalKey::Character('a'), KeyLocation::Standard),
                Keycode::B => (LogicalKey::Character('b'), KeyLocation::Standard),
                Keycode::C => (LogicalKey::Character('c'), KeyLocation::Standard),
                Keycode::D => (LogicalKey::Character('d'), KeyLocation::Standard),
                Keycode::E => (LogicalKey::Character('e'), KeyLocation::Standard),
                Keycode::F => (LogicalKey::Character('f'), KeyLocation::Standard),
                Keycode::G => (LogicalKey::Character('g'), KeyLocation::Standard),
                Keycode::H => (LogicalKey::Character('h'), KeyLocation::Standard),
                Keycode::I => (LogicalKey::Character('i'), KeyLocation::Standard),
                Keycode::J => (LogicalKey::Character('j'), KeyLocation::Standard),
                Keycode::K => (LogicalKey::Character('k'), KeyLocation::Standard),
                Keycode::L => (LogicalKey::Character('l'), KeyLocation::Standard),
                Keycode::M => (LogicalKey::Character('m'), KeyLocation::Standard),
                Keycode::N => (LogicalKey::Character('n'), KeyLocation::Standard),
                Keycode::O => (LogicalKey::Character('o'), KeyLocation::Standard),
                Keycode::P => (LogicalKey::Character('p'), KeyLocation::Standard),
                Keycode::Q => (LogicalKey::Character('q'), KeyLocation::Standard),
                Keycode::R => (LogicalKey::Character('r'), KeyLocation::Standard),
                Keycode::S => (LogicalKey::Character('s'), KeyLocation::Standard),
                Keycode::T => (LogicalKey::Character('t'), KeyLocation::Standard),
                Keycode::U => (LogicalKey::Character('u'), KeyLocation::Standard),
                Keycode::V => (LogicalKey::Character('v'), KeyLocation::Standard),
                Keycode::W => (LogicalKey::Character('w'), KeyLocation::Standard),
                Keycode::X => (LogicalKey::Character('x'), KeyLocation::Standard),
                Keycode::Y => (LogicalKey::Character('y'), KeyLocation::Standard),
                Keycode::Z => (LogicalKey::Character('z'), KeyLocation::Standard),
                Keycode::DpadLeft => (LogicalKey::Named(NamedKey::ArrowLeft), KeyLocation::Standard),
                Keycode::DpadRight => (LogicalKey::Named(NamedKey::ArrowRight), KeyLocation::Standard),
                Keycode::DpadUp => (LogicalKey::Named(NamedKey::ArrowUp), KeyLocation::Standard),
                Keycode::DpadDown => (LogicalKey::Named(NamedKey::ArrowDown), KeyLocation::Standard),
                Keycode::AltLeft => (LogicalKey::Named(NamedKey::Alt), KeyLocation::Left),
                Keycode::AltRight => (LogicalKey::Named(NamedKey::Alt), KeyLocation::Right),
                Keycode::ShiftLeft => (LogicalKey::Named(NamedKey::Shift), KeyLocation::Left),
                Keycode::ShiftRight => (LogicalKey::Named(NamedKey::Shift), KeyLocation::Right),
                Keycode::Tab => (LogicalKey::Named(NamedKey::Tab), KeyLocation::Standard),
                Keycode::Space => (LogicalKey::Character(' '), KeyLocation::Standard),
                Keycode::Enter => (LogicalKey::Named(NamedKey::Enter), KeyLocation::Standard),
                Keycode::Del => (LogicalKey::Named(NamedKey::Delete), KeyLocation::Standard),
                Keycode::LeftBracket => (LogicalKey::Character('['), KeyLocation::Standard),
                Keycode::RightBracket => (LogicalKey::Character(']'), KeyLocation::Standard),
                Keycode::Escape => (LogicalKey::Named(NamedKey::Escape), KeyLocation::Standard),
                Keycode::CtrlLeft => (LogicalKey::Named(NamedKey::Control), KeyLocation::Left),
                Keycode::CtrlRight => (LogicalKey::Named(NamedKey::Control), KeyLocation::Right),
                Keycode::F1 => (LogicalKey::Named(NamedKey::F1), KeyLocation::Standard),
                Keycode::F2 => (LogicalKey::Named(NamedKey::F2), KeyLocation::Standard),
                Keycode::F3 => (LogicalKey::Named(NamedKey::F3), KeyLocation::Standard),
                Keycode::F4 => (LogicalKey::Named(NamedKey::F4), KeyLocation::Standard),
                Keycode::F5 => (LogicalKey::Named(NamedKey::F5), KeyLocation::Standard),
                Keycode::F6 => (LogicalKey::Named(NamedKey::F6), KeyLocation::Standard),
                Keycode::F7 => (LogicalKey::Named(NamedKey::F7), KeyLocation::Standard),
                Keycode::F8 => (LogicalKey::Named(NamedKey::F8), KeyLocation::Standard),
                Keycode::F9 => (LogicalKey::Named(NamedKey::F9), KeyLocation::Standard),
                Keycode::F10 => (LogicalKey::Named(NamedKey::F10), KeyLocation::Standard),
                Keycode::F11 => (LogicalKey::Named(NamedKey::F11), KeyLocation::Standard),
                Keycode::F12 => (LogicalKey::Named(NamedKey::F12), KeyLocation::Standard),
                _ => continue,
            };
            it.insert(key.into(), 
            KeyDescriptor { 
                physical_key: physical, 
                logical_key: logical, 
                key_location: location 
            });
        }
        it
    }

    fn route_table() -> HashMap<i32, KeyState> {
        let mut it = HashMap::new();

        // Pad Left => ArrowLeft
        let mut key: i32 = Keycode::DpadLeft.into();
        let mut desc = Self::descriptor_from_keycode(Keycode::DpadLeft).unwrap();
        it.insert(key, KeyState::new(desc));

        // Pad Right => ArrowRight
        key = Keycode::DpadRight.into();
        desc = Self::descriptor_from_keycode(Keycode::DpadRight).unwrap();
        it.insert(key, KeyState::new(desc));

        // Pad Up => ArrowUp
        key = Keycode::DpadUp.into();
        desc = Self::descriptor_from_keycode(Keycode::DpadUp).unwrap();
        it.insert(key, KeyState::new(desc));

        // Pad Down => ArrowDown
        key = Keycode::DpadDown.into();
        desc = Self::descriptor_from_keycode(Keycode::DpadDown).unwrap();
        it.insert(key, KeyState::new(desc));
        
        // Button A => A
        key = Keycode::ButtonA.into();
        desc = Self::descriptor_from_keycode(Keycode::A).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button B => B
        key = Keycode::ButtonB.into();
        desc = Self::descriptor_from_keycode(Keycode::B).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button X => X
        key = Keycode::ButtonX.into();
        desc = Self::descriptor_from_keycode(Keycode::X).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button Y => Y
        key = Keycode::ButtonY.into();
        desc = Self::descriptor_from_keycode(Keycode::Y).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button Select => Tab
        key = Keycode::ButtonSelect.into();
        desc = Self::descriptor_from_keycode(Keycode::Tab).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button Start => Enter
        key = Keycode::ButtonStart.into();
        desc = Self::descriptor_from_keycode(Keycode::Enter).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button L1 => F1
        key = Keycode::ButtonL1.into();
        desc = Self::descriptor_from_keycode(Keycode::F1).unwrap();
        it.insert(key, KeyState::new(desc));

        // Button R1 => F2
        key = Keycode::ButtonR1.into();
        desc = Self::descriptor_from_keycode(Keycode::F2).unwrap();
        it.insert(key, KeyState::new(desc));

        it
    }

    fn descriptor_from_keycode(key: Keycode) -> Option<KeyDescriptor> {
        let c: i32 = key.into();
        KEYBORAD.get(&c)
            .copied()
    }

    pub fn dispacth_pointer_event<'a>(event: PointerEvent, player: &mut MutexGuard<'a, Player>) {
        if POINTER_PRESSED.load(atomic::Ordering::Relaxed) != event.pressed {
            if event.pressed {
                player.handle_event(
                    PlayerEvent::MouseDown { 
                        x: event.x, 
                        y: event.y, 
                        button: MouseButton::Left, 
                        index: None
                    }
                );
            } else {
                player.handle_event(
                    PlayerEvent::MouseUp { 
                        x: event.x, 
                        y: event.y, 
                        button: MouseButton::Left, 
                    }
                );
            }
            POINTER_PRESSED.store(event.pressed, atomic::Ordering::Relaxed);
        } else if event.pressed {
            player.handle_event(
                    PlayerEvent::MouseMove { 
                        x: event.x, 
                        y: event.y,
                    }
                );
        }
    }

    pub fn dispacth_key_event<'a>(event: KeyEvent, player: &mut MutexGuard<'a, Player>) {
        let key: i32 = event.key.into();
        if let Some(key_state) = ROUTE_TABLE.lock().unwrap().get_mut(&key) {
            if key_state.state != event.state {
                if event.state == KEY_DOWN {
                    player.handle_event(
                        PlayerEvent::KeyDown { 
                            key: key_state.descriptor 
                        }
                    );
                } else {
                    player.handle_event(
                        PlayerEvent::KeyUp { 
                            key: key_state.descriptor 
                        }
                    );
                }
                key_state.state = event.state;
            }   
        }
    }
}