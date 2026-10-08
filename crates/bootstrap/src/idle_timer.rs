//! Keep the screen awake through a load and a match (iOS).
//!
//! iOS locks the screen after the Auto-Lock interval without a touch (a controller
//! may not reset it), and a lock suspends the process. A 30-60 s load with the
//! phone untouched can therefore stall until someone wakes it: the bake and the
//! loading screen both stop. Menus keep the system setting; every other platform
//! registers nothing.
use bevy::prelude::*;

pub(crate) fn register(app: &mut App) {
    #[cfg(target_os = "ios")]
    app.insert_non_send(ios::MainThread)
        .add_systems(Last, (ios::follow_screen, ios::log_lifecycle));
    #[cfg(not(target_os = "ios"))]
    let _ = app;
}

#[cfg(target_os = "ios")]
mod ios {
    use bevy::prelude::*;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    /// NonSend marker: UIKit is touched on the main thread only.
    pub(super) struct MainThread;

    /// The log lines carry no timestamps, so a stall cannot be told from a
    /// suspend. `lifecycle:` lines carry the process clock (`ns=`).
    pub(super) fn log_lifecycle(mut states: MessageReader<bevy::window::AppLifecycle>) {
        for state in states.read() {
            diag::lifecycle_boundary(&format!("app {state:?}"), "");
        }
    }

    pub(super) fn follow_screen(
        _main: NonSend<MainThread>,
        screen: Option<Res<frame::AppScreen>>,
        mut live: Local<Option<bool>>,
    ) {
        let awake = matches!(
            screen.as_deref(),
            Some(
                frame::AppScreen::Loading
                    | frame::AppScreen::ClassSelect
                    | frame::AppScreen::InGame
            )
        );
        if *live == Some(awake) {
            return;
        }
        *live = Some(awake);
        // SAFETY: plain UIKit messages with their documented signatures, on the
        // main thread (NonSend). `idleTimerDisabled` is a BOOL property.
        unsafe {
            let app: Option<Retained<AnyObject>> =
                msg_send![class!(UIApplication), sharedApplication];
            let Some(app) = app else {
                return;
            };
            let _: () = msg_send![&*app, setIdleTimerDisabled: awake];
        }
        diag::info!(
            Launch,
            "idle timer: {}",
            if awake {
                "screen kept awake (load / match)"
            } else {
                "system auto-lock restored (menus)"
            }
        );
    }
}
