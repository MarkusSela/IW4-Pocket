//! "IW4 Pocket options": the phone-only choices (device profile, texture size, frame
//! settings, shadows) and the "Send logs" button.
//!
//! Every choice is kept in `Documents/iw4l-env.txt`, the file the launcher reads before
//! graphics start, so a choice applies after the app is restarted. "Auto" removes the
//! line. Lines this menu does not own, comments included, are left alone.
//!
//! Nothing here touches the game's own settings (`settings.cfg`).
use std::path::PathBuf;

use bevy::prelude::*;

/// Asks the platform layer to open a web page (iOS: Safari). Handled in `bootstrap`.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct OpenExternalUrl(pub String);

/// One menu row: the dvar the row edits, the `iw4l-env.txt` name it controls, and what
/// each shown value writes (`None` = remove the line = Auto).
pub(crate) struct Opt {
    pub dvar: &'static str,
    pub env: &'static str,
    pub map: &'static [(&'static str, Option<&'static str>)],
}

/// Keep in step with `OPTIONS` in tools/gen_pocket_menu.py (a test checks this).
pub(crate) const OPTIONS: &[Opt] = &[
    Opt {
        dvar: "ui_pocket_tier",
        env: diag::memory_settings::TIER_ENV,
        map: &[("auto", None), ("low", Some("low")), ("mid", Some("mid")), ("high", Some("high"))],
    },
    Opt {
        dvar: "ui_pocket_scale",
        env: "IW4L_RENDER_SCALE",
        map: &[
            ("auto", None),
            ("0.2", Some("0.2")),
            ("0.3", Some("0.3")),
            ("0.4", Some("0.4")),
            ("0.5", Some("0.5")),
            ("0.67", Some("0.67")),
            ("0.75", Some("0.75")),
            ("0.85", Some("0.85")),
        ],
    },
    Opt {
        dvar: "ui_pocket_pace",
        env: "IW4L_FRAME_PACE",
        map: &[("auto", None), ("30", Some("30")), ("40", Some("40")), ("60", Some("60"))],
    },
    Opt {
        dvar: "ui_pocket_texture",
        env: diag::memory_settings::TEXTURE_CAP_ENV,
        map: &[
            ("auto", None),
            ("128", Some("128")),
            ("256", Some("256")),
            ("512", Some("512")),
            ("1024", Some("1024")),
        ],
    },
    Opt {
        dvar: "ui_pocket_lean",
        env: "IW4L_LEAN_SHADERS",
        map: &[("auto", None), ("on", Some("1")), ("off", Some("0"))],
    },
    Opt {
        dvar: "ui_pocket_camo",
        env: "IW4L_SKIP_CAMO",
        map: &[("auto", None), ("skip", Some("1")), ("keep", Some("0"))],
    },
    Opt {
        dvar: "ui_pocket_latency",
        env: "IW4L_FRAME_LATENCY",
        map: &[("auto", None), ("1", Some("1")), ("2", Some("2"))],
    },
    Opt {
        dvar: "ui_pocket_promotion",
        env: "IW4L_PROMOTION",
        map: &[("auto", None), ("on", Some("1")), ("off", Some("0"))],
    },
    Opt {
        dvar: "ui_pocket_promodelay",
        env: "IW4L_PROMOTION_DELAY",
        map: &[("auto", None), ("0", Some("0")), ("30", Some("30")), ("60", Some("60")), ("120", Some("120"))],
    },
    Opt {
        dvar: "ui_pocket_bc",
        env: diag::memory_settings::IOS_BC_ENV,
        map: &[("auto", None), ("off", Some("0"))],
    },
    Opt {
        dvar: "ui_pocket_shadows",
        env: diag::memory_settings::SHADOWS_ENV,
        map: &[("auto", None), ("off", Some("0")), ("on", Some("1"))],
    },
    Opt {
        dvar: "ui_pocket_clip",
        env: "IW4L_CLIP_RING",
        map: &[("auto", None), ("on", Some("1")), ("off", Some("0"))],
    },
    Opt {
        dvar: "ui_pocket_touch",
        env: "IW4L_TOUCH_CONTROLS",
        map: &[("auto", None), ("on", Some("1")), ("off", Some("0"))],
    },
    Opt {
        dvar: "ui_pocket_diag",
        env: diag::detail::DIAG_ENV,
        map: &[("auto", None), ("normal", Some("normal")), ("detailed", Some("detailed"))],
    },
    // Env names are literals: bootstrap's TEXT_SHARPNESS_ENV is pub(crate) and
    // assets::weapon_residency::config::ENV_VAR is not a console dependency to keep in sync
    // by type; test_pocket_menu_matches_the_code and the allowlist tests pin the strings.
    Opt {
        dvar: "ui_pocket_sharp",
        env: "IW4L_TEXT_SHARPNESS",
        map: &[("auto", None), ("off", Some("off")), ("native", Some("native")), ("high", Some("high")), ("medium", Some("medium"))],
    },
    Opt {
        dvar: "ui_pocket_weapons",
        env: "IW4L_SELECTIVE_WEAPONS",
        map: &[("auto", None), ("on", Some("on")), ("off", Some("off"))],
    },
];

/// The menu value for what `iw4l-env.txt` currently holds. A value the menu cannot show
/// (a hand-edited one) reads as Auto and is left in the file until the row is changed.
pub(crate) fn menu_value(opt: &Opt, file_value: Option<&str>) -> &'static str {
    match file_value {
        None => "auto",
        Some(v) => opt
            .map
            .iter()
            .find(|(_, env)| *env == Some(v))
            .map_or("auto", |(shown, _)| *shown),
    }
}

/// What a menu value writes: `Some(None)` removes the line, `Some(Some(v))` sets it,
/// `None` means the menu value is not one of this row's choices.
pub(crate) fn env_value(opt: &Opt, shown: &str) -> Option<Option<&'static str>> {
    opt.map.iter().find(|(s, _)| *s == shown).map(|(_, env)| *env)
}

/// Keeps the bot count in `iw4l-env.txt` (iOS), so the next launch starts with it.
pub(crate) fn save_bots(n: u32) {
    let Some(path) = env_path() else { return };
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let value = (n > 0).then(|| n.to_string());
    let updated = diag::ios_env::merge_env_lines(&existing, &[("IW4L_BOTS", value.as_deref())]);
    if updated != existing && let Err(error) = std::fs::write(&path, updated) {
        diag::warn!(Console, "pocket options: could not save {}: {error}", path.display());
    }
}

fn env_path() -> Option<PathBuf> {
    if !cfg!(target_os = "ios") {
        return None;
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Documents").join("iw4l-env.txt"))
}

fn boot_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Documents").join("iw4l-boot.log"))
}

const SEND_WARNING: &str = "^3Press again to open GitHub.^7 Nothing is sent until you submit.";
const RESTART_HINT: &str = "^3Restart the app to apply your changes.";
const DEFAULT_HINT: &str = "Auto picks the best value for your phone.";

#[derive(Default)]
pub(crate) struct PocketState {
    started: bool,
    launched: Vec<&'static str>,
    current: Vec<&'static str>,
    armed: bool,
    shown_hint: String,
}

pub(crate) fn pocket_options_menu(
    mut events: MessageReader<crate::ConsoleCommand>,
    mut dvars: ResMut<frame::UiMenuDvars>,
    mut state: Local<PocketState>,
    mut open: MessageWriter<OpenExternalUrl>,
) {
    if !state.started {
        state.started = true;
        let text = env_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        for opt in OPTIONS {
            let shown = menu_value(opt, diag::ios_env::env_file_value(&text, opt.env).as_deref());
            dvars.set(opt.dvar, shown);
            state.launched.push(shown);
            state.current.push(shown);
        }
        dvars.set("ui_pocket_info", info_line());
        let version = diag::memory_settings::app_version();
        if !version.is_empty() {
            dvars.set("ui_pocket_version", format!("IW4 Pocket {version}"));
        }
    }
    for command in events.read() {
        if !matches!(command.name.as_str(), "set" | "seta") {
            continue;
        }
        let [name, value, ..] = command.args.as_slice() else {
            continue;
        };
        if name.eq_ignore_ascii_case("ui_pocket_send_logs") {
            if value == "1" {
                if state.armed {
                    state.armed = false;
                    open.write(OpenExternalUrl(report_url()));
                } else {
                    state.armed = true;
                }
            }
            continue;
        }
        let Some(at) = OPTIONS.iter().position(|o| o.dvar.eq_ignore_ascii_case(name)) else {
            continue;
        };
        let opt = &OPTIONS[at];
        let Some(env) = env_value(opt, value) else {
            continue;
        };
        if let Some(path) = env_path() {
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            let updated = diag::ios_env::merge_env_lines(&existing, &[(opt.env, env)]);
            if updated != existing && let Err(error) = std::fs::write(&path, updated) {
                diag::warn!(Console, "pocket options: could not save {}: {error}", path.display());
            }
        }
        state.current[at] = menu_value(opt, env);
        // Leaving the send button armed would let a stray press open the browser.
        state.armed = false;
    }
    let hint = if state.armed {
        SEND_WARNING
    } else if state.current != state.launched {
        RESTART_HINT
    } else {
        DEFAULT_HINT
    };
    if state.shown_hint != hint {
        state.shown_hint = hint.to_owned();
        dvars.set("ui_pocket_hint", hint);
    }
}

fn info_line() -> String {
    let settings = diag::memory_settings::get();
    let granted = diag::memory_settings::granted_mib();
    if granted > 0 {
        format!("iOS gives this app {granted} MB. Profile in use: {}.", settings.tier.name())
    } else {
        format!("Profile in use: {}.", settings.tier.name())
    }
}

// ---- the problem report ------------------------------------------------------

const REPO_NEW_ISSUE: &str = "https://github.com/MarkusSela/IW4-Pocket/issues/new";
/// GitHub refuses very long links (about 8 KB); the encoded body stays well under.
const ENCODED_BODY_LIMIT: usize = 5500;

/// Builds the pre-filled GitHub link from the boot log. Only the short summary below is
/// put in the link; the log files are attached by hand (the body says how).
pub(crate) fn report_url() -> String {
    let log = boot_log_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    build_report_url(&log)
}

pub(crate) fn build_report_url(log: &str) -> String {
    let field = |prefix: &str| {
        log.lines()
            .rev()
            .find(|line| line.starts_with(prefix))
            .map(str::trim)
            .unwrap_or("")
    };
    let device = field("device:");
    let app = device
        .split(" | ")
        .find_map(|part| part.trim().strip_prefix("app="))
        .unwrap_or("?");
    let mut summary: Vec<String> = Vec::new();
    for prefix in [
        "device:",
        "device tier:",
        "gpu:",
        "previous run:",
        "session summary:",
        "texture cap:",
        "window:",
    ] {
        let line = field(prefix);
        if !line.is_empty() {
            summary.push(format!("- {line}"));
        }
    }
    if let Some(at) = log.lines().position(|line| line.starts_with("RENDER ERROR")) {
        let error: Vec<&str> = log.lines().skip(at).take(5).map(str::trim).collect();
        summary.push(format!("- {}", error.join(" ")));
    }
    let last_mem = field("mem: footprint");
    if !last_mem.is_empty() {
        let short: Vec<&str> = last_mem.split(" | ").take(2).collect();
        summary.push(format!("- last sample: {}", short.join(" | ")));
    }
    let mut body = String::from(
        "What happened (map, what you were doing):\n\n\nHow did you install it (Xcode, SideStore, TrollStore, other)?\n\n\n---\nAutomatic summary (no personal data):\n",
    );
    body.push_str(&summary.join("\n"));
    body.push_str(
        "\n\nPlease attach `iw4l-boot.log` and `iw4l-memory-settings.txt` (Files app > On My iPhone > IW4 Pocket). If the app was closed by iOS, also attach `iw4l-boot.crash.log`.\n",
    );
    // The limit that matters is the encoded size: every non-ASCII or punctuation byte
    // becomes three characters. Trim the summary (never the title) until it fits.
    while percent_encode(&body).len() > ENCODED_BODY_LIMIT && body.len() > 200 {
        let mut cut = body.len() - body.len() / 10;
        while !body.is_char_boundary(cut) {
            cut -= 1;
        }
        body.truncate(cut);
    }
    format!(
        "{REPO_NEW_ISSUE}?title={}&body={}",
        percent_encode(&format!("Problem report (app {app})")),
        percent_encode(&body)
    )
}

/// RFC 3986 unreserved characters pass; everything else becomes `%XX`.
pub(crate) fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(dvar: &str) -> &'static Opt {
        OPTIONS.iter().find(|o| o.dvar == dvar).unwrap()
    }

    #[test]
    fn every_row_starts_with_auto_and_writes_valid_values() {
        for o in OPTIONS {
            assert_eq!(o.map[0], ("auto", None), "{} must start with Auto", o.dvar);
            for (shown, env) in o.map {
                if let Some(value) = env {
                    assert!(
                        diag::memory_settings::valid_file_setting(o.env, value),
                        "{}: {shown} -> {}={value} would be rejected at launch",
                        o.dvar,
                        o.env
                    );
                }
            }
        }
    }

    #[test]
    fn menu_and_file_round_trip() {
        let o = opt("ui_pocket_promotion");
        assert_eq!(menu_value(o, None), "auto");
        assert_eq!(menu_value(o, Some("0")), "off");
        assert_eq!(menu_value(o, Some("1")), "on");
        assert_eq!(menu_value(o, Some("junk")), "auto");
        assert_eq!(env_value(o, "off"), Some(Some("0")));
        assert_eq!(env_value(o, "auto"), Some(None));
        assert_eq!(env_value(o, "sideways"), None);
    }

    #[test]
    fn changing_one_row_keeps_the_rest_of_the_file() {
        let o = opt("ui_pocket_tier");
        let before = "# keep\nIW4L_SOUND=off\n";
        let after = diag::ios_env::merge_env_lines(
            before,
            &[(o.env, env_value(o, "low").unwrap())],
        );
        assert_eq!(after, "# keep\nIW4L_SOUND=off\nIW4L_TIER=low\n");
        let back = diag::ios_env::merge_env_lines(&after, &[(o.env, env_value(o, "auto").unwrap())]);
        assert_eq!(back, before);
    }

    #[test]
    fn diagnostics_row_writes_only_valid_levels() {
        let o = OPTIONS.iter().find(|o| o.dvar == "ui_pocket_diag").unwrap();
        assert_eq!(env_value(o, "auto"), Some(None));
        assert_eq!(env_value(o, "detailed"), Some(Some("detailed")));
        for (_, value) in o.map.iter().filter_map(|(s, v)| v.map(|v| (s, v))) {
            assert!(diag::memory_settings::valid_file_setting(o.env, value), "{value}");
        }
    }

    #[test]
    fn percent_encoding_is_strict() {
        assert_eq!(percent_encode("a b&c=d/é"), "a%20b%26c%3Dd%2F%C3%A9");
        assert_eq!(percent_encode("Az09-_.~"), "Az09-_.~");
    }

    #[test]
    fn report_url_carries_the_summary_and_no_install_path() {
        let log = "1 main entered\n\
device: model=iPhone14,3 ram=5639MB cpus=6 | darwin=27.2.0 build=24B5089g | app=0.3.3 | iOS grants 3069MB at launch\n\
device tier: low | memory settings: x\n\
gpu: adapter=Apple A15 GPU | bc textures=no | float32 filtering=no | texture arrays=yes\n\
RENDER ERROR Validation: boom\n\
\n\
Caused by:\n  In Device::create_bind_group\n\
mem: footprint 2906 MiB | iOS still allows 165 MiB | heap live 1 MiB\n";
        let url = build_report_url(log);
        assert!(url.starts_with("https://github.com/MarkusSela/IW4-Pocket/issues/new?title="));
        assert!(url.contains("app%200.3.3"));
        assert!(url.contains("iPhone14%2C3"));
        assert!(url.contains("float32%20filtering%3Dno"));
        assert!(url.contains("RENDER%20ERROR"));
        assert!(!url.contains("Application"));
        assert!(url.len() < 7000, "{}", url.len());
    }

    #[test]
    fn report_url_without_a_log_still_works() {
        let url = build_report_url("");
        assert!(url.contains("title=Problem%20report%20%28app%20%3F%29"));
    }

    #[test]
    fn report_url_is_capped_even_for_text_that_encodes_large() {
        let plain = format!("device: {}\n", "x".repeat(20000));
        assert!(build_report_url(&plain).len() < 7000);
        let heavy = format!("device: {}\n", "é&=/ ".repeat(8000));
        let url = build_report_url(&heavy);
        assert!(url.len() < 7000, "{}", url.len());
        assert!(url.starts_with("https://github.com/MarkusSela/IW4-Pocket/issues/new?title="));
    }
}
