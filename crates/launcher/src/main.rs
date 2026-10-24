use std::path::PathBuf;

use asset_transport::{ensure_artifacts_dir, games_root_from_env};

#[global_allocator]
static PROCESS_ALLOCATOR: diag::ProcessCountingAllocator = diag::ProcessCountingAllocator;

const LICENSES: [(&str, &str); 5] = [
    ("LICENSE", include_str!("../../../LICENSE")),
    ("NOTICE", include_str!("../../../NOTICE")),
    (
        "OFL-Oxanium.txt",
        include_str!("../../ui/assets/OFL-Oxanium.txt"),
    ),
    (
        "OFL-FiraMono.txt",
        include_str!("../../console/assets/OFL-FiraMono.txt"),
    ),
    (
        "THIRD-PARTY-LICENSES.txt",
        include_str!("../../../THIRD-PARTY-LICENSES.txt"),
    ),
];

/// iOS sandbox: everything lives in the app's Documents folder, which the Files app
/// can reach (UIFileSharingEnabled). Game data goes in Documents/Games.
#[cfg(target_os = "ios")]
fn prepare_ios_sandbox() {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let docs = home.join("Documents");
    let games = docs.join("Games");
    let artifacts = docs.join("iw4l-artifacts");
    let config = home.join("Library").join("Application Support");
    for dir in [&games, &artifacts, &config] {
        let _ = std::fs::create_dir_all(dir);
    }
    // SAFETY: called first thing in main, before any thread is spawned.
    unsafe {
        if std::env::var_os("IW4L_GAMES").is_none() {
            std::env::set_var("IW4L_GAMES", &games);
        }
        if std::env::var_os("IW4L_ARTIFACTS_DIR").is_none() {
            std::env::set_var("IW4L_ARTIFACTS_DIR", &artifacts);
        }
        std::env::set_var("XDG_CONFIG_HOME", &config);
    }
}

/// iOS has no console and jetsam kills leave no panic. Sample 4x/s and log on every 40 MB
/// change (or every 2 s): footprint (what iOS enforces, GPU included), what iOS still
/// allows, and the RGBA texture bytes sent to the GPU. The last line before a silent exit
/// shows how much memory was left.
#[cfg(target_os = "ios")]
fn spawn_memory_logger() {
    let _ = std::thread::Builder::new().name("mem-log".into()).spawn(|| {
        let mut last_mb = 0u64;
        let mut last_log = std::time::Instant::now();
        let mut last_classes = std::time::Instant::now();
        let mut last_relief = std::time::Instant::now() - std::time::Duration::from_secs(60);
        let mut last_relief_log = std::time::Instant::now() - std::time::Duration::from_secs(60);
        loop {
            if let Some(huge) = diag::take_last_huge() {
                diag::boot_crumb(&format!("big allocation: {} MB requested", huge / (1024 * 1024)));
            }
            if last_classes.elapsed() >= std::time::Duration::from_secs(5) {
                diag::boot_crumb(&diag::size_class_report());
                let per_kind = diag::asset_heap::report();
                if !per_kind.is_empty() {
                    diag::boot_crumb(&per_kind);
                }
                // Last line of a run that iOS ends silently: peak and least headroom so far.
                diag::boot_crumb(&diag::session::summary_line());
                last_classes = std::time::Instant::now();
            }
            let footprint = diag::ios_env::footprint_bytes().unwrap_or(0) / (1024 * 1024);
            // Every 250 ms, so the peak and the least headroom are not missed between lines.
            diag::session::note_sample(
                footprint,
                diag::ios_env::available_bytes().map(|b| b / (1024 * 1024)),
            );
            // Close to the limit the steps shrink, so the lines before a silent kill
            // show how fast memory was climbing (a 40 MiB step hid the last seconds).
            let headroom = diag::ios_env::available_bytes().unwrap_or(u64::MAX) / (1024 * 1024);
            // Close to the kill line, hand back allocator caches at most every 2 s.
            if headroom < 400 && last_relief.elapsed() >= std::time::Duration::from_secs(2) {
                let footprint_before = diag::ios_env::footprint_bytes().unwrap_or(0);
                let (took, released) = diag::release_freed_heap_report();
                let footprint_after = diag::ios_env::footprint_bytes().unwrap_or(0);
                if released > 0 || last_relief_log.elapsed() >= std::time::Duration::from_secs(10) {
                    diag::boot_crumb(&format!(
                        "heap relief under pressure: released_bytes={released} footprint_before_bytes={footprint_before} footprint_after_bytes={footprint_after} in {:.1}ms (iOS still allows {headroom} MiB)",
                        took.as_secs_f32() * 1000.0
                    ));
                    last_relief_log = std::time::Instant::now();
                }
                last_relief = std::time::Instant::now();
            }
            let step = if headroom < 300 { 8 } else { 40 };
            let changed = footprint.abs_diff(last_mb) >= step;
            if changed || last_log.elapsed() >= std::time::Duration::from_secs(2) {
                let available_mb = diag::ios_env::available_bytes().unwrap_or(0) / (1024 * 1024);
                let (heap_used, heap_reserved) = diag::ios_env::heap_bytes().unwrap_or((0, 0));
                let (heap_used, heap_reserved) = (heap_used / (1024 * 1024), heap_reserved / (1024 * 1024));
                let fpv_retained = diag::memtrack::FPV_RETAINED_BYTES.load(std::sync::atomic::Ordering::Relaxed) / (1024 * 1024);
                let moved = diag::memtrack::IMAGE_MOVED_BYTES.load(std::sync::atomic::Ordering::Relaxed) / (1024 * 1024);
                let tex_mb =
                    diag::IOS_TEXTURE_BYTES.load(std::sync::atomic::Ordering::Relaxed) / (1024 * 1024);
                let outside = footprint.saturating_sub(heap_reserved);
                diag::boot_crumb(&format!(
                    "mem: footprint {footprint} MiB | iOS still allows {available_mb} MiB | heap live {heap_used} MiB, reserved {heap_reserved} MiB | optional FPV retained live {fpv_retained} MiB (overlaps active assets) | textures expanded cumulative {tex_mb} MiB | Bevy image moved cumulative {moved} MiB | outside heap (footprint - heap reserved) {outside} MiB | {}",
                    diag::memtrack::snapshot_line()
                ));
                last_mb = footprint;
                last_log = std::time::Instant::now();
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    });
}

/// A fatal signal (SIGSEGV, SIGABRT, ...) is not a panic. Log which one, then die as usual.
/// SIGKILL cannot be caught: if the log just stops, that is iOS killing the app for memory.
#[cfg(target_os = "ios")]
extern "C" fn fatal_signal(signal: libc::c_int) {
    diag::boot_crumb(&format!(
        "FATAL SIGNAL {signal} (11=SEGV 6=ABRT 10=BUS 4=ILL 5=TRAP)\n{}",
        std::backtrace::Backtrace::force_capture()
    ));
    // SAFETY: restore the default action and re-raise so iOS still gets its crash report.
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

#[cfg(target_os = "ios")]
fn install_signal_logging() {
    for signal in [libc::SIGSEGV, libc::SIGABRT, libc::SIGBUS, libc::SIGILL, libc::SIGTRAP] {
        // SAFETY: installing a plain C handler at startup.
        unsafe {
            libc::signal(signal, fatal_signal as extern "C" fn(libc::c_int) as libc::sighandler_t);
        }
    }
}

/// Release name shown small in the main menu (the Cargo version is `0.4.0`).
const APP_LABEL: &str = "0.4.0";

fn main() {
    #[cfg(target_os = "ios")]
    {
        prepare_ios_sandbox();
        let docs = PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Documents");
        // Read how the last run ended before its log is moved aside: a run closed by
        // iOS leaves no exit line, and this is the only way to tell.
        let previous = std::fs::read_to_string(docs.join("iw4l-boot.log")).unwrap_or_default();
        // A run that iOS closed leaves no exit line. Keep its log under a name the next
        // launch does not overwrite (the .prev log is replaced every start).
        if !previous.is_empty() && !previous.contains("process_exit") {
            let crash = docs.join("iw4l-boot.crash.log");
            // Keep the one before it, so a second launch does not erase the evidence.
            let _ = std::fs::rename(&crash, docs.join("iw4l-boot.crash.prev.log"));
            let _ = std::fs::write(&crash, &previous);
        }
        diag::ios_env::rotate_log(&docs.join("iw4l-boot.log"));
        diag::boot_crumb("1 main entered, sandbox ready");
        diag::boot_crumb(&diag::session::previous_run_note(&previous));
        diag::memory_settings::record_app_version(APP_LABEL);
        diag::boot_crumb(&diag::ios_env::device_report(env!("CARGO_PKG_VERSION")));
        for line in diag::ios_env::apply_env_file(&docs.join("iw4l-env.txt")) {
            diag::boot_crumb(&format!("iw4l-env.txt: {line}"));
        }
        if let Some(bytes) = diag::ios_env::available_bytes() {
            diag::memory_settings::record_granted_mib(bytes / (1024 * 1024));
        }
        let settings = diag::memory_settings::get().report();
        diag::boot_crumb(&settings);
        for note in diag::memory_settings::apply_tier_defaults() {
            diag::boot_crumb(&format!("profile default: {note}"));
        }
        // Persist effective controls even if jetsam kills us before summary.json.
        if let Err(error) = std::fs::write(docs.join("iw4l-memory-settings.txt"), &settings) {
            diag::boot_crumb(&format!("memory settings report write failed: {error}"));
        }
        install_signal_logging();
        spawn_memory_logger();
        std::panic::set_hook(Box::new(|info| {
            let thread = std::thread::current();
            diag::boot_crumb(&format!(
                "PANIC on thread {:?}: {info}\n{}",
                thread.name(),
                std::backtrace::Backtrace::force_capture()
            ));
        }));
    }
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "licenses")
    {
        for (name, text) in LICENSES {
            println!("==> {name} <==\n\n{text}\n");
        }
        return;
    }
    #[cfg(target_os = "ios")]
    diag::boot_crumb("2 updater::startup");
    let mut args = match updater::startup().unwrap_or_else(|e| diag::exit_launch_error(&e)) {
        Some(args) => args,
        None => return,
    };
    if args.is_empty() {
        args.push("menu".into());
    }
    #[cfg(target_os = "ios")]
    diag::boot_crumb("3 after updater");
    bootstrap::bench::arm();
    prepare_process_root().unwrap_or_else(|e| {
        diag::exit_launch_error(&e);
    });
    let artifacts = ensure_artifacts_dir().unwrap_or_else(|e| diag::exit_launch_error(&e));
    announce_log(diag::init_log(&artifacts));
    let (mode, acceptance, cheats) = bootstrap::parse_cli(
        args.into_iter()
            .map(|arg| arg.to_string_lossy().into_owned()),
    )
    .unwrap_or_else(|e| diag::exit_launch_error(&e));
    let games = games_root_from_env().unwrap_or_else(|e| diag::exit_launch_error(&e));
    #[cfg(target_os = "ios")]
    diag::boot_crumb("4 calling bootstrap::launch");
    bootstrap::launch(games, artifacts, mode, acceptance, cheats);
}

fn prepare_process_root() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe =
            std::env::current_exe().map_err(|error| format!("cannot locate iw4l.exe: {error}"))?;
        let root = exe
            .parent()
            .ok_or_else(|| format!("iw4l.exe has no parent directory: {}", exe.display()))?;
        std::env::set_current_dir(root).map_err(|error| {
            format!(
                "cannot enter launcher directory {}: {error}",
                root.display()
            )
        })?;
    }
    Ok(())
}

fn announce_log(path: PathBuf) {
    diag::announce_log_stdout(&path, diag::latest_log_path().as_deref());
    diag::info!(Launch, "log: {}", path.display());
}
