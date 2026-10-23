"""Run against a generated/installed package, optionally with a test daemon."""

import asyncio
import os
import queue
import tempfile
import unittest
from unittest import mock

import unbill


class NativeBindingsTests(unittest.TestCase):
    def test_public_names_hide_bridge_details(self):
        self.assertFalse([name for name in dir(unbill) if "ffi" in name.lower()])
        self.assertEqual(unbill.ConsoleError.__module__, "unbill._native")

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
                with self.assertRaises(unbill.ConsoleError):
                    unbill.Console.open_rpc(directory)

    def test_in_process_persists_without_daemon_and_owns_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.dict(os.environ, {"UNBILL_SOCKET": directory + "/missing.sock"}):
                console = unbill.Console.open_in_process(directory)
                try:
                    device_id = console.device_id()
                    self.assertEqual(
                        console.service_build_info().version,
                        unbill.client_build_info().version,
                    )
                    ledger = console.create_ledger("Embedded Python", "USD")
                    with self.assertRaises(unbill.ConsoleError.DataDirectoryInUse):
                        unbill.Console.open_in_process(directory)
                finally:
                    del console

                reopened = unbill.Console.open_in_process(directory)
                try:
                    self.assertEqual(reopened.device_id(), device_id)
                    self.assertEqual(
                        reopened.ledger_detail(ledger.ledger_id).summary.name,
                        "Embedded Python",
                    )
                finally:
                    del reopened

    def test_in_process_operations_and_observer(self):
        with tempfile.TemporaryDirectory() as directory:
            console = unbill.Console.open_in_process(directory)
            try:
                self.check_operations_and_observer(console)
            finally:
                del console

    @unittest.skipUnless(os.environ.get("UNBILL_PYTHON_TEST_DIR"), "requires an isolated test daemon")
    def test_daemon_operations_and_observer(self):
        console = unbill.Console.open_rpc(os.environ["UNBILL_PYTHON_TEST_DIR"])
        self.check_operations_and_observer(console)

    def check_operations_and_observer(self, console):
        events = queue.Queue()

        class Observer(unbill.ConsoleObserver):
            def on_event(self, event):
                events.put(event)

        subscription = console.observe(Observer())
        try:
            ledger = console.create_ledger("Python binding test", "USD")
            self.assertTrue(ledger.ledger_id)
            self.assertEqual(console.ledger_detail(ledger.ledger_id).summary.name, "Python binding test")
            events.get(timeout=10)
            with self.assertRaises(unbill.ConsoleError):
                console.create_ledger("Invalid", "NOT_A_CURRENCY")

            async def invalid_sync():
                with self.assertRaises(unbill.ConsoleError):
                    await console.sync_once("invalid-node-id")

            asyncio.run(invalid_sync())
        finally:
            subscription.cancel()


if __name__ == "__main__":
    unittest.main()
