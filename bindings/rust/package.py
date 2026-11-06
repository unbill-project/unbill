#!/usr/bin/env python3
"""Package the Rust Console and supporting libraries; publish only with --publish."""

import argparse
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
import json
from pathlib import Path
import re
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[2]


def run(*args, capture=False):
    result = subprocess.run(
        args, cwd=ROOT, check=True, text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    return result.stdout


def require_clean_checkout(root=ROOT):
    status = subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=normal"],
        cwd=root, text=True,
    )
    if status:
        raise RuntimeError(
            "The checkout has uncommitted changes; commit them or pass --allow-dirty.\n"
            + status
        )


def require_release_tag(tag, packages):
    mismatched = [
        f"{package['name']} ({package['version']})"
        for package in packages
        if tag != f"v{package['version']}"
    ]
    if mismatched:
        raise RuntimeError(
            f"Release tag {tag!r} does not match package versions: "
            + ", ".join(mismatched)
        )


def require_registry_metadata(packages):
    missing = [
        f"{package['name']} ({field})"
        for package in packages
        for field in ("description", "repository")
        if not package.get(field)
    ]
    if missing:
        raise RuntimeError("Missing registry metadata: " + ", ".join(missing))


def unpublished_packages(packages):
    remaining = []
    for package in packages:
        spec = f"{package['name']}@{package['version']}"
        result = subprocess.run(
            ["cargo", "info", spec, "--registry", "crates-io"],
            cwd=ROOT, capture_output=True, text=True,
        )
        if result.returncode == 0:
            print(f"Already published: {spec}", flush=True)
        elif f"could not find `{spec}` in registry" in result.stderr:
            remaining.append(package)
        else:
            raise RuntimeError(f"Could not check registry status for {spec}: {result.stderr}")
    return remaining


def publish_with_rate_limit(packages):
    for attempt in range(10):
        remaining = unpublished_packages(packages)
        if not remaining:
            return
        selection = [arg for package in remaining for arg in ("-p", package["name"])]
        result = subprocess.run(
            ["cargo", "publish", "--registry=crates-io", *selection, "--allow-dirty"],
            cwd=ROOT, capture_output=True, text=True,
        )
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode == 0:
            return
        match = re.search(r"Please try again after ([^\n]+? GMT)", result.stderr)
        if result.returncode != 101 or not match:
            raise subprocess.CalledProcessError(result.returncode, result.args)
        retry_at = parsedate_to_datetime(match.group(1))
        delay = max(0, (retry_at - datetime.now(timezone.utc)).total_seconds()) + 10
        print(f"crates.io rate limit; retrying in {delay:.0f} seconds", flush=True)
        time.sleep(delay)
    raise RuntimeError("crates.io rate limit persisted after 10 attempts")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-dirty", action="store_true")
    parser.add_argument("--publish", action="store_true", help="Upload to crates.io")
    parser.add_argument("--tag", help="Require v<version> to match every package")
    args = parser.parse_args()
    if not args.allow_dirty:
        require_clean_checkout()
    metadata = json.loads(run(
        "cargo", "metadata", "--format-version=1", "--no-deps", "--locked",
        capture=True,
    ))
    packages = {package["name"]: package for package in metadata["packages"]}
    selected = set()

    def visit(name):
        if name in selected:
            return
        selected.add(name)
        for dependency in packages[name]["dependencies"]:
            if dependency.get("path"):
                visit(dependency["name"])

    visit("unbill")
    if args.tag is not None:
        require_release_tag(args.tag, [packages[name] for name in sorted(selected)])
    require_registry_metadata([packages[name] for name in sorted(selected)])
    selection = [arg for name in sorted(selected) for arg in ("-p", name)]
    print("Rust packages: " + ", ".join(sorted(selected)), flush=True)

    # The catalog needs the complete release workspace. Capture its build output
    # before Cargo verifies the standalone archives outside that workspace.
    output = run(
        "cargo", "build", "-p", "unbill-license", "--locked",
        "--message-format=json", capture=True,
    )
    license_id = packages["unbill-license"]["id"]
    catalog = None
    for line in output.splitlines():
        message = json.loads(line)
        if (message.get("reason") == "build-script-executed"
                and message.get("package_id") == license_id):
            catalog = Path(message["out_dir"]) / "legal.json.gz"
    if catalog is None:
        raise RuntimeError("Cargo did not report the unbill-license catalog")

    snapshot = ROOT / "crates/unbill-license/catalog.json.gz"
    previous = snapshot.read_bytes() if snapshot.exists() else None
    try:
        snapshot.write_bytes(catalog.read_bytes())
        # The checkout was checked before generating the intentionally untracked
        # catalog. Cargo must now allow that generated file in the archive.
        if args.publish:
            publish_with_rate_limit([packages[name] for name in sorted(selected)])
        else:
            run("cargo", "package", "--registry=crates-io", *selection, "--allow-dirty")
    finally:
        if previous is None:
            snapshot.unlink(missing_ok=True)
        else:
            snapshot.write_bytes(previous)


if __name__ == "__main__":
    main()
