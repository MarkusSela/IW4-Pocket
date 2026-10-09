//! On-screen frame timing (`IW4L_FRAME_STATS`, iOS only, on by default).
//!
//! The vendored wgpu-hal calls [`on_submitted`] after each present call and
//! [`on_presented`] from Metal's presented handler (any thread), keyed by the
//! drawable's id. Each frame a `Last` system moves the finished drawables into
//! [`PacingStats`] and every [`LOG_EVERY`] presents logs one `frame pacing:`
//! line: the on-screen interval mix in 120 Hz vsyncs, dropped drawables, the
//! frame time step against the interval it was shown for, the drawable wait
//! and the present call. Nothing is logged per frame.
//!
//! The time step is matched to the drawable through [`FRAME_DT_NS`], set after
//! the main world's time update and read by the present in the same
//! `App::update`, which holds while rendering is not pipelined (the iOS
//! default). With the stats off none of this is registered and the hal has no
//! hooks; [`note_drawable_wait`] is then a single atomic load.
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

use bevy::prelude::*;
use bevy::time::TimeSystems;
use bevy::window::{Monitor, PrimaryMonitor};
use diag::frame_pacing::{FramePace, LOG_EVERY, PacingStats, PresentRecord, PresentRing};

static ENABLED: AtomicBool = AtomicBool::new(false);
static RING: Mutex<PresentRing> = Mutex::new(PresentRing::new());

/// `Time<Real>` delta of the frame being drawn, in ns.
pub static FRAME_DT_NS: AtomicU64 = AtomicU64::new(0);
/// How far that frame's game time is ahead of the wall clock, in ns; written
/// only by the dt snap (bootstrap `frame_pace`).
pub static FRAME_LEAD_NS: AtomicI64 = AtomicI64::new(0);

/// wgpu-hal `pacing::Hooks::submitted`.
pub fn on_submitted(drawable_id: u64, present_call_ns: u64) {
    let record = PresentRecord::submitted(
        drawable_id,
        FRAME_DT_NS.load(Ordering::Relaxed),
        FRAME_LEAD_NS.load(Ordering::Relaxed),
        present_call_ns,
    );
    if let Ok(mut ring) = RING.lock() {
        ring.submit(record);
    }
}

/// wgpu-hal `pacing::Hooks::presented`; Metal calls it on its own thread.
pub fn on_presented(drawable_id: u64, presented_host_s: f64) {
    if let Ok(mut ring) = RING.lock() {
        ring.presented(drawable_id, presented_host_s);
    }
}

/// This frame's `PrepareViews` time, which is mostly the wait for a drawable.
pub(crate) fn note_drawable_wait(ms: f32) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut ring) = RING.lock() {
        ring.note_wait(ms);
    }
}

#[derive(Resource)]
struct PacingLog {
    pace: FramePace,
    stats: PacingStats,
}

/// Collect and log the stats. Call only when the hal hooks are installed.
pub fn register(app: &mut App, pace: FramePace) {
    ENABLED.store(true, Ordering::Relaxed);
    app.insert_resource(PacingLog {
        pace,
        stats: PacingStats::default(),
    })
    .add_systems(First, store_frame_dt.after(TimeSystems))
    .add_systems(Last, log_frame_pacing);
}

fn store_frame_dt(time: Res<Time<Real>>) {
    let ns = u64::try_from(time.delta().as_nanos()).unwrap_or(u64::MAX);
    FRAME_DT_NS.store(ns, Ordering::Relaxed);
}

fn log_frame_pacing(mut log: ResMut<PacingLog>, monitors: Query<(&Monitor, Has<PrimaryMonitor>)>) {
    let log = &mut *log;
    if let Ok(mut ring) = RING.lock() {
        ring.drain_into(&mut log.stats);
    }
    if log.stats.n() < LOG_EVERY {
        return;
    }
    let screen_max_fps = monitors
        .iter()
        .max_by_key(|(_, primary)| *primary)
        .and_then(|(monitor, _)| monitor.refresh_rate_millihertz)
        .map(|mhz| (mhz + 500) / 1000);
    diag::info!(World, "{}", log.stats.line(log.pace, screen_max_fps));
    log.stats.reset_window();
}
