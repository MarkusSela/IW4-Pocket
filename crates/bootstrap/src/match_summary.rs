//! One `match summary:` line every 30 s while a match is running, and one last line when it
//! ends (`diag::events`). The last line matters: a short match never reaches the 30 s mark,
//! and the log used to carry no summary of it at all.
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
    /// Bots seen during the last frame of the match: the roster is cleared when it ends.
    last_bots: u32,
}

/// Average frames per second over a window, or `None` when it holds no time.
fn average_fps(frames: u32, secs: f64) -> Option<f32> {
    (secs > 0.0).then(|| (f64::from(frames) / secs) as f32)
}

/// A closing line is due when the screen has left the match and something was played
/// since the last line (a match that just printed a periodic line has nothing new to add).
fn closing_line_due(window: &Window) -> bool {
    window.frames > 0
}

fn log_match_summary(
    time: Res<Time<Real>>,
    screen: Option<Res<frame::AppScreen>>,
    roster: Option<Res<bots::BotRoster>>,
    mut window: Local<Window>,
) {
    if screen.as_deref() != Some(&frame::AppScreen::InGame) {
        if closing_line_due(&window) {
            let fps = average_fps(window.frames, window.secs);
            diag::events::match_summary(window.match_secs, window.last_bots, fps);
        }
        *window = Window::default();
        return;
    }
    let dt = time.delta_secs_f64();
    window.match_secs += dt;
    window.secs += dt;
    window.frames += 1;
    let bots = roster.map_or(0, |roster| roster.bots.len() as u32);
    window.last_bots = bots;
    if window.secs < diag::events::MATCH_SUMMARY_EVERY_SECS {
        return;
    }
    let fps = average_fps(window.frames, window.secs);
    diag::events::match_summary(window.match_secs, bots, fps);
    window.secs = 0.0;
    window.frames = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_is_frames_over_seconds_and_none_without_time() {
        assert_eq!(average_fps(90, 3.0), Some(30.0));
        assert_eq!(average_fps(10, 0.0), None);
    }

    #[test]
    fn a_match_shorter_than_one_window_still_gets_a_closing_line() {
        let mut window = Window::default();
        assert!(!closing_line_due(&window), "nothing played, nothing to write");
        window.frames = 120;
        window.secs = 4.0;
        assert!(closing_line_due(&window), "4 s of play must be summarised when it ends");
    }

    #[test]
    fn right_after_a_periodic_line_there_is_nothing_new_to_close() {
        // The periodic path resets `frames` to 0 after writing.
        let window = Window { match_secs: 30.0, secs: 0.0, frames: 0, last_bots: 17 };
        assert!(!closing_line_due(&window));
    }
}
