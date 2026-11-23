#!/usr/bin/env python3
"""Build once, profile isolated device workloads, and render local reports."""
import argparse
import html
import json
from pathlib import Path
import subprocess
import sys
from report import build_report

ROOT = Path(__file__).resolve().parents[2]


def build(headless):
    command = ["cargo", "build", "--locked", "--profile", "profiling", "-p", "unbill-device-profile", "--message-format=json-render-diagnostics"]
    if headless:
        command += ["--features", "headless"]
    result = subprocess.run(command, cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE)
    for line in result.stdout.splitlines():
        item = json.loads(line)
        if item.get("reason") == "compiler-artifact" and item.get("executable") and item["target"]["name"] == "unbill-device-profile":
            return Path(item["executable"])
    raise RuntimeError("Cargo did not report the profiler executable")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, help="Use an already-built profiler")
    parser.add_argument("--headless", action="store_true", help="Build without BLE; IP networking remains enabled")
    parser.add_argument("--include-sync", action="store_true", help="Also exercise real peer networking")
    parser.add_argument("--stores", nargs="+", choices=["sqlite", "memory"], default=["sqlite", "memory"])
    parser.add_argument("--ledgers", type=int, default=10)
    parser.add_argument("--bills", type=int, default=10)
    parser.add_argument("--iterations", type=int, default=20)
    parser.add_argument("--concurrency", type=int, default=4)
    parser.add_argument("--worker-threads", type=int, help="Tokio worker threads (default: available parallelism)")
    parser.add_argument("--idle-secs", type=int, default=5)
    parser.add_argument("--timeout-secs", type=int, default=120)
    parser.add_argument("--baseline", type=Path, help="An earlier suite directory with matching workload configuration")
    parser.add_argument("--no-trace", action="store_true", help="Measure the workload with spans disabled")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists; choose a fresh directory")
    if min(args.ledgers, args.iterations, args.concurrency, args.timeout_secs) < 1 or min(args.bills, args.idle_secs) < 0:
        parser.error("counts must be positive (bills and idle-secs may be zero)")
    if args.binary and args.headless:
        parser.error("--headless applies only when building; use the desired binary directly")
    if args.worker_threads is not None and args.worker_threads < 1:
        parser.error("worker-threads must be positive")
    binary = args.binary.resolve() if args.binary else build(args.headless)
    args.output.mkdir(parents=True)
    entries = []
    failures = []
    for store in args.stores:
        for scenario in ["idle", "read", "write", "auth"] + (["sync"] if args.include_sync else []):
            name = f"{store}-{scenario}"
            destination = args.output / name
            command = [str(binary), "--output", str(destination.resolve()), "--store", store, "--scenario", scenario]
            for flag in ["ledgers", "bills", "iterations", "concurrency", "idle-secs", "timeout-secs"]:
                command += [f"--{flag}", str(getattr(args, flag.replace("-", "_")))]
            if args.worker_threads is not None:
                command += ["--worker-threads", str(args.worker_threads)]
            if args.no_trace:
                command.append("--no-trace")
            print(f"Profiling {name}", flush=True)
            with (args.output / f"{name}.log").open("w", encoding="utf-8") as log:
                try:
                    result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=args.timeout_secs + 30)
                    if result.returncode:
                        failures.append(name)
                except subprocess.TimeoutExpired:
                    failures.append(name)
            try:
                summary = build_report(destination, args.baseline / name if args.baseline else None)
                entries.append(f'<li><a href="{name}/report.html">{name}</a> — {"completed" if summary["result"]["success"] else "failed"}</li>')
            except (OSError, ValueError) as error:
                print(f"{name}: {error}", file=sys.stderr)
                if name not in failures:
                    failures.append(name)
                entries.append(f'<li>{name} — {html.escape(str(error))}</li>')
    (args.output / "index.html").write_text('<!doctype html><meta charset="utf-8"><title>UnbillDevice profiling suite</title><h1>UnbillDevice profiling suite</h1><ul>' + "".join(entries) + '</ul>', encoding='utf-8')
    print(args.output / "index.html")
    if failures:
        parser.exit(1, "Failed captures: " + ", ".join(failures) + "\n")


if __name__ == "__main__":
    main()
