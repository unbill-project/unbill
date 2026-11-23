import json
from pathlib import Path
import tempfile
import unittest
from report import build_report, chrome_trace, percentile, read_jsonl, resource_metrics, summarize


class Reports(unittest.TestCase):
    def test_percentiles_are_nearest_rank(self):
        self.assertEqual(percentile([1, 2, 3, 4, 5], .95), 5)
        self.assertEqual(percentile([2, 1], .50), 1)
        self.assertIsNone(percentile([], .95))

    def test_queue_uses_its_actor_operation_and_trace_uses_unique_ids(self):
        spans = [
            dict(id=1, parent=None, name="store.request", operation="LoadLedger", start_us=0, duration_us=1000, entered_us=0),
            dict(id=2, parent=1, name="store.queue", operation=None, start_us=5, duration_us=900, entered_us=0),
        ]
        metrics = summarize(spans)
        self.assertEqual(metrics["store.queue [LoadLedger]"]["p95_ms"], .9)
        events = chrome_trace(spans, 42)["traceEvents"]
        starts = [e for e in events if e["ph"] == "b"]
        ends = [e for e in events if e["ph"] == "e"]
        self.assertEqual({e["id"] for e in starts}, {e["id"] for e in ends})
        self.assertEqual(starts[1]["args"]["parent_id"], 1)

    def test_only_an_incomplete_final_record_is_ignored(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "capture"
            path.write_text('{"id":1}\n{"id":')
            self.assertEqual(read_jsonl(path), [{"id": 1}])
            path.write_text('{"id":1}\nbroken\n')
            with self.assertRaises(json.JSONDecodeError):
                read_jsonl(path)

    def test_cpu_ticks_are_process_wide_and_unsupported_resources_are_missing(self):
        samples = [
            dict(elapsed_ms=0, scheduler_lag_ms=0, rss_kib=1024, threads=2, user_ticks=10, system_ticks=5),
            dict(elapsed_ms=1000, scheduler_lag_ms=3, rss_kib=2048, threads=3, user_ticks=160, system_ticks=55),
        ]
        stats, cpu = resource_metrics(samples, 100)
        self.assertEqual(cpu, [(1000, 200)])
        self.assertEqual(stats["peak_sampled_rss_mib"], 2)
        stats, cpu = resource_metrics([], None)
        self.assertIsNone(stats["peak_sampled_rss_mib"])
        self.assertEqual(cpu, [])

    def test_comparison_rejects_incompatible_or_failed_captures(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            baseline = root / "old"
            baseline.mkdir()
            metadata = dict(schema=1, pid=1, store="sqlite", scenario="read", concurrency=1, ledgers=1, bills_per_ledger=0)
            baseline.joinpath("metadata.json").write_text(json.dumps(metadata))
            baseline.joinpath("result.json").write_text('{"success":true}')
            baseline.joinpath("spans.jsonl").write_text("")
            build_report(baseline)
            current = root / "new"
            current.mkdir()
            current.joinpath("metadata.json").write_text(json.dumps({**metadata, "concurrency": 2}))
            current.joinpath("result.json").write_text('{"success":true}')
            current.joinpath("spans.jsonl").write_text("")
            with self.assertRaisesRegex(ValueError, "concurrency"):
                build_report(current, baseline)
            current.joinpath("metadata.json").write_text(json.dumps(metadata))
            old_spans = [dict(id=1, parent=None, name="workload.read", operation=None, start_us=0, duration_us=1000, entered_us=100)]
            baseline.joinpath("spans.jsonl").write_text(json.dumps(old_spans[0]) + "\n")
            build_report(baseline)
            current.joinpath("spans.jsonl").write_text(json.dumps({**old_spans[0], "duration_us": 2000}) + "\n")
            compared = build_report(current, baseline)
            self.assertEqual(compared["p95_change_percent"]["workload.read"], 100)
            current.joinpath("result.json").write_text('{"success":false}')
            with self.assertRaisesRegex(ValueError, "unsuccessful"):
                build_report(current, baseline)


if __name__ == "__main__":
    unittest.main()
