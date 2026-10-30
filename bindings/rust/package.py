#!/usr/bin/env python3
"""Package the Rust Console and supporting libraries; publish only with --publish."""

import argparse
import json
from pathlib import Path
import subprocess


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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-dirty", action="store_true")
    parser.add_argument("--publish", action="store_true", help="Upload to crates.io")
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
        command = "publish" if args.publish else "package"
        # The checkout was checked before generating the intentionally untracked
        # catalog. Cargo must now allow that generated file in the archive.
        run("cargo", command, "--registry=crates-io", *selection, "--allow-dirty")
    finally:
        if previous is None:
            snapshot.unlink(missing_ok=True)
        else:
            snapshot.write_bytes(previous)


if __name__ == "__main__":
    main()
