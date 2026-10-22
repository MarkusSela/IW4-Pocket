#!/usr/bin/env python3
"""Source-level contract checks. These do NOT compile or run Rust."""
from pathlib import Path
import plistlib
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


def text(path):
    return (ROOT / path).read_text()


class Contract(unittest.TestCase):
    def test_versions_agree(self):
        cargo = re.search(r'^version = "([^"]+)"$', text("crates/launcher/Cargo.toml"), re.M).group(1)
        plist = plistlib.loads((ROOT / "ios/Info.plist").read_bytes())
        self.assertEqual(plist["CFBundleShortVersionString"], cargo.split("-", 1)[0])
        self.assertEqual(cargo, "0.3.8")
        self.assertEqual(plist["CFBundleVersion"], "22")
        # The label shown in the main menu is the release name.
        self.assertIn('const APP_LABEL: &str = "0.3.8";', text("crates/launcher/src/main.rs"))
        self.assertEqual(plist["CFBundleIdentifier"], "com.markussela.iw4l")
        lock = re.search(r'name = "launcher"\nversion = "([^"]+)"', text("Cargo.lock"))
        self.assertIsNotNone(lock)
        self.assertEqual(lock.group(1), cargo)

    def test_markers_exist_in_sources(self):
        sources = "\n".join(p.read_text() for p in (ROOT / "crates").rglob("*.rs"))
        verify = text("tools/verify_ios_ipa.py")
        for marker in re.findall(r'b"([^"]+)"', verify.split("MARKERS", 1)[1].split(")", 1)[0]):
            self.assertIn(marker, sources, marker)

    def test_env_allowlist_is_closed(self):
        src = text("crates/diag/src/memory_settings.rs")
        body = src.split("pub fn valid_file_setting", 1)[1].split("#[cfg(test)]", 1)[0]
        self.assertIn("_ => false", body)
        for name in ("IW4L_GAMES", "IW4L_ARTIFACTS_DIR"):
            self.assertNotIn(f'"{name}"', body)

    def test_ios_defaults_are_conservative(self):
        src = text("crates/diag/src/memory_settings.rs")
        self.assertIn("IW4L_FPV_RETAIN_MIB", src)
        self.assertIn("IW4L_SHADER_WORKERS", src)
        self.assertIn("Tier::pick", src)
        self.assertIn("RESIDENT_MAP_ENV", src)

    def test_workflow_verifies_before_publish(self):
        wf = text(".github/workflows/ios-release.yml")
        verify = wf.index("tools/verify_ios_ipa.py")
        self.assertLess(verify, wf.index("actions/upload-artifact"))
        self.assertLess(verify, wf.index("gh release create"))
        self.assertIn("--profile ios", wf)
        self.assertIn("$GITHUB_SHA", wf)
        # The SideStore source logic is protected by its own tests, run before the build.
        self.assertIn("python3 tools/test_gen_source.py", wf)
        self.assertLess(wf.index("python3 tools/test_gen_source.py"), wf.index("cargo build"))

    def test_no_secret_shaped_strings(self):
        pattern = re.compile(r"gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16}")
        for rel in ("tools", ".github", "crates/diag", "crates/launcher", "RELEASE_NOTES.md"):
            base = ROOT / rel
            files = [base] if base.is_file() else [p for p in base.rglob("*") if p.is_file() and p.suffix in {".py", ".yml", ".rs", ".md"}]
            for p in files:
                self.assertIsNone(pattern.search(p.read_text()), str(p))

    def test_pocket_menu_matches_the_code(self):
        """The generated menu and pocket_options.rs must list the same rows."""
        import json
        pages = json.loads(text("crates/ui/menus/pocket.json"))
        self.assertEqual([m["name"] for m in pages], ["options_pocket", "options_pocket2"])
        menu = {"item_copies": [c for m in pages for c in m["item_copies"]]}
        rust = text("crates/console/src/pocket_options.rs")
        rust_rows = re.findall(r'dvar: "(ui_pocket_[a-z]+)"', rust)
        menu_rows = [c["fields"]["dvar"] for c in menu["item_copies"]
                     if c["fields"].get("dvar", "").startswith("ui_pocket_")]
        self.assertEqual(rust_rows, menu_rows)
        for c in menu["item_copies"]:
            f = c["fields"]
            if f.get("dvar", "").startswith("ui_pocket_"):
                self.assertEqual(f["choices"][0], ["Auto", "auto"], f["dvar"])
                self.assertEqual(f["item_type"], 12)
        # Every shown value must be a key in the row's map in pocket_options.rs.
        maps = {}
        for dvar, body in re.findall(r'dvar: "(ui_pocket_[a-z]+)",.*?map: &\[(.*?)\],\n    \}', rust, re.S):
            maps[dvar] = re.findall(r'\("([a-z0-9.]+)", ', body)
        for c in menu["item_copies"]:
            f = c["fields"]
            if f.get("dvar", "").startswith("ui_pocket_"):
                self.assertEqual([v for _, v in f["choices"]], maps[f["dvar"]], f["dvar"])
        # Reached from the settings menu and loaded with the others.
        menu_rs = text("crates/ui/src/menu.rs")
        self.assertIn("../menus/pocket.json", menu_rs)
        self.assertIn("open options_pocket", menu_rs)

    def test_generated_menu_is_current(self):
        import json, subprocess, sys, tempfile, shutil
        target = ROOT / "crates/ui/menus/pocket.json"
        before = json.loads(target.read_text())
        backup = tempfile.mktemp()
        shutil.copy(target, backup)
        try:
            subprocess.run([sys.executable, "tools/gen_pocket_menu.py"], cwd=ROOT, check=True,
                           capture_output=True)
            self.assertEqual(before, json.loads(target.read_text()),
                             "pocket.json is stale: run tools/gen_pocket_menu.py")
        finally:
            shutil.copy(backup, target)

    def test_only_the_project_repository_can_be_opened(self):
        src = text("crates/bootstrap/src/open_url.rs")
        self.assertIn("https://github.com/MarkusSela/IW4-Pocket/", src)
        self.assertIn("allowed(&request.0)", src)
        report = text("crates/console/src/pocket_options.rs")
        self.assertIn("MarkusSela/IW4-Pocket/issues/new", report)

    def test_log_is_scrubbed_of_the_install_id(self):
        lib = text("crates/diag/src/lib.rs")
        body = lib.split("pub fn boot_crumb", 1)[1].split("pub fn boot_note", 1)[0]
        self.assertIn("anonymize_paths", body)

    def test_shadows_follow_the_gpu_and_the_option(self):
        plugins = text("crates/bootstrap/src/plugins.rs")
        self.assertIn("shadows_decision", plugins)
        self.assertIn("FLOAT32_FILTERABLE", plugins)
        settings = text("crates/console/src/user_settings.rs")
        self.assertIn("shadows_forced_off", settings)

    def test_r32_float_slots_are_masked_without_float_filtering(self):
        src = text("crates/render_gpu/src/drawsurf/texture_table.rs")
        self.assertIn("FLOAT32_FILTERABLE", src)
        for name in ("CODE_TEXTURE_SHADOWMAP_SUN", "CODE_TEXTURE_SHADOWMAP_SPOT", "CODE_TEXTURE_FLOATZ"):
            self.assertIn(name, src)
        self.assertIn("SpotShadowRt", src)

    def test_memory_settings_compiles_standalone_in_ci(self):
        # CI runs `rustc --edition=2024 --test crates/diag/src/memory_settings.rs` on this
        # file alone: no other module of the crate exists there (0.3.8 build failed on this).
        src = text("crates/diag/src/memory_settings.rs")
        non_test = src.split("#[cfg(test)]", 1)[0]
        code = "\n".join(line.split("//", 1)[0] for line in non_test.splitlines())
        for bad in ("crate::", "super::", "diag::", "self::"):
            self.assertNotIn(bad, code, f"{bad} is not available in the standalone CI build")
        for name in re.findall(r"^\s*(?:pub\s+)?(?:use|mod)\s+([A-Za-z_][\w:]*)", code, re.M):
            self.assertTrue(name.startswith("std"), f"external use/mod in standalone file: {name}")
        detail = text("crates/diag/src/detail.rs")
        env = re.search(r'pub const DIAG_ENV: &str = "([^"]+)";', detail).group(1)
        self.assertIn(f'"{env}" => matches!(value, "normal" | "detailed")', code)


if __name__ == "__main__":
    unittest.main(verbosity=2)
