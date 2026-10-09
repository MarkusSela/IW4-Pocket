//! Frame pacing settings and on-screen frame timing statistics (iOS).
//!
//! Only the logic lives here, so it tests on the host without Bevy or Metal:
//! the `IW4L_FRAME_PACE` / `IW4L_DT_SNAP` / `IW4L_FRAME_STATS` settings, the
//! minimum on-screen duration handed to `presentDrawable:afterMinimumDuration:`,
//! the clock that snaps the frame time step to the vsync grid, the ring that
//! pairs each submitted drawable with the time Metal says it reached the
//! screen, and the summary logged every [`LOG_EVERY`] presents.
//! `crates/bootstrap/src/frame_pace.rs` and
//! `crates/render_gpu/src/diag/frame_pacing.rs` wire them into the app and into
//! the vendored wgpu-hal (`metal::pacing`).
use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub const FRAME_PACE_ENV: &str = "IW4L_FRAME_PACE";
pub const DT_SNAP_ENV: &str = "IW4L_DT_SNAP";
pub const FRAME_STATS_ENV: &str = "IW4L_FRAME_STATS";

/// One vsync of a 120 Hz panel in ns. A 60 Hz panel's vsync is two of these, so
/// the same grid fits every iPhone.
pub const VSYNC_UNIT_NS: u64 = 8_333_333;
const VSYNC_UNIT_MS: f64 = 1000.0 / 120.0;
/// An interval further than this from a whole number of 120 Hz vsyncs is
/// `off_grid`: a 48 or 80 Hz mode lands 4.17 ms off.
const OFF_GRID_MS: f64 = 2.0;
/// Presents per `frame pacing:` line.
pub const LOG_EVERY: u32 = 128;
/// A submitted drawable whose presented handler has not run after this many
/// newer submissions is counted as lost instead of holding up the queue.
const EXPIRE_AFTER: usize = 32;
const RING_CAPACITY: usize = 256;
const WAIT_CAPACITY: usize = 512;
const EARLY_CAPACITY: usize = 8;

/// The restart-time frame pacing switches, read once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePace {
    /// Presents per second the present is held to; 0 = off (plain present).
    pub hz: u32,
    /// `IW4L_DT_SNAP`: put the frame time step on the vsync grid. Acts only
    /// with a pace; see [`FramePace::snap_active`].
    pub snap: bool,
    /// `IW4L_FRAME_STATS`: install the present hooks and log `frame pacing:`.
    pub stats: bool,
}

impl FramePace {
    /// Everything off: what every platform but iOS gets.
    pub const OFF: Self = Self {
        hz: 0,
        snap: false,
        stats: false,
    };

    /// Off the phone every switch is ignored: the hooks only exist there.
    /// On iOS the pace is off unless set, the snap is off unless set (it only acts
    /// with a pace; see [`PaceClock`] for why it is not on by default) and the
    /// stats default to on. An invalid value keeps the
    /// default and returns a warning to log.
    pub fn parse(
        ios: bool,
        pace: Option<&str>,
        snap: Option<&str>,
        stats: Option<&str>,
    ) -> (Self, Vec<String>) {
        if !ios {
            return (Self::OFF, Vec::new());
        }
        let mut warnings = Vec::new();
        let hz = match pace.map(str::trim) {
            None | Some("off" | "0") => 0,
            Some("30") => 30,
            Some("40") => 40,
            Some("60") => 60,
            Some(other) => {
                warnings.push(format!(
                    "{FRAME_PACE_ENV}={other} is not off|0|30|40|60; using off"
                ));
                0
            }
        };
        let snap = switch(DT_SNAP_ENV, snap, false, &mut warnings);
        let stats = switch(FRAME_STATS_ENV, stats, true, &mut warnings);
        (Self { hz, snap, stats }, warnings)
    }

    /// Read once from the environment (after the iOS settings file).
    pub fn get() -> Self {
        static PACE: OnceLock<FramePace> = OnceLock::new();
        *PACE.get_or_init(|| {
            let var = |name| std::env::var(name).ok();
            let (pace, warnings) = Self::parse(
                cfg!(target_os = "ios"),
                var(FRAME_PACE_ENV).as_deref(),
                var(DT_SNAP_ENV).as_deref(),
                var(FRAME_STATS_ENV).as_deref(),
            );
            for warning in warnings {
                crate::warn!(Launch, "{warning}");
            }
            pace
        })
    }

    pub fn snap_active(self) -> bool {
        self.hz > 0 && self.snap
    }

    pub fn min_present_us(self) -> u32 {
        min_present_us(self.hz)
    }

    /// The smallest time step, in 120 Hz vsyncs, while the pace holds.
    pub fn floor_units(self) -> u32 {
        floor_units(self.hz)
    }

    /// The boot line, e.g. `frame pace: pace=30 Hz min_present=31.33 ms
    /// frame_latency=2 dt_snap=on unit=8.33 ms floor=4 frame_stats=on`.
    pub fn boot_line(self, frame_latency: u32) -> String {
        let stats = on_off(self.stats);
        if self.hz == 0 {
            let snap = if self.snap { "off (no pace)" } else { "off" };
            return format!(
                "frame pace: pace=off (plain present, automatic time step) \
                 frame_latency={frame_latency} dt_snap={snap} frame_stats={stats}"
            );
        }
        format!(
            "frame pace: pace={} Hz min_present={:.2} ms frame_latency={frame_latency} \
             dt_snap={} unit={VSYNC_UNIT_MS:.2} ms floor={} frame_stats={stats}",
            self.hz,
            f64::from(self.min_present_us()) / 1000.0,
            on_off(self.snap),
            self.floor_units(),
        )
    }
}

fn switch(name: &str, value: Option<&str>, default: bool, warnings: &mut Vec<String>) -> bool {
    match value.map(str::trim) {
        None => default,
        Some("1") => true,
        Some("0") => false,
        Some(other) => {
            warnings.push(format!(
                "{name}={other} is not 0|1; using {}",
                u8::from(default)
            ));
            default
        }
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// The minimum on-screen duration for a pace: its period less 2 ms, which sits
/// strictly between two 120 Hz vsync counts and clear of the 48/80 Hz
/// intervals, so the present rounds onto the intended vsync at 60 and 120 Hz.
/// 30 -> 31333, 40 -> 23000, 60 -> 14667; 0 (off) -> 0.
pub fn min_present_us(hz: u32) -> u32 {
    if hz == 0 {
        return 0;
    }
    (1_000_000 + hz / 2) / hz - 2000
}

fn floor_units(hz: u32) -> u32 {
    if hz == 0 { 0 } else { (120 / hz).max(1) }
}

/// The frame time step snapped to the 120 Hz vsync grid, with the pace as a
/// floor. Fed the wall clock at the start of every frame, it returns the
/// instant to advance game time to:
///
/// - a frame that holds the pace steps exactly `floor` vsyncs, as long as each
///   sample stays within half a vsync of the first sample's phase;
/// - a missed frame steps one vsync more, on the frame that sees it;
/// - game time is never more than one pace step ahead of the wall clock (it
///   holds still when ahead) and never more than half a vsync behind.
#[derive(Clone, Debug)]
pub struct PaceClock {
    prev: Option<Instant>,
    floor: u64,
}

impl PaceClock {
    pub fn new(hz: u32) -> Self {
        Self {
            prev: None,
            floor: u64::from(floor_units(hz).max(1)),
        }
    }

    pub fn next(&mut self, now: Instant) -> Instant {
        let Some(prev) = self.prev else {
            self.prev = Some(now);
            return now;
        };
        let units = match now.checked_duration_since(prev) {
            // Honour the pace unless game time is already ahead.
            Some(behind) => round_units(behind).max(self.floor),
            // Game time is ahead: rounding a negative gap gives zero or less.
            None => 0,
        };
        let next = prev + Duration::from_nanos(units.saturating_mul(VSYNC_UNIT_NS));
        self.prev = Some(next);
        next
    }
}

fn round_units(gap: Duration) -> u64 {
    let units = (gap.as_nanos() + u128::from(VSYNC_UNIT_NS / 2)) / u128::from(VSYNC_UNIT_NS);
    u64::try_from(units).unwrap_or(u64::MAX)
}

/// How far `game` is ahead of `wall` in ns (negative when behind).
pub fn lead_ns(game: Instant, wall: Instant) -> i64 {
    match game.checked_duration_since(wall) {
        Some(ahead) => i64::try_from(ahead.as_nanos()).unwrap_or(i64::MAX),
        None => -i64::try_from(wall.duration_since(game).as_nanos()).unwrap_or(i64::MAX),
    }
}

/// One drawable, from its present call to Metal's presented callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentRecord {
    pub drawable_id: u64,
    /// `Time<Real>` delta of the frame that drew it.
    pub dt_ns: u64,
    /// How far that frame's game time was ahead of the wall clock (snap only).
    pub lead_ns: i64,
    /// Time spent in presentDrawable and commit.
    pub present_call_ns: u64,
    /// Metal's `presentedTime` in host seconds. `Some(0.0)`: never shown
    /// (dropped). `None`: the callback has not run, or never did (lost).
    pub presented_s: Option<f64>,
}

impl PresentRecord {
    pub fn submitted(drawable_id: u64, dt_ns: u64, lead_ns: i64, present_call_ns: u64) -> Self {
        Self {
            drawable_id,
            dt_ns,
            lead_ns,
            present_call_ns,
            presented_s: None,
        }
    }
}

/// Submitted drawables waiting for their presented callback, in submission
/// order, plus each frame's drawable wait. Bounded: nothing here grows with
/// the length of a run.
#[derive(Debug, Default)]
pub struct PresentRing {
    pending: VecDeque<PresentRecord>,
    /// Callbacks that ran before their drawable was recorded.
    early: Vec<(u64, f64)>,
    waits_ms: Vec<f32>,
}

impl PresentRing {
    pub const fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            early: Vec::new(),
            waits_ms: Vec::new(),
        }
    }

    pub fn submit(&mut self, mut record: PresentRecord) {
        if let Some(i) = self
            .early
            .iter()
            .position(|(id, _)| *id == record.drawable_id)
        {
            record.presented_s = Some(self.early.swap_remove(i).1);
        }
        if self.pending.len() == RING_CAPACITY {
            self.pending.pop_front();
        }
        self.pending.push_back(record);
    }

    pub fn presented(&mut self, drawable_id: u64, presented_s: f64) {
        if let Some(record) = self
            .pending
            .iter_mut()
            .rev()
            .find(|r| r.drawable_id == drawable_id && r.presented_s.is_none())
        {
            record.presented_s = Some(presented_s);
            return;
        }
        if self.early.len() == EARLY_CAPACITY {
            self.early.remove(0);
        }
        self.early.push((drawable_id, presented_s));
    }

    pub fn note_wait(&mut self, ms: f32) {
        if self.waits_ms.len() < WAIT_CAPACITY {
            self.waits_ms.push(ms);
        }
    }

    /// Move every finished record (in order) and every wait into `stats`. A
    /// record still waiting with [`EXPIRE_AFTER`] newer ones behind it is
    /// handed over as lost.
    pub fn drain_into(&mut self, stats: &mut PacingStats) {
        while let Some(front) = self.pending.front() {
            if front.presented_s.is_none() && self.pending.len() <= EXPIRE_AFTER {
                break;
            }
            if let Some(record) = self.pending.pop_front() {
                stats.push(record);
            }
        }
        stats.waits_ms.append(&mut self.waits_ms);
    }
}

/// Bucket labels of [`Summary::hist`], in 120 Hz vsyncs 1..=6 and more.
const HIST_LABELS: [&str; 7] = ["8.3", "16.7", "25", "33.3", "41.7", "50", ">50"];

/// One window of presents. Intervals are measured between presented
/// drawables in submission order; a dropped drawable keeps the previous one on
/// screen, so the interval runs across it, and a lost callback or a gap in
/// drawable ids starts a new chain. The step error needs consecutive ids.
#[derive(Debug, Default)]
pub struct PacingStats {
    n: u32,
    intervals_ms: Vec<f32>,
    dt_ms: Vec<f32>,
    err_ms: Vec<f32>,
    call_ms: Vec<f32>,
    lead_ms: Vec<f32>,
    waits_ms: Vec<f32>,
    hist: [u32; 7],
    off_grid: u32,
    dropped: u32,
    lost: u32,
    /// Carried across windows.
    last_id: Option<u64>,
    last_presented: Option<(u64, f64)>,
}

impl PacingStats {
    pub fn n(&self) -> u32 {
        self.n
    }

    pub fn push(&mut self, record: PresentRecord) {
        let contiguous = self.last_id.and_then(|id| id.checked_add(1)) == Some(record.drawable_id);
        if !contiguous {
            self.last_presented = None;
        }
        self.last_id = Some(record.drawable_id);
        self.n += 1;
        let dt_ms = ns_ms(record.dt_ns);
        self.dt_ms.push(dt_ms);
        self.call_ms.push(ns_ms(record.present_call_ns));
        self.lead_ms.push(record.lead_ns as f32 / 1e6);
        match record.presented_s {
            None => {
                self.lost += 1;
                self.last_presented = None;
            }
            Some(at) if at <= 0.0 => self.dropped += 1,
            Some(at) => {
                if let Some((prev_id, prev_at)) = self.last_presented {
                    let interval = (at - prev_at) * 1000.0;
                    self.intervals_ms.push(interval as f32);
                    self.bucket(interval);
                    if prev_id.checked_add(1) == Some(record.drawable_id) {
                        self.err_ms.push((f64::from(dt_ms) - interval).abs() as f32);
                    }
                }
                self.last_presented = Some((record.drawable_id, at));
            }
        }
    }

    fn bucket(&mut self, interval_ms: f64) {
        let units = (interval_ms / VSYNC_UNIT_MS).round();
        if units < 1.0 || (interval_ms - units * VSYNC_UNIT_MS).abs() > OFF_GRID_MS {
            self.off_grid += 1;
        } else {
            self.hist[(units as usize).min(7) - 1] += 1;
        }
    }

    /// Start the next window; the interval chain carries over.
    pub fn reset_window(&mut self) {
        self.n = 0;
        for v in [
            &mut self.intervals_ms,
            &mut self.dt_ms,
            &mut self.err_ms,
            &mut self.call_ms,
            &mut self.lead_ms,
            &mut self.waits_ms,
        ] {
            v.clear();
        }
        self.hist = [0; 7];
        self.off_grid = 0;
        self.dropped = 0;
        self.lost = 0;
    }

    pub fn summary(&self) -> Summary {
        let p = |v: &[f32], qs: &[f32]| -> Vec<Option<f32>> {
            let mut sorted = v.to_vec();
            sorted.sort_by(f32::total_cmp);
            qs.iter().map(|q| percentile(&sorted, *q)).collect()
        };
        let four = |v: Vec<Option<f32>>| [v[0], v[1], v[2], v[3]];
        let three = |v: Vec<Option<f32>>| [v[0], v[1], v[2]];
        Summary {
            n: self.n,
            interval_ms: four(p(&self.intervals_ms, &[0.1, 0.5, 0.9, 1.0])),
            hist: self.hist,
            off_grid: self.off_grid,
            dropped: self.dropped,
            lost: self.lost,
            dt_ms: three(p(&self.dt_ms, &[0.1, 0.5, 0.9])),
            err_ms: three(p(&self.err_ms, &[0.5, 0.9, 1.0])),
            wait_ms: three(p(&self.waits_ms, &[0.5, 0.9, 1.0])),
            present_call_ms: {
                let v = p(&self.call_ms, &[0.5, 1.0]);
                [v[0], v[1]]
            },
            lead_max_ms: p(&self.lead_ms, &[1.0])[0],
        }
    }

    /// The `frame pacing:` line. `screen_max_fps` is the panel's highest rate.
    pub fn line(&self, pace: FramePace, screen_max_fps: Option<u32>) -> String {
        let s = self.summary();
        let hist = HIST_LABELS
            .iter()
            .zip(s.hist)
            .map(|(label, n)| format!("{label}:{n}"))
            .collect::<Vec<_>>()
            .join(" ");
        let pace_label = if pace.hz == 0 {
            "off".to_owned()
        } else {
            pace.hz.to_string()
        };
        let screen = screen_max_fps.map_or_else(|| "?".to_owned(), |fps| fps.to_string());
        let lead = if pace.snap_active() {
            ms(s.lead_max_ms)
        } else {
            "-".to_owned()
        };
        format!(
            "frame pacing: pace={pace_label} snap={} n={} screen_max_fps={screen} \
             interval_ms p10/p50/p90/max={} hist{{{hist} off_grid:{}}} dropped={} lost={} \
             | dt_ms p10/p50/p90={} | err_ms p50/p90/max={} | wait_ms p50/p90/max={} \
             | present_call_ms p50/max={} | lead_ms max={lead}",
            on_off(pace.snap_active()),
            s.n,
            slash(&s.interval_ms),
            s.off_grid,
            s.dropped,
            s.lost,
            slash(&s.dt_ms),
            slash(&s.err_ms),
            slash(&s.wait_ms),
            slash(&s.present_call_ms),
        )
    }
}

/// Percentiles of one window; `None` where nothing was measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub n: u32,
    /// p10, p50, p90, max of on-screen intervals.
    pub interval_ms: [Option<f32>; 4],
    /// On-screen intervals by 120 Hz vsync count: 1..=6, then more.
    pub hist: [u32; 7],
    pub off_grid: u32,
    pub dropped: u32,
    pub lost: u32,
    /// p10, p50, p90 of the time step.
    pub dt_ms: [Option<f32>; 3],
    /// p50, p90, max of |time step - on-screen interval|.
    pub err_ms: [Option<f32>; 3],
    /// p50, p90, max of the per-frame drawable wait.
    pub wait_ms: [Option<f32>; 3],
    /// p50, max of the present call.
    pub present_call_ms: [Option<f32>; 2],
    pub lead_max_ms: Option<f32>,
}

/// Nearest rank on a sorted slice.
fn percentile(sorted: &[f32], q: f32) -> Option<f32> {
    let last = sorted.len().checked_sub(1)?;
    sorted.get(((last as f32) * q).round() as usize).copied()
}

fn ns_ms(ns: u64) -> f32 {
    (ns as f64 / 1e6) as f32
}

fn ms(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| format!("{v:.2}"))
}

fn slash(values: &[Option<f32>]) -> String {
    values.iter().map(|v| ms(*v)).collect::<Vec<_>>().join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: Duration = Duration::from_nanos(VSYNC_UNIT_NS);

    fn ms_d(ms: f64) -> Duration {
        Duration::from_secs_f64(ms / 1000.0)
    }

    fn units_between(a: Instant, b: Instant) -> f64 {
        b.duration_since(a).as_nanos() as f64 / VSYNC_UNIT_NS as f64
    }

    #[test]
    fn parse_defaults_and_invalid_values() {
        let (ios, warnings) = FramePace::parse(true, None, None, None);
        assert_eq!(
            ios,
            FramePace {
                hz: 0,
                snap: false,
                stats: true
            }
        );
        assert!(warnings.is_empty());
        assert!(!ios.snap_active());
        assert_eq!(ios.min_present_us(), 0);

        // Desktop ignores every switch: the hooks only exist on the phone.
        let (desktop, warnings) = FramePace::parse(false, Some("30"), Some("1"), Some("1"));
        assert_eq!(desktop, FramePace::OFF);
        assert!(warnings.is_empty());

        for off in ["off", "0", " off "] {
            assert_eq!(FramePace::parse(true, Some(off), None, None).0.hz, 0);
        }
        for (value, hz) in [("30", 30), ("40", 40), ("60", 60)] {
            let (pace, warnings) = FramePace::parse(true, Some(value), None, None);
            assert_eq!(pace.hz, hz);
            assert!(!pace.snap_active());
            assert!(
                FramePace::parse(true, Some(value), Some("1"), None)
                    .0
                    .snap_active()
            );
            assert!(warnings.is_empty());
        }

        let (junk, warnings) = FramePace::parse(true, Some("45"), Some("yes"), Some("2"));
        assert_eq!(
            junk,
            FramePace {
                hz: 0,
                snap: false,
                stats: true
            }
        );
        assert_eq!(warnings.len(), 3);

        let (set, _) = FramePace::parse(true, Some("30"), Some("0"), Some("0"));
        assert_eq!(
            set,
            FramePace {
                hz: 30,
                snap: false,
                stats: false
            }
        );
        assert!(!set.snap_active());
    }

    #[test]
    fn boot_line_names_every_switch() {
        let (pace, _) = FramePace::parse(true, Some("30"), Some("1"), None);
        assert_eq!(
            pace.boot_line(2),
            "frame pace: pace=30 Hz min_present=31.33 ms frame_latency=2 dt_snap=on \
             unit=8.33 ms floor=4 frame_stats=on"
        );
        let (off, _) = FramePace::parse(true, None, None, Some("0"));
        assert!(off.boot_line(2).starts_with("frame pace: pace=off"));
        assert!(off.boot_line(2).ends_with("dt_snap=off frame_stats=off"));
    }

    #[test]
    fn min_present_sits_between_vsync_counts_and_clear_of_other_modes() {
        assert_eq!(min_present_us(30), 31_333);
        assert_eq!(min_present_us(40), 23_000);
        assert_eq!(min_present_us(60), 14_667);
        assert_eq!(min_present_us(0), 0);
        for hz in [30u32, 40, 60] {
            let k = f64::from(120 / hz);
            let min_ms = f64::from(min_present_us(hz)) / 1000.0;
            assert!(min_ms > (k - 1.0) * VSYNC_UNIT_MS, "{hz}: {min_ms}");
            assert!(min_ms < k * VSYNC_UNIT_MS, "{hz}: {min_ms}");
            for other in [1000.0 / 48.0, 1000.0 / 80.0, 1000.0 / 40.0] {
                assert!((min_ms - other).abs() > 0.5, "{hz}: {min_ms} vs {other}");
            }
        }
        assert_eq!(
            FramePace {
                hz: 40,
                snap: true,
                stats: true
            }
            .floor_units(),
            3
        );
        assert_eq!(
            FramePace {
                hz: 60,
                snap: true,
                stats: true
            }
            .floor_units(),
            2
        );
    }

    #[test]
    fn jittered_frames_at_the_pace_step_exactly_four_units() {
        let base = Instant::now();
        let mut clock = PaceClock::new(30);
        let mut prev = clock.next(base);
        // Deterministic jitter in [-4, 4] ms around a 33.33 ms cadence.
        let mut seed = 0x2545_f491_u32;
        for k in 1..300u32 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let jitter = f64::from(seed >> 8) / f64::from(1u32 << 24) * 8.0 - 4.0;
            let now = base + ms_d(f64::from(k) * 1000.0 / 30.0 + jitter);
            let next = clock.next(now);
            assert_eq!(next.duration_since(prev), UNIT * 4, "frame {k}");
            assert!(lead_ns(next, now) <= (UNIT * 4).as_nanos() as i64);
            prev = next;
        }
    }

    #[test]
    fn one_missed_vsync_is_one_extra_unit_without_catch_up() {
        let base = Instant::now();
        let mut clock = PaceClock::new(30);
        let mut wall = base;
        let mut prev = clock.next(wall);
        let mut steps = Vec::new();
        for k in 0..20 {
            wall += if k == 10 { UNIT * 5 } else { UNIT * 4 };
            let next = clock.next(wall);
            steps.push(units_between(prev, next).round() as u32);
            prev = next;
        }
        let mut expected = vec![4; 20];
        expected[10] = 5;
        assert_eq!(steps, expected);
    }

    #[test]
    fn unheld_pace_stays_within_one_step_and_never_steps_back() {
        let base = Instant::now();
        let mut clock = PaceClock::new(30);
        let mut prev = clock.next(base);
        let step = (UNIT * 4).as_nanos() as i64;
        for k in 1..200u32 {
            let now = base + ms_d(f64::from(k) * 16.7);
            let next = clock.next(now);
            assert!(next >= prev, "frame {k} stepped back");
            let lead = lead_ns(next, now);
            assert!(lead <= step, "frame {k} lead {lead}");
            assert!(lead >= -(VSYNC_UNIT_NS as i64) / 2, "frame {k} lag {lead}");
            prev = next;
        }
    }

    #[test]
    fn a_long_hitch_is_one_step_to_the_wall_clock() {
        let base = Instant::now();
        let mut clock = PaceClock::new(30);
        clock.next(base);
        let now = base + ms_d(500.0);
        let next = clock.next(now);
        assert!(lead_ns(next, now).abs() <= (VSYNC_UNIT_NS / 2) as i64);
        assert_eq!(units_between(base, next).round(), 60.0);
    }

    /// Presented `at_ms` after a 1000 s host-time base, or dropped (`None`).
    fn record(id: u64, dt_ms: f64, at_ms: Option<f64>) -> PresentRecord {
        PresentRecord {
            presented_s: Some(at_ms.map_or(0.0, |ms| 1000.0 + ms / 1000.0)),
            ..PresentRecord::submitted(id, (dt_ms * 1e6) as u64, 0, 250_000)
        }
    }

    #[test]
    fn stats_bucket_intervals_and_pair_errors_by_drawable() {
        // Presented at 0, 33.3, 66.7, 108.3, dropped, 141.7 ms.
        let mut stats = PacingStats::default();
        let presented = [
            Some(0.0),
            Some(33.3),
            Some(66.7),
            Some(108.3),
            None,
            Some(141.7),
        ];
        for (i, at) in presented.into_iter().enumerate() {
            stats.push(record(10 + i as u64, 33.3, at));
        }
        let s = stats.summary();
        assert_eq!(s.n, 6);
        assert_eq!(s.dropped, 1);
        assert_eq!(s.lost, 0);
        // 33.3, 33.4, 41.6 and, across the dropped drawable, 33.4.
        assert_eq!(s.hist, [0, 0, 0, 3, 1, 0, 0]);
        assert_eq!(s.off_grid, 0);
        let close = |v: Option<f32>, want: f32| (v.unwrap() - want).abs() < 0.01;
        assert!(close(s.interval_ms[0], 33.3), "{:?}", s.interval_ms);
        assert!(close(s.interval_ms[1], 33.4), "{:?}", s.interval_ms);
        assert!(close(s.interval_ms[2], 41.6), "{:?}", s.interval_ms);
        assert!(close(s.interval_ms[3], 41.6), "{:?}", s.interval_ms);
        // Errors only between consecutive presented ids (0.0, 0.1, 8.3), not
        // across the dropped drawable.
        assert!(close(s.err_ms[0], 0.1), "{:?}", s.err_ms);
        assert!(close(s.err_ms[1], 8.3), "{:?}", s.err_ms);
        assert!(close(s.err_ms[2], 8.3), "{:?}", s.err_ms);
        assert!(close(s.dt_ms[1], 33.3));
        assert!(close(s.present_call_ms[1], 0.25));
        assert_eq!(s.wait_ms, [None, None, None]);

        // The chain carries into the next window; a 48 Hz interval is off the
        // grid, and a gap in drawable ids starts a new chain.
        stats.reset_window();
        stats.push(record(16, 20.8, Some(162.5)));
        stats.push(record(18, 33.3, Some(195.8)));
        let s = stats.summary();
        assert_eq!((s.n, s.off_grid, s.hist, s.dropped), (2, 1, [0; 7], 0));
        assert_eq!(s.interval_ms[3].map(|v| (v * 10.0).round()), Some(208.0));
        assert!(close(s.err_ms[2], 0.0), "{:?}", s.err_ms);
        let pace = FramePace {
            hz: 30,
            snap: true,
            stats: true,
        };
        let line = stats.line(pace, Some(120));
        assert!(
            line.starts_with("frame pacing: pace=30 snap=on n=2 screen_max_fps=120 "),
            "{line}"
        );
        assert!(
            line.contains("hist{8.3:0 16.7:0 25:0 33.3:0 41.7:0 50:0 >50:0 off_grid:1}"),
            "{line}"
        );
        assert!(line.contains("| wait_ms p50/p90/max=-/-/- |"), "{line}");
        assert!(line.ends_with("| lead_ms max=0.00"), "{line}");
        assert!(
            stats
                .line(FramePace::OFF, None)
                .ends_with("| lead_ms max=-")
        );
    }

    #[test]
    fn ring_pairs_callbacks_in_order_and_expires_lost_ones() {
        let mut ring = PresentRing::new();
        let mut stats = PacingStats::default();
        // A callback that beats its record is kept until the record arrives.
        ring.presented(1, 1000.0);
        ring.submit(PresentRecord::submitted(1, 33_333_333, 0, 0));
        ring.submit(PresentRecord::submitted(2, 33_333_333, 0, 0));
        ring.submit(PresentRecord::submitted(3, 33_333_333, 0, 0));
        ring.presented(3, 1000.0 + 0.066_666);
        ring.note_wait(2.5);
        ring.drain_into(&mut stats);
        // 2 has no callback yet, so 3 waits behind it.
        assert_eq!(stats.n(), 1);
        assert_eq!(stats.summary().wait_ms[0], Some(2.5));
        ring.presented(2, 1000.0 + 0.033_333);
        ring.drain_into(&mut stats);
        assert_eq!(stats.n(), 3);
        assert_eq!(stats.summary().hist[3], 2);

        // A callback that never comes is handed over as lost.
        ring.submit(PresentRecord::submitted(4, 33_333_333, 0, 0));
        for id in 5..5 + EXPIRE_AFTER as u64 {
            ring.submit(PresentRecord::submitted(id, 33_333_333, 0, 0));
            ring.presented(id, 2000.0 + id as f64 / 30.0);
        }
        ring.drain_into(&mut stats);
        let s = stats.summary();
        assert_eq!(s.lost, 1);
        assert_eq!(s.n, 4 + EXPIRE_AFTER as u32);
    }

    #[test]
    fn env_names_are_allowlisted() {
        use crate::memory_settings::valid_file_setting;
        for v in ["off", "0", "30", "40", "60"] {
            assert!(valid_file_setting(FRAME_PACE_ENV, v), "{v}");
        }
        for name in [DT_SNAP_ENV, FRAME_STATS_ENV] {
            assert!(valid_file_setting(name, "0"));
            assert!(valid_file_setting(name, "1"));
        }
    }
}
