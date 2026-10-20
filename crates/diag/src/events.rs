//! Log helpers other crates call, so the same facts always read the same way.
//!
//! Label convention in every line: `name(live)` = value right now, `name(total)` =
//! cumulative since the process or match started, `name(est)` = an estimate. A name with
//! no suffix is a plain fact about this one event.
use std::sync::Mutex;

use crate::{Channel, Level, write_event};

static MAP: Mutex<String> = Mutex::new(String::new());

/// Remember the map being loaded, for [`match_summary`]. Called when loading starts.
pub fn note_map(name: &str) {
    if let Ok(mut slot) = MAP.lock() {
        slot.clear();
        slot.push_str(name);
    }
}

fn map_name() -> String {
    MAP.lock()
        .map(|m| m.clone())
        .ok()
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "?".to_owned())
}

/// Bot roster change: what was asked for, what was added, how many bots are there now,
/// why some were refused, and the id→name map of the bots just created.
///
/// Written as a warning when `refusal` is set. Always written, in both detail levels.
pub fn bots_event(
    requested: u32,
    added: u32,
    present: u32,
    refusal: Option<&str>,
    names: &[(u32, &str)],
) {
    let names = names
        .iter()
        .map(|(id, name)| format!("{id}={name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!(
        "bots: roster requested={requested} added={added} present(live)={present} refused={} names=[{names}]",
        refusal.unwrap_or("none")
    );
    let lvl = if refusal.is_some() { Level::Warn } else { Level::Info };
    write_event(Channel::Sim, lvl, &line, Some("bots_roster"), None);
}

/// Weapon or class request: what was asked, how many weapon assets this request
/// loaded / reused / released, and how long it took.
pub fn weapon_event(
    kind: &str,
    requested: &str,
    loaded: u32,
    reused: u32,
    released: u32,
    took_ms: u64,
) {
    let line = format!(
        "weapons: {kind} request={requested} loaded={loaded} reused={reused} released={released} took_ms={took_ms}"
    );
    write_event(Channel::World, Level::Info, &line, Some("weapons"), None);
}

/// 3D render size, UI size (both in pixels) and the text scale in use.
pub fn resolution_event(render: (u32, u32), ui: (u32, u32), text_scale: f32) {
    let line = format!(
        "resolution: 3d={}x{} ui={}x{} text_scale={text_scale:.2}",
        render.0, render.1, ui.0, ui.1
    );
    write_event(Channel::Launch, Level::Info, &line, Some("resolution"), None);
}

/// Seconds between two match summaries.
pub const MATCH_SUMMARY_EVERY_SECS: f64 = 30.0;

/// One periodic line while a match is running. `bots` and `fps` are live values, the
/// memory figures come from the same counters as the `mem:` line; `?` means the platform
/// gave no reading.
pub fn match_summary(match_secs: f64, bots: u32, fps: Option<f32>) {
    const MIB: u64 = 1024 * 1024;
    let mib = |v: Option<u64>| v.map_or_else(|| "?".to_owned(), |b| (b / MIB).to_string());
    let (heap_live, heap_reserved) = crate::ios_env::heap_bytes().unzip();
    let fps = fps.map_or_else(|| "?".to_owned(), |f| format!("{f:.1}"));
    // The session line holds the peak footprint and the least headroom since the
    // process started (cumulative), not just this match.
    let session = crate::session::summary_line();
    let session = session.strip_prefix("session summary: ").unwrap_or(session.as_str());
    let line = format!(
        "match summary: t={match_secs:.0}s map={} bots(live)={bots} fps(avg last window)={fps} \
         footprint(live)={} MiB available(live)={} MiB heap live={} MiB reserved={} MiB \
         | since start: {session}",
        map_name(),
        mib(crate::ios_env::footprint_bytes()),
        mib(crate::ios_env::available_bytes()),
        mib(heap_live),
        mib(heap_reserved),
    );
    write_event(Channel::Launch, Level::Info, &line, Some("match_summary"), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_is_remembered() {
        note_map("mp_rust");
        assert_eq!(map_name(), "mp_rust");
    }
}
