"""Run against a generated/installed package, optionally with a test daemon."""

import asyncio
import os
import queue
import tempfile
import unittest
from unittest import mock

import unbill


class NativeBindingsTests(unittest.TestCase):
    def test_currency_catalog_crosses_native_boundary(self):
        codes = unbill.supported_currency_codes()
        self.assertEqual(codes, sorted(set(codes)))
        self.assertIn("USD", codes)

    def test_build_info_and_bill_ids(self):
        self.assertTrue(unbill.client_build_info().version)
        first, second = unbill.new_bill_id(), unbill.new_bill_id()
        self.assertEqual(len(first), 26)
        self.assertNotEqual(first, second)

    def test_missing_daemon_raises_generated_error(self):
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.dict(os.environ, {"UNBILL_SOCKET": directory + "/missing.sock"}):
                with self.assertRaises(unbill.FfiError):
                    unbill.FfiConsole.open(directory)

    @unittest.skipUnless(os.environ.get("UNBILL_PYTHON_TEST_DIR"), "requires an isolated test daemon")
    def test_daemon_operations_and_observer(self):
        console = unbill.FfiConsole.open(os.environ["UNBILL_PYTHON_TEST_DIR"])
        events = queue.Queue()

        class Observer(unbill.FfiConsoleObserver):
            def on_event(self, event):
                events.put(event)

        subscription = console.observe(Observer())
        try:
            ledger = console.create_ledger("Python binding test", "USD")
            self.assertTrue(ledger.ledger_id)
            self.assertEqual(console.ledger_detail(ledger.ledger_id).summary.name, "Python binding test")
            events.get(timeout=10)
            with self.assertRaises(unbill.FfiError):
                console.create_ledger("Invalid", "NOT_A_CURRENCY")

            async def invalid_sync():
                with self.assertRaises(unbill.FfiError):
                    await console.sync_once("invalid-node-id")

            asyncio.run(invalid_sync())
        finally:
            subscription.cancel()


if __name__ == "__main__":
    unittest.main()
