"""Offline source contract tests; deterministic IPA fixtures contain only Info.plist."""
import contextlib
import hashlib
import io
import json
import plistlib
import unittest
import zipfile

import gen_source as source


def ipa(version, build, bundle="com.markussela.iw4l"):
    data = io.BytesIO()
    with zipfile.ZipFile(data, "w") as archive:
        archive.writestr(zipfile.ZipInfo("Payload/IW4L.app/Info.plist"), plistlib.dumps({
            "CFBundleIdentifier": bundle, "CFBundleShortVersionString": version,
            "CFBundleVersion": build, "MinimumOSVersion": "15.0",
        }))
    return data.getvalue()


def release(tag, test=False):
    return {"tag_name": tag, "prerelease": test, "draft": False,
            "published_at": "2026-10-17T11:59:52Z", "body": "# Release\nNotes",
            "assets": [{"name": "IW4.Pocket.ipa",
                        "browser_download_url": f"https://example.test/{tag}/IW4.Pocket.ipa"}]}


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.blobs = {}

    def fixture(self, tag, version, build, test=False, bundle=source.BUNDLE):
        rel = release(tag, test)
        data = ipa(version, build, bundle)
        asset = rel["assets"][0]
        asset.update(size=len(data), digest="sha256:" + hashlib.sha256(data).hexdigest())
        self.blobs[asset["browser_download_url"]] = data
        return rel

    def generate(self, releases):
        return source.build_source(releases, load_bytes=self.blobs.__getitem__)["apps"][0]

    def test_every_verified_release_is_listed_newest_first_by_ipa_identity(self):
        releases = [self.fixture("v0.3.7b", "0.3.7", "20", True),
                    self.fixture("v0.3.6", "0.3.6", "18"),
                    self.fixture("v0.3.7c", "0.3.7", "21", True)]
        result = source.build_source(releases, load_bytes=self.blobs.__getitem__)
        self.assertEqual(result["identifier"], "com.markussela.iw4l.source")
        self.assertEqual(len(result["apps"]), 1)
        self.assertNotIn("version", result)  # Do not opt into source schema v2.
        app = result["apps"][0]
        self.assertEqual(app["bundleIdentifier"], "com.markussela.iw4l")
        self.assertNotIn("releaseChannels", app)  # nothing hides behind a beta switch
        self.assertEqual([(v["version"], v["buildVersion"]) for v in app["versions"]],
                         [("0.3.7", "21"), ("0.3.7", "20"), ("0.3.6", "18")])
        newest = app["versions"][0]
        self.assertIn("TEST 0.3.7c", newest["localizedDescription"])
        self.assertIn("v0.3.7c/", newest["downloadURL"])
        self.assertNotIn("TEST", app["versions"][2]["localizedDescription"])
        self.assertEqual(newest["minOSVersion"], "15.0")
        self.assertEqual(newest["sha256"], releases[2]["assets"][0]["digest"][7:])

    def test_test_description_does_not_repeat_the_warning_twice(self):
        rel = self.fixture("v0.3.9", "0.3.9", "23", True)
        rel["body"] = ("# IW4 Pocket 0.3.9\n\n**Experimental prerelease. Not verified on a phone.** "
                       "Stability fixes; the memory crash is still unproven.")
        text = self.generate([rel])["versions"][0]["localizedDescription"]
        self.assertEqual(text.count("Experimental prerelease."), 1, text)
        self.assertEqual(text.count("Not verified on a phone."), 1, text)
        self.assertTrue(text.endswith("Stability fixes; the memory crash is still unproven."), text)
        self.assertEqual(source.strip_repeated_warning("Experimental prerelease."), "Experimental prerelease.")
        self.assertEqual(source.strip_repeated_warning("Menus changed."), "Menus changed.")

    def test_the_token_goes_to_the_github_api_and_nowhere_else(self):
        api = source.api_headers("https://api.github.com/repos/x/y/releases", token="secret")
        self.assertEqual(api["Authorization"], "Bearer secret")
        # IPA downloads redirect to other hosts: no credentials there.
        self.assertEqual(source.api_headers("https://github.com/x/y/releases/download/v1/a.ipa", token="secret"), {})
        self.assertEqual(source.api_headers("https://objects.example/a.ipa", token="secret"), {})
        self.assertEqual(source.api_headers("https://api.github.com/x", token=""), {})

    def test_all_pages_of_releases_are_read(self):
        pages = {1: [{"n": i} for i in range(100)], 2: [{"n": i} for i in range(100, 130)]}
        seen = []
        def fetch(url):
            seen.append(url)
            return pages[int(url.rsplit("page=", 1)[1])]
        self.assertEqual(len(source.get_all_releases(fetch)), 130)
        self.assertEqual(len(seen), 2, "stops at the first short page")

    def test_require_release_fails_when_the_new_release_is_missing(self):
        rel = self.fixture("v0.3.9", "0.3.9", "23", True)
        # A real GitHub asset URL carries /download/<tag>/, which is what require_release reads.
        url = "https://github.com/MarkusSela/IW4-Pocket/releases/download/v0.3.9/IW4.Pocket.ipa"
        self.blobs[url] = self.blobs.pop(rel["assets"][0]["browser_download_url"])
        rel["assets"][0]["browser_download_url"] = url
        result = source.build_source([rel], load_bytes=self.blobs.__getitem__)
        source.require_release(result, "v0.3.9")
        with self.assertRaises(ValueError):
            source.require_release(result, "v0.4.0")

    def test_stable_and_test_releases_both_appear_even_when_stable_is_newer(self):
        releases = [self.fixture("v0.3.7c", "0.3.7", "21", True),
                    self.fixture("v0.3.8", "0.3.8", "22")]
        app = self.generate(releases)
        self.assertEqual([v["buildVersion"] for v in app["versions"]], ["22", "21"])

    def test_drafts_missing_ipa_and_unloadable_old_releases_are_not_listed(self):
        stable = self.fixture("v0.3.6", "0.3.6", "18")
        stale = [release("v0.3.4"), release("v0.3.5c", True)]  # IPA cannot be loaded
        draft = release("v0.3.9", True)
        draft["draft"] = True
        missing = release("v0.4.0")
        missing["assets"] = []
        with contextlib.redirect_stderr(io.StringIO()) as warnings:
            app = self.generate(stale + [draft, missing, stable])
        self.assertEqual([v["version"] for v in app["versions"]], ["0.3.6"])
        self.assertIn("Skipping unverified", warnings.getvalue())

    def test_invalid_test_is_skipped_and_stable_stays(self):
        stable = self.fixture("v0.3.6", "0.3.6", "18")
        invalid = self.fixture("v0.3.7c", "0.3.7", "21", True, "wrong.bundle")
        with contextlib.redirect_stderr(io.StringIO()) as warnings:
            app = self.generate([stable, invalid])
        self.assertIn("Skipping unverified TEST v0.3.7c", warnings.getvalue())
        self.assertEqual([v["version"] for v in app["versions"]], ["0.3.6"])

    def test_without_stable_tests_are_ordered_by_numeric_build_not_tag(self):
        releases = [self.fixture("v0.3.7z", "0.3.7", "9", True),
                    self.fixture("v0.3.7a", "0.3.7", "21", True)]
        app = self.generate(releases)
        self.assertEqual([v["buildVersion"] for v in app["versions"]], ["21", "9"])

    def test_tag_does_not_decide_the_order_the_ipa_does(self):
        stable = self.fixture("v0.3.6", "0.3.6", "18")
        odd = self.fixture("v0.3.7c", "0.3.5", "21", True)
        app = self.generate([stable, odd])
        self.assertEqual([v["version"] for v in app["versions"]], ["0.3.6", "0.3.5"])

    def test_report_cargo_version_is_not_ipa_short_version(self):
        rel = self.fixture("v0.3.7c", "0.3.7", "021", True)
        url = "https://example.test/report.json"
        rel["assets"].append({"name": "ipa-verification.json", "browser_download_url": url})
        self.blobs[url] = json.dumps({"version": "0.3.7-beta.2", "build": "021",
                                     "bundle_identifier": source.BUNDLE,
                                     "sha256": rel["assets"][0]["digest"][7:]}).encode()
        version = self.generate([rel])["versions"][0]
        self.assertEqual((version["version"], version["buildVersion"]), ("0.3.7", "021"))
        report = json.loads(self.blobs[url])
        report["build"] = "21"
        self.blobs[url] = json.dumps(report).encode()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(ValueError, "No verified"):
            self.generate([rel])

    def test_corrupt_size_digest_and_stable_identity_fail_closed(self):
        for field, value in [("size", 1), ("digest", "sha256:" + "0" * 64)]:
            with self.subTest(field=field):
                rel = self.fixture("v0.3.6", "0.3.6", "18")
                rel["assets"][0][field] = value
                with self.assertRaisesRegex(ValueError, "Cannot verify stable"):
                    self.generate([rel])
        rel = self.fixture("v0.3.6", "0.3.6", "18", bundle="wrong.bundle")
        with self.assertRaisesRegex(ValueError, "bundle identifier"):
            self.generate([rel])

    def test_invalid_ipa_versions_are_not_inferred_from_tag(self):
        for version, build in [("0.3.7c", "21"), ("0.3.7", 21), ("", "21")]:
            with self.subTest(version=version, build=build):
                rel = self.fixture("v0.3.7c", version, build, True)
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(ValueError, "No verified"):
                    self.generate([rel])

    def test_empty_source_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "No verified"):
            self.generate([])


if __name__ == "__main__":
    unittest.main()
