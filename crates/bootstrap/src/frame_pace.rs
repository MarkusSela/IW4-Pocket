//! Frame pacing on iOS, all switches in `iw4l-env.txt`, read once at startup:
//!
//! - `IW4L_FRAME_PACE=off|0|30|40|60` (default off): hold each present to the
//!   given rate with `presentDrawable:afterMinimumDuration:` in the vendored
//!   wgpu-hal. Off is the plain present.
//! - `IW4L_DT_SNAP=0|1` (default 0, acts only with a pace): advance game time
//!   on the 120 Hz vsync grid (see `diag::frame_pacing::PaceClock`) through
//!   `TimeUpdateStrategy::ManualInstant`, instead of Bevy's sample taken after
//!   the previous present. Without a pace the time update stays automatic.
//! - `IW4L_FRAME_STATS=0|1` (default 1): install the hal's present hooks and log
//!   `frame pacing:` every 128 presents (`render_gpu::diag::frame_pacing`).
//!   0 leaves the hal without hooks.
//!
//! Every other platform registers nothing: the hooks only exist on the phone.
use bevy::prelude::*;

pub(crate) fn register(app: &mut App) {
    #[cfg(target_os = "ios")]
    ios::register(app);
    #[cfg(not(target_os = "ios"))]
    let _ = app;
}

#[cfg(target_os = "ios")]
mod ios {
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    use bevy::prelude::*;
    use bevy::time::{TimeSystems, TimeUpdateStrategy};
    use diag::frame_pacing::{FramePace, PaceClock, lead_ns};
    use render_gpu::diag::frame_pacing as stats;
    use wgpu_hal::metal::pacing;

    pub(super) fn register(app: &mut App) {
        let pace = FramePace::get();
        pacing::MIN_PRESENT_US.store(pace.min_present_us(), Ordering::Relaxed);
        if pace.stats {
            // Once per process. A second app keeps these hooks, which feed the
            // same statics.
            let _ = pacing::HOOKS.set(pacing::Hooks {
                submitted: stats::on_submitted,
                presented: stats::on_presented,
            });
            stats::register(app, pace);
        }
        if pace.snap_active() {
            app.insert_resource(PaceTime(PaceClock::new(pace.hz)))
                .add_systems(First, pace_time.before(TimeSystems));
        }
        diag::info!(
            Launch,
            "{}",
            pace.boot_line(crate::plugins::frame_latency())
        );
    }

    #[derive(Resource)]
    struct PaceTime(PaceClock);

    fn pace_time(mut clock: ResMut<PaceTime>, mut strategy: ResMut<TimeUpdateStrategy>) {
        let now = Instant::now();
        let at = clock.0.next(now);
        stats::FRAME_LEAD_NS.store(lead_ns(at, now), Ordering::Relaxed);
        *strategy = TimeUpdateStrategy::ManualInstant(at);
    }
}
