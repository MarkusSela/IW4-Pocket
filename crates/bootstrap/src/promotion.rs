//! ProMotion on iOS. The game presents through a CAMetalLayer with nothing that
//! asks the display for more than 60 Hz, so on a 120 Hz iPhone every frame still
//! lands on the 16.7 ms grid: at the usual 35-45 fps in a match the frames show
//! for one or two refreshes in turn (16.7 / 33.3 ms), which is visible judder.
//! A display link whose preferred range is 120 Hz is the system's way to ask for
//! the faster refresh; it calls nothing useful, it only holds the request. It runs
//! during a match (class select and in game) and is paused elsewhere, so the
//! menus stay at 60 Hz and do not draw twice as often for nothing.
//!
//! `IW4L_PROMOTION=0` in `iw4l-env.txt` leaves the display alone. Every other
//! platform registers nothing.
use bevy::prelude::*;

pub(crate) fn register(app: &mut App) {
    #[cfg(target_os = "ios")]
    ios::register(app);
    #[cfg(not(target_os = "ios"))]
    let _ = app;
}

#[cfg(target_os = "ios")]
mod ios {
    use bevy::prelude::*;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{ClassType, Encode, Encoding, class, define_class, msg_send, sel};

    const PROMOTION_ENV: &str = "IW4L_PROMOTION";

    /// QuartzCore's `CAFrameRateRange` (iOS 15).
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CAFrameRateRange {
        minimum: f32,
        maximum: f32,
        preferred: f32,
    }

    // SAFETY: matches the C struct `{CAFrameRateRange=fff}`.
    unsafe impl Encode for CAFrameRateRange {
        const ENCODING: Encoding = Encoding::Struct(
            "CAFrameRateRange",
            &[f32::ENCODING, f32::ENCODING, f32::ENCODING],
        );
    }

    #[link(name = "Foundation", kind = "framework")]
    unsafe extern "C" {
        static NSRunLoopCommonModes: &'static AnyObject;
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and this type has no
        // Drop impl.
        #[unsafe(super(NSObject))]
        #[name = "IW4LDisplayRateHint"]
        struct DisplayRateHint;

        impl DisplayRateHint {
            #[unsafe(method(tick:))]
            fn tick(&self, _link: *mut AnyObject) {}
        }
    );

    /// The display link, touched on the main thread only.
    struct RateLink(Retained<AnyObject>);

    pub(super) fn register(app: &mut App) {
        if std::env::var(PROMOTION_ENV).is_ok_and(|value| value.trim() == "0") {
            diag::info!(
                Launch,
                "promotion: off ({PROMOTION_ENV}=0), display left at 60 Hz"
            );
            return;
        }
        let Some(link) = create_link() else {
            diag::warn!(
                Launch,
                "promotion: CADisplayLink unavailable, display left at 60 Hz"
            );
            return;
        };
        app.insert_non_send(RateLink(link))
            .add_systems(Last, follow_screen);
        diag::info!(
            Launch,
            "promotion: 120 Hz display link ready (matches only)"
        );
    }

    fn create_link() -> Option<Retained<AnyObject>> {
        // SAFETY: plain Foundation/QuartzCore messages with their documented
        // signatures, on the main thread during app setup. The run loop retains
        // the link and the link retains its target.
        unsafe {
            let target: Retained<DisplayRateHint> = msg_send![DisplayRateHint::class(), new];
            let link: Option<Retained<AnyObject>> = msg_send![
                class!(CADisplayLink),
                displayLinkWithTarget: &*target,
                selector: sel!(tick:)
            ];
            let link = link?;
            let range = CAFrameRateRange {
                minimum: 80.0,
                maximum: 120.0,
                preferred: 120.0,
            };
            let _: () = msg_send![&*link, setPreferredFrameRateRange: range];
            let _: () = msg_send![&*link, setPaused: true];
            let run_loop: Retained<AnyObject> = msg_send![class!(NSRunLoop), mainRunLoop];
            let _: () = msg_send![
                &*link,
                addToRunLoop: &*run_loop,
                forMode: NSRunLoopCommonModes
            ];
            Some(link)
        }
    }

    fn follow_screen(
        link: NonSend<RateLink>,
        screen: Option<Res<frame::AppScreen>>,
        mut live: Local<Option<bool>>,
    ) {
        let in_match = matches!(
            screen.as_deref(),
            Some(frame::AppScreen::ClassSelect | frame::AppScreen::InGame)
        );
        if *live == Some(in_match) {
            return;
        }
        *live = Some(in_match);
        // SAFETY: NonSend keeps this on the main thread; `paused` is a BOOL property.
        unsafe {
            let _: () = msg_send![&*link.0, setPaused: !in_match];
        }
        diag::info!(
            Launch,
            "promotion: {}",
            if in_match {
                "match, asking the display for 120 Hz"
            } else {
                "out of match, display back to 60 Hz"
            }
        );
    }
}
