"""Exercise checkout cleanliness and release-version guards before publishing."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from package import main, require_clean_checkout, require_release_tag


class CleanCheckoutTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="unbill-package-test-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / ".gitignore").write_text("catalog.json.gz\n")
        (self.root / "source.rs").write_text("// committed source\n")
        self.git("init", "--quiet")
        self.git("add", ".")
        self.git(
            "-c", "user.name=Package Test",
            "-c", "user.email=package-test@example.invalid",
            "commit", "--quiet", "-m", "Fixture",
        )

    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)

    def test_clean_checkout_and_ignored_catalog_are_accepted(self):
        require_clean_checkout(self.root)
        (self.root / "catalog.json.gz").write_bytes(b"generated catalog")
        require_clean_checkout(self.root)

    def test_unstaged_and_staged_source_changes_are_rejected(self):
        (self.root / "source.rs").write_text("// changed source\n")
        with self.assertRaisesRegex(RuntimeError, "uncommitted changes"):
            require_clean_checkout(self.root)
        self.git("add", "source.rs")
        with self.assertRaisesRegex(RuntimeError, "uncommitted changes"):
            require_clean_checkout(self.root)

    def test_untracked_source_is_rejected(self):
        (self.root / "new.rs").write_text("// new source\n")
        with self.assertRaisesRegex(RuntimeError, "uncommitted changes"):
            require_clean_checkout(self.root)


class ReleaseTagTests(unittest.TestCase):
    def test_matching_shared_version_is_accepted(self):
        for version in ("0.0.9", "1.0.0-rc.1"):
            with self.subTest(version=version):
                require_release_tag(f"v{version}", [
                    {"name": "unbill", "version": version},
                    {"name": "unbill-console", "version": version},
                ])

    def test_wrong_version_and_nonrelease_tags_are_rejected(self):
        for tag in ("v0.0.10", "0.0.9", "nightly-20261030", "main", ""):
            with self.subTest(tag=tag):
                with self.assertRaisesRegex(RuntimeError, "does not match"):
                    require_release_tag(tag, [{"name": "unbill", "version": "0.0.9"}])

    def test_mismatched_dependency_stops_publish_before_building(self):
        metadata = {"packages": [
            {"name": "unbill", "version": "0.0.9", "dependencies": [
                {"name": "unbill-console", "path": "crates/unbill-console"},
            ]},
            {"name": "unbill-console", "version": "0.0.8", "dependencies": []},
        ]}
        with (
            patch("sys.argv", ["package.py", "--publish", "--tag", "v0.0.9"]),
            patch("package.require_clean_checkout"),
            patch("package.run", return_value=json.dumps(metadata)) as run,
        ):
            with self.assertRaisesRegex(RuntimeError, r"unbill-console \(0.0.8\)"):
                main()
        run.assert_called_once_with(
            "cargo", "metadata", "--format-version=1", "--no-deps", "--locked",
            capture=True,
        )


if __name__ == "__main__":
    unittest.main()
