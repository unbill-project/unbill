"""Checks for release identity and native wheel metadata."""

from email.parser import BytesParser
from pathlib import Path
import platform
import sys
import unittest
import zipfile

from build import validate_release_tag


class ReleaseTests(unittest.TestCase):
    def test_workspace_version_is_the_release_identity(self):
        validate_release_tag("0.0.7", "v0.0.7")
        validate_release_tag("1.0.0-beta.1", "v1.0.0-beta.1")

    def test_mismatched_and_nonrelease_tags_are_rejected(self):
        for tag in ["v0.0.8", "nightly-20261011", "0.0.7", "v0.0.7-extra"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                validate_release_tag("0.0.7", tag)


class WheelTests(unittest.TestCase):
    def test_distributions_include_native_code_and_release_metadata(self):
        directory = Path(sys.argv[1])
        wheels = list(directory.glob("*.whl"))
        self.assertTrue(wheels, "No wheels found")
        for wheel in wheels:
            with self.subTest(wheel=wheel.name), zipfile.ZipFile(wheel) as archive:
                names = archive.namelist()
                metadata_path = next(n for n in names if n.endswith(".dist-info/METADATA"))
                metadata = BytesParser().parsebytes(archive.read(metadata_path))
                self.assertEqual(metadata["Name"], "unbill")
                self.assertEqual(metadata["Requires-Python"], ">=3.9")
                self.assertEqual(metadata["License-Expression"], "MIT OR Apache-2.0")
                self.assertTrue(any(n.endswith("LICENSE-MIT") for n in names))
                self.assertTrue(any(n.endswith("LICENSE-APACHE") for n in names))
                # Repair tools compute library paths in the wheel tree. Keeping
                # the package at the root preserves those paths after pip install.
                self.assertIn("unbill/_native.py", names)
                self.assertNotIn("unbill/unbill_ffi.py", names)
                self.assertTrue(any(n.endswith((".dylib", ".so", ".dll")) for n in names))
                wheel_metadata = BytesParser().parsebytes(
                    archive.read(metadata_path.replace("METADATA", "WHEEL"))
                )
                self.assertEqual(wheel_metadata["Root-Is-Purelib"], "false")
                tags = wheel_metadata.get_all("Tag", [])
                self.assertTrue(any(tag.startswith("py3-none-") for tag in tags))
                if sys.platform == "darwin":
                    self.assertIn(platform.machine(), wheel.name)
                    self.assertNotIn("universal2", wheel.name)


if __name__ == "__main__":
    # An optional wheel directory enables distribution checks after repair.
    if len(sys.argv) > 1:
        unittest.main(argv=[sys.argv[0]], defaultTest=None)
    else:
        unittest.main(defaultTest="ReleaseTests")
