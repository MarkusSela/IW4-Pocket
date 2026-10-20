//! One `match summary:` line every 30 s while a match is running (`diag::events`).
//!
//! The figures are the ones the rest of the diagnostics already keep: frames counted
//! here, bots in the roster, footprint / available / heap from `diag::ios_env`, and the
//! peak footprint and least headroom from `diag::session`. Nothing is written in menus
//! or while a map loads.
use bevy::prelude::*;

pub(crate) fn register(app: &mut App) {
    app.add_systems(Last, log_match_summary);
}

#[derive(Default)]
struct Window {
    /// Real seconds since the match began (live).
    match_secs: f64,
    /// Real seconds and frames since the last summary line.
    secs: f64,
    frames: u32,
}

fn log_match_summary(
    time: Res<Time<Real>>,
    screen: Option<Res<frame::AppScreen>>,
    roster: Option<Res<bots::BotRoster>>,
    mut window: Local<Window>,
) {
    if screen.as_deref() != Some(&frame::AppScreen::InGame) {
        *window = Window::default();
        return;
    }
    let dt = time.delta_secs_f64();
    window.match_secs += dt;
    window.secs += dt;
    window.frames += 1;
    if window.secs < diag::events::MATCH_SUMMARY_EVERY_SECS {
        return;
    }
    let fps = (window.secs > 0.0).then(|| (f64::from(window.frames) / window.secs) as f32);
    let bots = roster.map_or(0, |roster| roster.bots.len() as u32);
    diag::events::match_summary(window.match_secs, bots, fps);
    window.secs = 0.0;
    window.frames = 0;
}
