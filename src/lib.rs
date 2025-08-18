mod media;
mod input;
mod util;
use std::{
    any::Any,
    os::raw::c_void,
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use jni::{
    JNIEnv, JavaVM, NativeMethod,
    objects::{JObject, JString, JValue},
    sys::{JNI_TRUE, JNI_VERSION_1_6, jboolean, jint},
};
use log::{LevelFilter, error, info};
use ndk::{event::Keycode, native_window::NativeWindow};
use ndk_sys::ANativeWindow_fromSurface;
use ruffle_core::{
    backend::log::NullLogBackend, config::Letterbox, tag_utils::SwfMovie, Player, PlayerBuilder, ViewportDimensions
};
use ruffle_render_wgpu::{
    backend::WgpuRenderBackend,
    target::SwapChainTarget,
    wgpu::{
        Backends, PowerPreference, SurfaceTargetUnsafe,
        rwh::{AndroidDisplayHandle, HasWindowHandle, RawDisplayHandle},
    },
};

use crate::{
    input::{
        InputDispatcher, KeyAction, KeyEvent, PointerEvent, RETRO_DEVICE_ID_JOYPAD_A, RETRO_DEVICE_ID_JOYPAD_B, RETRO_DEVICE_ID_JOYPAD_DOWN, RETRO_DEVICE_ID_JOYPAD_L, RETRO_DEVICE_ID_JOYPAD_LEFT, RETRO_DEVICE_ID_JOYPAD_MASK, RETRO_DEVICE_ID_JOYPAD_R, RETRO_DEVICE_ID_JOYPAD_RIGHT, RETRO_DEVICE_ID_JOYPAD_SELECT, RETRO_DEVICE_ID_JOYPAD_START, RETRO_DEVICE_ID_JOYPAD_UP, RETRO_DEVICE_ID_JOYPAD_X, RETRO_DEVICE_ID_JOYPAD_Y, RETRO_DEVICE_ID_POINTER_PRESSED, RETRO_DEVICE_ID_POINTER_X, RETRO_DEVICE_ID_POINTER_Y, RETRO_DEVICE_JOYPAD, RETRO_DEVICE_POINTER
    }, media::AAudioAudioBackend, util::{JniUtils, Properties}
};

enum RuffleEvent {
    AttachSurface(NativeWindow),
    AdjustSurfaceSize(i32, i32),
    DetachSurface,
    HandleKeyEvent(KeyEvent),
    Kill,
}

const PROP_DISPLAY_SCALED_DENSITY: &str = "ruffle_scale_factor";

static TX: Mutex<Option<Sender<RuffleEvent>>> = Mutex::new(None);
static RX: Mutex<Option<Receiver<RuffleEvent>>> = Mutex::new(None);
static THREAD_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

static PROPS: Mutex<Properties> = Mutex::new(Properties::new());

fn send_event(event: RuffleEvent) {
    TX.lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .send(event)
        .unwrap_or_else(|err| {
            error!("Event send failed. {err}");
        });
}

fn poll_event() -> Result<RuffleEvent, mpsc::TryRecvError> {
    RX.lock().unwrap().as_ref().unwrap().try_recv()
}

fn em_attach_surface(env: JNIEnv, _thiz: JObject, _activity: JObject, sf: JObject) {
    unsafe {
        let window_ptr = ANativeWindow_fromSurface(env.get_raw(), sf.into_raw());
        let window = NativeWindow::from_ptr(NonNull::new(window_ptr).unwrap());
        send_event(RuffleEvent::AttachSurface(window));
    };
}

fn em_adjust_surface_size(_env: JNIEnv, _thiz: JObject, vw: jint, vh: jint) {
    send_event(RuffleEvent::AdjustSurfaceSize(vw, vh));
}

fn em_detach_surface(_env: JNIEnv, _thiz: JObject) {
    send_event(RuffleEvent::DetachSurface);
}

fn em_stop(_env: JNIEnv, _thiz: JObject) {
    send_event(RuffleEvent::Kill);
    info!("Waiting main thread to exit...");
    let mut handle_guard = THREAD_HANDLE.lock().unwrap();
    if let Some(handle) = handle_guard.take() {
        let _ = handle.join();
    }
}

fn em_start(mut env: JNIEnv, thiz: JObject, path: JString) -> jboolean {
    let filepath = JniUtils::to_string(&mut env, path);
    let vm = env.get_java_vm().unwrap();
    let s_thiz = env
        .new_global_ref(thiz)
        .expect("Failed to global thiz ref!");
    let handle = thread::spawn(move || {
        let mut player_ref: Option<Arc<Mutex<Player>>> = None;
        let mut prev_frame_time = Instant::now();
        let mut s_env = vm
            .attach_current_thread()
            .expect("Failed to attach env thread");
        let dpi_scale_factor = PROPS
            .lock()
            .unwrap()
            .get_float(PROP_DISPLAY_SCALED_DENSITY, 1.0) as f64;

        loop {
            match poll_event() {
                Ok(event) => match event {
                    RuffleEvent::AttachSurface(window) => unsafe {
                        let vw: u32 = window.width() as u32;
                        let vh: u32 = window.height() as u32;
                        if let Some(player_mtx) = &player_ref {
                            let mut player = player_mtx.lock().unwrap();
                            let renderer = <dyn Any>::downcast_mut::<
                                WgpuRenderBackend<SwapChainTarget>,
                            >(player.renderer_mut())
                            .unwrap();
                            let _ = renderer.recreate_surface_unsafe(
                                SurfaceTargetUnsafe::RawHandle {
                                    raw_display_handle: RawDisplayHandle::Android(
                                        AndroidDisplayHandle::new(),
                                    ),
                                    raw_window_handle: window.window_handle().unwrap().into(),
                                },
                                (vw, vh),
                            );
                            player.set_is_playing(true);
                        } else {
                            let movie = SwfMovie::from_path(&filepath, None).unwrap();
                            let renderer = WgpuRenderBackend::for_window_unsafe(
                                SurfaceTargetUnsafe::RawHandle {
                                    raw_display_handle: RawDisplayHandle::Android(
                                        AndroidDisplayHandle::new(),
                                    ),
                                    raw_window_handle: window.window_handle().unwrap().into(),
                                },
                                (vw, vh),
                                Backends::GL,
                                PowerPreference::HighPerformance,
                            )
                            .unwrap();
                            player_ref = Some(
                                PlayerBuilder::new()
                                    .with_renderer(renderer)
                                    .with_movie(movie)
                                    .with_audio(AAudioAudioBackend::new().unwrap())
                                    .with_log(NullLogBackend::new())
                                    .with_viewport_dimensions(
                                        vw,
                                        vh,
                                        dpi_scale_factor,
                                    )
                                    .with_letterbox(Letterbox::On)
                                    .build(),
                            );

                            if let Some(player_mtx) = &player_ref {
                                let mut player = player_mtx.lock()
                                    .unwrap();
                                player.set_is_playing(true);
                                let result = s_env.call_method(
                                &s_thiz, 
                                "onNativeVideoSizeChanged", 
                                "(III)V", 
                                &[JValue::from(player.movie_width() as i32), JValue::from(player.movie_height() as i32), JValue::from(0)]
                                );
                                if let Err(e) = result {
                                    error!("Failed to report movie size! {e}");
                                }
                            }
                        }
                    },
                    RuffleEvent::AdjustSurfaceSize(vw, vh) => {
                        if let Some(player_mtx) = &player_ref {
                            player_mtx
                            .lock()
                            .unwrap()
                            .set_viewport_dimensions(ViewportDimensions {
                                width: vw as u32,
                                height: vh as u32,
                                scale_factor: dpi_scale_factor,
                            });
                        }
                    }
                    RuffleEvent::DetachSurface => {
                        if let Some(player_mtx) = &player_ref {
                            player_mtx
                                .lock()
                                .unwrap()
                                .set_is_playing(false);
                        }
                    }
                    RuffleEvent::HandleKeyEvent(event) => {
                        if let Some(player_mtx) = &player_ref {
                            let mut player = player_mtx
                                .lock()
                                .unwrap();
                            InputDispatcher::dispacth_key_event(event, &mut player);
                        }
                    }
                    RuffleEvent::Kill => break,
                },
                Err(e) => {
                    dbg!(e);
                }
            }
            if let Some(player_mtx) = &player_ref {
                let mut player = player_mtx.lock().unwrap();
                let now = Instant::now();
                let dt = now.duration_since(prev_frame_time).as_micros();
                if dt > 0 {
                    prev_frame_time = now;
                    player.tick(dt as f64 / 1000.0);
                    if player.needs_render() {
                        player.render();
                    }
                    let audio =
                        <dyn Any>::downcast_mut::<AAudioAudioBackend>(player.audio_mut()).unwrap();
                    audio.keep_stream_valid();
                }

                for port in 0..1 {
                    let status = s_env
                        .call_method(
                            &s_thiz,
                            "onNativePollInput",
                            "(IIII)I",
                            &[
                                JValue::from(port),
                                JValue::from(RETRO_DEVICE_JOYPAD),
                                JValue::from(0),
                                JValue::from(RETRO_DEVICE_ID_JOYPAD_MASK),
                            ],
                        )
                        .expect("Failed to poll joypad status!")
                        .i()
                        .expect("Failed to poll joypad status!");

                    let mut action = status >> RETRO_DEVICE_ID_JOYPAD_A & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonA, KeyAction::from(action)), &mut player);

                    action = status >> RETRO_DEVICE_ID_JOYPAD_B & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonB, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_X & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonX, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_Y & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonY, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_LEFT & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::DpadLeft, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_RIGHT & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::DpadRight, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_UP & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::DpadUp, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_DOWN & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::DpadDown, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_SELECT & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonSelect, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_START & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonStart, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_L & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonL1, KeyAction::from(action)), &mut player);
                    action = status >> RETRO_DEVICE_ID_JOYPAD_R & 0x1;
                    InputDispatcher::dispacth_key_event(KeyEvent::gamepad(Keycode::ButtonR1, KeyAction::from(action)), &mut player);
                }
                let pressed = s_env
                    .call_method(
                        &s_thiz,
                        "onNativePollInput",
                        "(IIII)I",
                        &[
                            JValue::from(0),
                            JValue::from(RETRO_DEVICE_POINTER),
                            JValue::from(0),
                            JValue::from(RETRO_DEVICE_ID_POINTER_PRESSED),
                        ],
                    )
                    .expect("Failed to poll pointer state!")
                    .i()
                    .expect("Failed to poll pointer state!")
                    == 1;
                let pointer_x = s_env
                    .call_method(
                        &s_thiz,
                        "onNativePollInput",
                        "(IIII)I",
                        &[
                            JValue::from(0),
                            JValue::from(RETRO_DEVICE_POINTER),
                            JValue::from(0),
                            JValue::from(RETRO_DEVICE_ID_POINTER_X),
                        ],
                    )
                    .expect("Failed to poll pointer state!")
                    .i()
                    .expect("Failed to poll pointer state!");
                let pointer_y = s_env
                    .call_method(
                        &s_thiz,
                        "onNativePollInput",
                        "(IIII)I",
                        &[
                            JValue::from(0),
                            JValue::from(RETRO_DEVICE_POINTER),
                            JValue::from(0),
                            JValue::from(RETRO_DEVICE_ID_POINTER_Y),
                        ],
                    )
                    .expect("Failed to poll pointer state!")
                    .i()
                    .expect("Failed to poll pointer state!");
                let dimension = player.viewport_dimensions();
                let x = (pointer_x as f64 + 32767.0) * dimension.width as f64 / 65534.0;
                let y = (pointer_y as f64 + 32767.0) * dimension.height as f64 / 65534.0;
                InputDispatcher::dispacth_pointer_event(
                    PointerEvent::new(x, y, pressed), 
                    &mut player
                );
            }
        }
    });
    *THREAD_HANDLE.lock().unwrap() = Some(handle);
    JNI_TRUE
}

fn em_set_prop(mut env: JNIEnv, _thiz: JObject, k: JString, prop: JObject) {
    let key = JniUtils::to_string(&mut env, k);
    match key.as_str() {
        PROP_DISPLAY_SCALED_DENSITY => {
            let val = JniUtils::to_float(&mut env, prop);
            PROPS.lock().unwrap().set_prop(key.as_str(), val);
        }
        _ => (),
    }
}

fn em_dispatch_keyboard_event(mut env: JNIEnv, _thiz: JObject, event: JObject) -> jboolean {
    let key = env.call_method(&event, "getKeyCode", "()I", &[])
        .expect("Failed to call KeyEvent::getKeyCode() method!")
        .i()
        .expect("Failed to call KeyEvent::getKeyCode() method!");
    let action = env.call_method(&event, "getAction", "()I", &[])
        .expect("Failed to call KeyEvent::getAction() method!")
        .i()
        .expect("Failed to call KeyEvent::getAction() method!");
    error!("{key}, {action}");
    send_event(RuffleEvent::HandleKeyEvent(KeyEvent::new(Keycode::from(key), KeyAction::from(action))));
    JNI_TRUE
}

#[unsafe(no_mangle)]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, _reserved: *const c_void) -> jint {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(LevelFilter::Info)
            .with_tag("libruffle"),
    );
    let mut env = vm.get_env().unwrap();
    let methods = [
        NativeMethod {
            name: "nativeStart".into(),
            sig: "(Ljava/lang/String;)Z".into(),
            fn_ptr: em_start as *mut _,
        },
        NativeMethod {
            name: "nativeStop".into(),
            sig: "()V".into(),
            fn_ptr: em_stop as *mut _,
        },
        NativeMethod {
            name: "nativeAttachSurface".into(),
            sig: "(Landroid/app/Activity;Landroid/view/Surface;)V".into(),
            fn_ptr: em_attach_surface as *mut _,
        },
        NativeMethod {
            name: "nativeAdjustSurface".into(),
            sig: "(II)V".into(),
            fn_ptr: em_adjust_surface_size as *mut _,
        },
        NativeMethod {
            name: "nativeDetachSurface".into(),
            sig: "()V".into(),
            fn_ptr: em_detach_surface as *mut _,
        },
        NativeMethod {
            name: "nativeSetProp".into(),
            sig: "(Ljava/lang/String;Ljava/lang/Object;)V".into(),
            fn_ptr: em_set_prop as *mut _,
        },
        NativeMethod {
            name: "nativeDispatchKeyboardEvent".into(),
            sig: "(Landroid/view/KeyEvent;)Z".into(),
            fn_ptr: em_dispatch_keyboard_event as *mut _,
        }
    ];
    assert!(
        env.register_native_methods("org/wkuwku/plug/ruffle/Ruffle", &methods)
            .is_ok()
    );
    let (tx, rx) = mpsc::channel::<RuffleEvent>();
    *TX.lock().unwrap() = Some(tx);
    *RX.lock().unwrap() = Some(rx);
    JNI_VERSION_1_6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
