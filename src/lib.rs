mod utils;
use std::{
    cell::OnceCell,
    os::raw::c_void,
    ptr::NonNull,
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use jni::{
    JNIEnv,
    objects::{JObject, JString},
    sys::{JNI_FALSE, JNI_TRUE, JNI_VERSION_1_6},
};
use log::{debug, error, info, warn};
use ndk::native_window::NativeWindow;
use ndk_sys::{ANativeWindow, ANativeWindow_fromSurface};
use ruffle_core::{
    Player, PlayerBuilder, ViewportDimensions, backend::log::LogBackend, tag_utils::SwfMovie,
};
use ruffle_render::backend::RenderBackend;
use ruffle_render_wgpu::{
    backend::WgpuRenderBackend,
    target::TextureTarget,
    wgpu::{
        Backends, PowerPreference, SurfaceTargetUnsafe,
        rwh::{AndroidDisplayHandle, HasWindowHandle, RawDisplayHandle},
    },
};
use ruffle_video_software::backend::SoftwareVideoBackend;

use crate::utils::JniUtils;

type JBoolean = u8;
type JInt = i32;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[derive(PartialEq)]
enum State {
    IDLE,
    RUNNING,
    PAUSED,
}

struct PrintWriter;

impl PrintWriter {
    fn new() -> Self {
        PrintWriter {}
    }
}

impl LogBackend for PrintWriter {
    fn avm_trace(&self, message: &str) {
        error!("{message}")
    }

    fn avm_warning(&self, message: &str) {
        warn!("{message}")
    }
}

enum RuffleEvent {
    AttachSurface(NativeWindow),
    AdjustSurfaceSize(i32, i32),
    Kill,
}

static TX: Mutex<Option<Sender<RuffleEvent>>> = Mutex::new(None);
static STATE: RwLock<State> = RwLock::new(State::IDLE);
static THREAD_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

pub extern "C" fn em_attach_surface(mut env: JNIEnv, sf: JObject) {
    if let Ok(tx_grant) = TX.lock() {
        if let Some(tx) = &*tx_grant {
            unsafe {
                let window_ptr = ANativeWindow_fromSurface(env.get_raw(), sf.into_raw());
                let window = NativeWindow::from_ptr(NonNull::new(window_ptr).unwrap());
                tx.send(RuffleEvent::AttachSurface(window));
            };
        }
    };
}

pub extern "C" fn em_adjust_surface_size(mut env: JNIEnv, vw: JInt, vh: JInt) {
    if let Ok(tx_grant) = TX.lock() {
        if let Some(tx) = &*tx_grant {
            tx.send(RuffleEvent::AdjustSurfaceSize(vw, vh));
        }
    };
}

pub extern "C" fn em_stop(mut env: JNIEnv) {
    if let Ok(tx_guard) = TX.lock() {
        if let Some(tx) = tx_guard.as_ref() {
            tx.send(RuffleEvent::Kill);
        }
    };
    info!("Waiting main thread to exit...");
    let mut handle_guard = THREAD_HANDLE.lock().unwrap();
    if let Some(handle) = handle_guard.take() {
        handle.join();
    }
}

pub extern "C" fn em_start(mut env: JNIEnv, path: JString) -> JBoolean {
    let filepath = JniUtils::to_string(&mut env, path);
    let (tx, rx) = mpsc::channel::<RuffleEvent>();
    *TX.lock().unwrap() = Some(tx);
    let handle = thread::spawn(move || {
        let mut player_ref: Option<Arc<Mutex<Player>>> = None;
        let mut prev_frame_time = Instant::now();
        loop {
            match rx.try_recv() {
                Ok(event) => match event {
                    RuffleEvent::AttachSurface(window) => {
                        let movie =
                            SwfMovie::from_path(&filepath, None).expect("Failed to load swf file.");
                        unsafe {
                            let renderer = WgpuRenderBackend::for_window_unsafe(
                                SurfaceTargetUnsafe::RawHandle {
                                    raw_display_handle: RawDisplayHandle::Android(
                                        AndroidDisplayHandle::new(),
                                    ),
                                    raw_window_handle: window.window_handle().unwrap().into(),
                                },
                                (window.width() as u32, window.height() as u32),
                                Backends::GL,
                                PowerPreference::HighPerformance,
                            )
                            .unwrap();
                            player_ref = Some(
                                PlayerBuilder::new()
                                    .with_renderer(renderer)
                                    .with_movie(movie)
                                    .with_log(PrintWriter::new())
                                    .with_fullscreen(true)
                                    .build(),
                            );
                        };
                    }
                    RuffleEvent::AdjustSurfaceSize(vw, vh) => {
                        if let Some(player) = &player_ref {
                            player
                                .lock()
                                .unwrap()
                                .set_viewport_dimensions(ViewportDimensions {
                                    width: vw as u32,
                                    height: vh as u32,
                                    scale_factor: 1.0,
                                });
                        }
                    }
                    RuffleEvent::Kill => break,
                },
                Err(e) => {
                    dbg!(e);
                }
            }
            if let Some(player_lock) = &player_ref {
                let mut player = player_lock.lock().unwrap();
                let dt = prev_frame_time.duration_since(Instant::now()).as_micros();
                if dt > 0 && *STATE.read().unwrap() == State::RUNNING {
                    player.tick(dt as f64 / 1000.0);
                    prev_frame_time = Instant::now();

                    if player.needs_render() {
                        player.render();
                    }
                }
            }
        }
    });
    *THREAD_HANDLE.lock().unwrap() = Some(handle);
    JNI_TRUE
}

pub extern "C" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> JInt {
    android_logger::init_once(android_logger::Config::default().with_tag("libruffle"));
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
