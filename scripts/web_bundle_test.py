import json
import shutil
import tempfile
import unittest
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

import web_bundle


class WebBundleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.web = self.root / "web"
        (self.web / "pkg").mkdir(parents=True)
        (self.web / "index.html").write_text('<script type="module" src="./pkg/rordle.js"></script>')
        (self.web / "pkg/rordle.js").write_text("export default function init() {}")
        (self.web / "pkg/rordle_bg.wasm").write_bytes(b"\0asm\x01\0\0\0")
        self.sha = "a" * 40
        self.dist = self.root / "dist"
        web_bundle.bundle(self.web, self.dist, self.sha, "123")
        self.archive = self.dist / "rordle-web.zip"

    def test_package_preserves_files_and_provenance(self):
        files, manifest = web_bundle.read_bundle(self.archive, self.sha, "123")
        self.assertEqual(manifest["source_sha"], self.sha)
        for name in web_bundle.WEB_FILES:
            self.assertEqual(files[name], (self.web / name).read_bytes())
        first = self.archive.read_bytes()
        web_bundle.bundle(self.web, self.dist, self.sha, "123")
        self.assertEqual(first, self.archive.read_bytes())

    def test_wrong_commit_or_run_is_rejected(self):
        for sha, run_id in [("b" * 40, "123"), (self.sha, "456")]:
            with self.assertRaises(ValueError):
                web_bundle.read_bundle(self.archive, sha, run_id)

    def test_modified_asset_is_rejected(self):
        files, _ = web_bundle.read_bundle(self.archive)
        files["pkg/rordle.js"] = b"modified"
        with ZipFile(self.archive, "w", compression=ZIP_DEFLATED) as zipped:
            for name, data in files.items():
                zipped.writestr(name, data)
        with self.assertRaisesRegex(ValueError, "checksums"):
            web_bundle.read_bundle(self.archive)

    def test_unexpected_archive_paths_are_rejected(self):
        with ZipFile(self.archive, "a") as zipped:
            zipped.writestr("../unexpected", "bad")
        with self.assertRaisesRegex(ValueError, "contents"):
            web_bundle.read_bundle(self.archive)

    def test_pages_retains_older_versions_and_updates_latest(self):
        history = self.root / "history"
        previous = history / "v0.1.0"
        previous.mkdir(parents=True)
        shutil.copyfile(self.archive, previous / "rordle-web.zip")
        (self.web / "index.html").write_text("new release")
        web_bundle.bundle(self.web, self.dist, "b" * 40, "456")
        site = self.root / "site"
        web_bundle.pages(self.archive, site, history, "v0.2.0", "b" * 40, "456")
        self.assertEqual((site / "index.html").read_text(), "new release")
        self.assertEqual((site / "releases/v0.2.0/index.html").read_text(), "new release")
        self.assertIn("<script", (site / "releases/v0.1.0/index.html").read_text())
        self.assertEqual(json.loads((site / "releases.json").read_text())["latest"], "v0.2.0")

    def test_existing_version_and_unsafe_versions_are_rejected(self):
        history = self.root / "history"
        previous = history / "v0.1.0"
        previous.mkdir(parents=True)
        shutil.copyfile(self.archive, previous / "rordle-web.zip")
        for version in ["v0.1.0", "../bad", "v01.0.0", "v1.0.0/extra"]:
            with self.assertRaises(ValueError):
                web_bundle.pages(self.archive, self.root / "site", history, version, self.sha, "123")


if __name__ == "__main__":
    unittest.main()
