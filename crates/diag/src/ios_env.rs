//! iOS-only helpers that the rest of the engine can call on every platform (on other
//! platforms they return `None` or do nothing, so callers need no `cfg`).
//!
//! * memory probes: the footprint iOS enforces, and the malloc heap in use
//! * device facts written once at the top of the log
//! * `Documents/iw4l-env.txt`: `NAME=value` lines applied as environment variables,
//!   so the engine's own switches (cache budgets, sound, present mode...) can be tried
//!   on a phone without a rebuild
//! * log rotation and a noise filter, so `iw4l-boot.log` stays readable

#[cfg(target_os = "ios")]
mod apple {
    #[repr(C)]
    #[derive(Default)]
    pub struct TaskVmInfo {
        pub virtual_size: u64,
        pub region_count: i32,
        pub page_size: i32,
        pub resident_size: u64,
        pub resident_size_peak: u64,
        pub device: u64,
        pub device_peak: u64,
        pub internal: u64,
        pub internal_peak: u64,
        pub external: u64,
        pub external_peak: u64,
        pub reusable: u64,
        pub reusable_peak: u64,
        pub purgeable_volatile_pmap: u64,
        pub purgeable_volatile_resident: u64,
        pub purgeable_volatile_virtual: u64,
        pub compressed: u64,
        pub compressed_peak: u64,
        pub compressed_lifetime: u64,
        pub phys_footprint: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct MallocStats {
        pub blocks_in_use: libc::c_uint,
        pub size_in_use: libc::size_t,
        pub max_size_in_use: libc::size_t,
        pub size_allocated: libc::size_t,
    }

    unsafe extern "C" {
        pub fn malloc_zone_statistics(zone: *mut libc::c_void, stats: *mut MallocStats);
        pub fn os_proc_available_memory() -> usize;
        pub fn sysctlbyname(
            name: *const libc::c_char,
            oldp: *mut libc::c_void,
            oldlenp: *mut libc::size_t,
            newp: *mut libc::c_void,
            newlen: libc::size_t,
        ) -> libc::c_int;
    }

    pub fn sysctl_string(name: &str) -> Option<String> {
        let cname = std::ffi::CString::new(name).ok()?;
        let mut len: libc::size_t = 0;
        // SAFETY: first call only asks for the size.
        let rc = unsafe {
            sysctlbyname(cname.as_ptr(), std::ptr::null_mut(), &raw mut len, std::ptr::null_mut(), 0)
        };
        if rc != 0 || len == 0 || len > 256 {
            return None;
        }
        let mut buf = vec![0u8; len];
        // SAFETY: buffer is `len` bytes long.
        let rc = unsafe {
            sysctlbyname(cname.as_ptr(), buf.as_mut_ptr().cast(), &raw mut len, std::ptr::null_mut(), 0)
        };
        if rc != 0 {
            return None;
        }
        buf.truncate(len);
        while buf.last() == Some(&0) {
            buf.pop();
        }
        String::from_utf8(buf).ok()
    }

    pub fn sysctl_u64(name: &str) -> Option<u64> {
        let cname = std::ffi::CString::new(name).ok()?;
        let mut value: u64 = 0;
        let mut len: libc::size_t = size_of::<u64>();
        // SAFETY: `value` is 8 bytes and `len` says so.
        let rc = unsafe {
            sysctlbyname(
                cname.as_ptr(),
                (&raw mut value).cast(),
                &raw mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        (rc == 0).then_some(value)
    }
}

/// The footprint iOS enforces (GPU-backed and compressed memory included), in bytes.
pub fn footprint_bytes() -> Option<u64> {
    #[cfg(target_os = "ios")]
    {
        const TASK_VM_INFO: libc::c_int = 22;
        let mut info = apple::TaskVmInfo::default();
        let mut count =
            (size_of::<apple::TaskVmInfo>() / size_of::<u32>()) as libc::mach_msg_type_number_t;
        // SAFETY: the struct mirrors the kernel's task_vm_info prefix; `count` bounds the write.
        #[allow(deprecated)]
        let kr = unsafe {
            libc::task_info(
                libc::mach_task_self(),
                TASK_VM_INFO as _,
                (&raw mut info).cast(),
                &raw mut count,
            )
        };
        (kr == 0).then_some(info.phys_footprint)
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

/// " | footprint N MiB (heap N MiB)" for lines that mark a life event (spawn, killcam), so the
/// log shows what each event costs. Empty off iOS.
pub fn event_memory_tag() -> String {
    const MIB: u64 = 1024 * 1024;
    match (footprint_bytes(), heap_in_use_bytes()) {
        (Some(foot), Some(heap)) => format!(" | footprint {} MiB, heap {} MiB", foot / MIB, heap / MIB),
        (Some(foot), None) => format!(" | footprint {} MiB", foot / MIB),
        _ => String::new(),
    }
}

/// (malloc bytes in use, malloc bytes reserved), over all zones.
pub fn heap_bytes() -> Option<(u64, u64)> {
    #[cfg(target_os = "ios")]
    {
        let mut stats = apple::MallocStats::default();
        // SAFETY: a null zone asks for totals over all zones; the struct matches the ABI.
        unsafe { apple::malloc_zone_statistics(std::ptr::null_mut(), &raw mut stats) };
        Some((stats.size_in_use as u64, stats.size_allocated as u64))
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

pub fn heap_in_use_bytes() -> Option<u64> {
    heap_bytes().map(|(used, _)| used)
}

/// Bytes iOS still lets this process allocate.
pub fn available_bytes() -> Option<u64> {
    #[cfg(target_os = "ios")]
    {
        // SAFETY: no arguments, returns a byte count (iOS 13+).
        Some(unsafe { apple::os_proc_available_memory() } as u64)
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

/// Multi-line device description for the top of the log.
pub fn device_report(app_version: &str) -> String {
    #[cfg(target_os = "ios")]
    {
        let model = apple::sysctl_string("hw.machine").unwrap_or_else(|| "?".into());
        let os_build = apple::sysctl_string("kern.osversion").unwrap_or_else(|| "?".into());
        let os_release = apple::sysctl_string("kern.osrelease").unwrap_or_else(|| "?".into());
        let ram = apple::sysctl_u64("hw.memsize").map_or(0, |b| b / (1024 * 1024));
        let cpus = apple::sysctl_u64("hw.ncpu").unwrap_or(0);
        let avail = available_bytes().map_or(0, |b| b / (1024 * 1024));
        format!(
            "device: model={model} ram={ram}MB cpus={cpus} | darwin={os_release} build={os_build} | app={app_version} | iOS grants {avail}MB at launch"
        )
    }
    #[cfg(not(target_os = "ios"))]
    {
        format!("device: non-iOS | app={app_version}")
    }
}

/// Apply `Documents/iw4l-env.txt`: one `NAME=value` per line, `#` starts a comment.
/// Only validated, allowlisted tuning controls are accepted. Returns diagnostics.
/// Call before any thread is spawned.
pub fn apply_env_file(path: &std::path::Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut applied = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            applied.push("ignored malformed setting (expected NAME=value)".into());
            continue;
        };
        let (name, value) = (name.trim(), value.trim());
        if !crate::memory_settings::valid_file_setting(name, value) {
            applied.push(format!("ignored unknown or invalid tuning setting: {name}"));
            continue;
        }
        // SAFETY: documented precondition: called at startup before other threads exist.
        unsafe { std::env::set_var(name, value) };
        applied.push(format!("{name}={value}"));
    }
    // IW4L_DIAG may just have been set: read it now, not on the first log line.
    crate::detail::reload_from_env();
    applied.push(crate::detail::config_line());
    applied
}

/// Rewrites `iw4l-env.txt` text for IW4 Pocket options. Each `(name, value)` is set
/// (`Some`) or removed (`None`, meaning Auto). Every other line, comments and blank
/// lines included, is kept as it was, so a hand-edited file is not lost.
pub fn merge_env_lines(existing: &str, updates: &[(&str, Option<&str>)]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut written = vec![false; updates.len()];
    for line in existing.lines() {
        let name = line.split_once('=').map(|(n, _)| n.trim());
        match name.and_then(|n| updates.iter().position(|(un, _)| *un == n)) {
            Some(i) if !line.trim_start().starts_with('#') => {
                // First occurrence is replaced (or dropped), later duplicates are dropped.
                if !written[i] {
                    written[i] = true;
                    if let Some(value) = updates[i].1 {
                        out.push(format!("{}={value}", updates[i].0));
                    }
                }
            }
            _ => out.push(line.to_owned()),
        }
    }
    for (i, (name, value)) in updates.iter().enumerate() {
        if let (false, Some(value)) = (written[i], value) {
            out.push(format!("{name}={value}"));
        }
    }
    let mut text = out.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

/// The value a setting has in `iw4l-env.txt` text, `None` when absent (Auto).
pub fn env_file_value(text: &str, name: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .find(|(n, _)| n.trim() == name)
        .map(|(_, v)| v.trim().to_owned())
}

/// Move an existing log aside so a crash does not lose it when the app is reopened.
pub fn rotate_log(path: &std::path::Path) {
    if path.exists() {
        let prev = path.with_file_name("iw4l-boot.prev.log");
        let _ = std::fs::remove_file(&prev);
        let _ = std::fs::rename(path, prev);
    }
}

/// True for lines that bury the useful ones (the menu catalog logs one per material).
pub fn is_log_noise(message: &str) -> bool {
    crate::detail::classify(message) == crate::detail::EventClass::Noise
}

/// Replaces the per-install app folder (it contains a unique install id) with
/// `<app>`, so a log shared in public does not identify the installation. Model,
/// RAM, iOS and app version stay: they are what a bug report needs.
///
/// `home` is this process's `$HOME`. The scrubber also catches the same folder
/// written with or without `/private` and any other `.../Application/<id>` path
/// (the log names a second candidate games folder).
pub fn anonymize_paths(message: &str, home: Option<&str>) -> String {
    let mut out = message.to_owned();
    if let Some(home) = home.filter(|h| h.len() > 8) {
        out = out.replace(home, "<app>");
        if let Some(stripped) = home.strip_prefix("/private") {
            out = out.replace(stripped, "<app>");
        }
    }
    scrub_application_ids(&out)
}

fn scrub_application_ids(text: &str) -> String {
    const MARK: &str = "/Containers/Data/Application/";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(MARK) {
        // Walk back over the folder prefix (`/private/var/mobile` or `/var/mobile`).
        let head = &rest[..at];
        let keep = head
            .rfind(|c: char| {
                c.is_whitespace() || matches!(c, '`' | '"' | '\'' | '(' | '[' | ':' | ',' | '=' | '|' | ';')
            })
            .map_or(0, |i| i + head[i..].chars().next().map_or(1, char::len_utf8));
        out.push_str(&rest[..keep]);
        out.push_str("<app>");
        let after = &rest[at + MARK.len()..];
        let id_len = after
            .find(|c: char| c.is_whitespace() || matches!(c, '/' | '`' | '"' | '\'' | ')' | ','))
            .unwrap_or(after.len());
        rest = &after[id_len..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "83CD7741-8794-4B03-8820-91A45176F6CC";

    #[test]
    fn home_is_replaced() {
        let home = format!("/private/var/mobile/Containers/Data/Application/{ID}");
        let line = format!("log: {home}/Documents/iw4l-artifacts/logs/1.log");
        let got = anonymize_paths(&line, Some(&home));
        assert_eq!(got, "log: <app>/Documents/iw4l-artifacts/logs/1.log");
        assert!(!got.contains(ID));
    }

    #[test]
    fn other_application_ids_are_scrubbed() {
        let line = format!("candidates: /private/var/mobile/Containers/Data/Application/{ID}/Documents/Games, `/var/mobile/Containers/Data/Application/{ID}`");
        let got = anonymize_paths(&line, None);
        assert!(!got.contains(ID), "{got}");
        assert!(got.contains("<app>/Documents/Games"));
    }

    #[test]
    fn device_line_keeps_model_and_versions() {
        let line = "device: model=iPhone14,3 ram=5639MB | darwin=27.2.0 | app=0.3.3";
        assert_eq!(anonymize_paths(line, Some("/private/var/x/y")), line);
    }

    #[test]
    fn merge_sets_replaces_and_removes_only_named_lines() {
        let before = "# my notes\nIW4L_SOUND=off\nIW4L_TIER=low\n\nIW4L_FRAME_PACE=30\n";
        let after = merge_env_lines(
            before,
            &[
                ("IW4L_TIER", Some("high")),
                ("IW4L_FRAME_PACE", None),
                ("IW4L_PROMOTION", Some("0")),
            ],
        );
        assert_eq!(
            after,
            "# my notes\nIW4L_SOUND=off\nIW4L_TIER=high\n\nIW4L_PROMOTION=0\n"
        );
    }

    #[test]
    fn merge_keeps_commented_out_setting_and_drops_duplicates() {
        let before = "#IW4L_TIER=low\nIW4L_TIER=mid\nIW4L_TIER=high\n";
        let after = merge_env_lines(before, &[("IW4L_TIER", Some("low"))]);
        assert_eq!(after, "#IW4L_TIER=low\nIW4L_TIER=low\n");
    }

    #[test]
    fn merge_into_empty_and_back_to_auto() {
        assert_eq!(merge_env_lines("", &[("IW4L_TIER", Some("mid"))]), "IW4L_TIER=mid\n");
        assert_eq!(merge_env_lines("IW4L_TIER=mid\n", &[("IW4L_TIER", None)]), "");
    }

    #[test]
    fn env_file_value_reads_only_live_lines() {
        let text = "#IW4L_TIER=low\n IW4L_TIER = high \nIW4L_SOUND=on\n";
        assert_eq!(env_file_value(text, "IW4L_TIER").as_deref(), Some("high"));
        assert_eq!(env_file_value(text, "IW4L_FRAME_PACE"), None);
    }

    #[test]
    fn noisy_lines_are_filtered() {
        assert!(is_log_noise("destructible death husk x: unresolved"));
        assert!(is_log_noise("world report: destructible death toy_a: clip=absent"));
        assert!(is_log_noise("static model mesh: foo (2 surfaces)"));
        assert!(!is_log_noise("static models: 3356 authored slots"));
    }
}
