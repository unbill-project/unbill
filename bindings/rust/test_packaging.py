"""Exercise the release cleanliness guard against real Git states."""

from pathlib import Path
import subprocess
import tempfile
import unittest

from package import require_clean_checkout


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


if __name__ == "__main__":
    unittest.main()
