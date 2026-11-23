#!/usr/bin/env python3
"""Render a direct-device capture as summary.json, trace.json and report.html."""
import argparse
from collections import defaultdict
import html
import json
import math
from pathlib import Path


def read_jsonl(path):
    """Only a truncated final record from an interrupted write may be ignored."""
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    result = []
    for index, line in enumerate(lines):
        try:
            result.append(json.loads(line))
        except json.JSONDecodeError:
            if index == len(lines) - 1 and not line.endswith("\n"):
                break
            raise
    return result


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)] if ordered else None


def summarize(spans):
    by_id = {s["id"]: s for s in spans}
    groups = defaultdict(list)
    for span in spans:
        operation = span.get("operation")
        if span["name"] in ("store.queue", "store.backpressure"):
            operation = by_id.get(span["parent"], {}).get("operation")
        label = span["name"] + (f" [{operation}]" if operation else "")
        groups[label].append(span)
    metrics = {}
    for label, rows in sorted(groups.items()):
        times = [s["duration_us"] / 1000 for s in rows]
        metrics[label] = {
            "count": len(rows), "total_ms": sum(times), "mean_ms": sum(times) / len(rows),
            "p50_ms": percentile(times, .50), "p95_ms": percentile(times, .95),
            "p99_ms": percentile(times, .99), "max_ms": max(times),
            "entered_ms": sum(s["entered_us"] for s in rows) / 1000,
        }
    return metrics


def chrome_trace(spans, pid):
    events = [{"ph": "M", "pid": pid, "tid": 0, "name": "process_name", "args": {"name": "UnbillDevice"}}]
    for span in spans:
        common = {"name": span["name"], "cat": span["name"].split(".")[0], "pid": pid, "tid": 0, "id": span["id"]}
        events.extend([
            {**common, "ph": "b", "ts": span["start_us"], "args": {"parent_id": span["parent"], "operation": span.get("operation"), "entered_us": span["entered_us"]}},
            {**common, "ph": "e", "ts": span["start_us"] + span["duration_us"]},
        ])
    return {"traceEvents": sorted(events, key=lambda e: (e.get("ts", -1), e["ph"] == "e")), "displayTimeUnit": "ms"}


def resource_metrics(samples, hz):
    cpu = []
    for previous, current in zip(samples, samples[1:]):
        if hz and all(row.get(key) is not None for row in (previous, current) for key in ("user_ticks", "system_ticks")):
            delta_ms = current["elapsed_ms"] - previous["elapsed_ms"]
            if delta_ms > 0:
                delta_ticks = sum(current[key] - previous[key] for key in ("user_ticks", "system_ticks"))
                cpu.append((current["elapsed_ms"], delta_ticks / hz / (delta_ms / 1000) * 100))
    lags = [row["scheduler_lag_ms"] for row in samples]
    rss = [row["rss_kib"] for row in samples if row.get("rss_kib") is not None]
    return {
        "sample_count": len(samples), "scheduler_lag_p95_ms": percentile(lags, .95),
        "scheduler_lag_max_ms": max(lags, default=None),
        "peak_sampled_rss_mib": max(rss) / 1024 if rss else None,
        "max_threads": max((s["threads"] for s in samples if s.get("threads") is not None), default=None),
    }, cpu


def chart(points, label):
    if not points:
        return f"<p>{html.escape(label)}: unavailable on this platform or capture too short.</p>"
    width, height = 900, 140
    maximum_x = max(max(x for x, _ in points), 1)
    observed_y = max(y for _, y in points)
    maximum_y = observed_y or 1
    coords = " ".join(f"{x / maximum_x * width:.1f},{height - y / maximum_y * height:.1f}" for x, y in points)
    return f'<h3>{html.escape(label)} (max {observed_y:.2f})</h3><svg viewBox="0 0 {width} {height}" role="img" aria-label="{html.escape(label)}"><polyline fill="none" stroke="#087e8b" stroke-width="2" points="{coords}"/></svg><small>0 → {maximum_x / 1000:.2f} seconds; y-axis starts at zero.</small>'


def build_report(directory, baseline=None):
    metadata = json.loads((directory / "metadata.json").read_text(encoding="utf-8"))
    if metadata.get("schema") != 1:
        raise ValueError("Unsupported capture schema")
    result_path = directory / "result.json"
    result = json.loads(result_path.read_text(encoding="utf-8")) if result_path.exists() else {"success": False, "error": "No result.json; capture ended before completion."}
    spans = read_jsonl(directory / "spans.jsonl")
    samples_path = directory / "runtime.jsonl"
    samples = read_jsonl(samples_path) if samples_path.exists() else []
    metrics = summarize(spans)
    resources, cpu = resource_metrics(samples, metadata.get("clock_ticks_per_sec"))
    summary = {"schema": 1, "metadata": metadata, "result": result, "metrics": metrics, "resources": resources}
    comparisons = {}
    if baseline:
        old = json.loads((baseline / "summary.json").read_text(encoding="utf-8"))
        keys = ("os", "arch", "store", "scenario", "ledgers", "bills_per_ledger", "iterations_per_worker", "concurrency", "worker_threads", "idle_secs", "headless", "debug_assertions", "trace_enabled")
        mismatches = [key for key in keys if old["metadata"].get(key) != metadata.get(key)]
        if mismatches:
            raise ValueError("Baseline configuration differs: " + ", ".join(mismatches))
        if not result["success"] or not old["result"]["success"]:
            raise ValueError("Cannot compare unsuccessful runs")
        for label, row in metrics.items():
            previous = old["metrics"].get(label)
            if previous and previous["p95_ms"]:
                comparisons[label] = (row["p95_ms"] / previous["p95_ms"] - 1) * 100
        summary["p95_change_percent"] = comparisons
    (directory / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    (directory / "trace.json").write_text(json.dumps(chrome_trace(spans, metadata["pid"])) + "\n", encoding="utf-8")
    rows = []
    for label, row in sorted(metrics.items(), key=lambda item: item[1]["total_ms"], reverse=True):
        delta = f'{comparisons[label]:+.1f}%' if label in comparisons else "—"
        cells = [html.escape(label), str(row["count"])] + [f'{row[key]:.3f}' for key in ("total_ms", "p50_ms", "p95_ms", "p99_ms", "max_ms", "entered_ms")] + [delta]
        rows.append("<tr>" + "".join(f"<td>{cell}</td>" for cell in cells) + "</tr>")
    title = f'UnbillDevice · {metadata["store"]} · {metadata["scenario"]}'
    status = "Completed" if result["success"] else "Incomplete / failed"
    page = f'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{html.escape(title)}</title>
<style>body{{font:15px system-ui;max-width:1200px;margin:40px auto;padding:0 20px;color:#142c35;background:#f7fafb}}h1{{font-size:30px}}table{{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums;background:white}}th,td{{padding:9px;border-bottom:1px solid #dde5e8;text-align:right}}td:first-child,th:first-child{{text-align:left}}thead{{position:sticky;top:0;background:#d9e8eb}}svg{{width:100%;background:white;border-bottom:1px solid #abc}}input{{padding:10px;width:300px;margin:15px 0}}pre{{white-space:pre-wrap}}.scroll{{overflow:auto}}</style>
<h1>{html.escape(title)}</h1><p><b>{status}</b> · {len(spans)} closed spans · {metadata["concurrency"]} workers · {metadata["ledgers"]} ledgers × {metadata["bills_per_ledger"]} bills</p>
<p>{html.escape(str(result.get("error") or ""))}</p>
<p>Wall durations include waiting. Entered time is inclusive time inside instrumented polls or synchronous spans; it is <b>not CPU time</b>. Nested/concurrent totals overlap and must not be added. Resource samples cover setup as well as workloads.</p>
<p>Open <a href="trace.json">trace.json</a> in Perfetto for the timeline. Each async span has its own ID and a parent_id argument. Closed spans include cancelled lifetimes; a span closing does not assert operation success.</p>
<pre>{html.escape(json.dumps(resources, indent=2))}</pre>
{chart(cpu, "Process CPU (% of one core, Linux)")}
{chart([(s["elapsed_ms"], s["rss_kib"] / 1024) for s in samples if s.get("rss_kib") is not None], "Resident memory (MiB, Linux)")}
{chart([(s["elapsed_ms"], s["scheduler_lag_ms"]) for s in samples], "100 ms heartbeat delay (ms)")}
<h2>Operation timings</h2><input id="filter" placeholder="Filter operation…" aria-label="Filter operation"><div class="scroll"><table><thead><tr><th>Operation</th><th>Count</th><th>Total ms</th><th>p50 ms</th><th>p95 ms</th><th>p99 ms</th><th>Max ms</th><th>Entered ms</th><th>p95 Δ</th></tr></thead><tbody>{''.join(rows)}</tbody></table></div>
<p>Small sample counts give unstable tail percentiles. Sync's first iteration transfers the initial delta; later iterations measure unchanged peers. Compare those paths separately in the timeline.</p>
<details><summary>Run configuration</summary><pre>{html.escape(json.dumps(metadata, indent=2))}</pre></details>
<script>document.getElementById('filter').addEventListener('input',e=>{{for(const row of document.querySelectorAll('tbody tr'))row.hidden=!row.firstChild.textContent.toLowerCase().includes(e.target.value.toLowerCase())}});</script></html>'''
    (directory / "report.html").write_text(page, encoding="utf-8")
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args()
    try:
        summary = build_report(args.capture, args.baseline)
    except (ValueError, OSError) as error:
        parser.exit(1, f"Report failed: {error}\n")
    print(args.capture / "report.html")
    print(f'{len(summary["metrics"])} operation groups; success={summary["result"]["success"]}')


if __name__ == "__main__":
    main()
