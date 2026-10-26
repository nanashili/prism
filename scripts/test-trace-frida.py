"""Offline collector lifecycle tests. Never imports Frida or attaches to a target."""

import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock


spec = importlib.util.spec_from_file_location("trace_frida", Path(__file__).with_name("trace-frida.py"))
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


class CaptureLifecycle(unittest.TestCase):
    def test_missing_authorization_is_rejected_before_runtime_import(self):
        command = [
            sys.executable, str(Path(__file__).with_name("trace-frida.py")),
            "--pid", "42", "--thread", "7", "--target", __file__,
            "--static-base", "0x1000", "--workflow", "offline fixture",
            "--output", "unused-capture.json",
        ]
        with tempfile.TemporaryDirectory() as directory:
            # A guard regression must fail without ever loading the real Frida package.
            sentinel = Path(directory) / "frida.py"
            sentinel.write_text('raise RuntimeError("unauthorized Frida import")\n')
            environment = {**os.environ, "PYTHONPATH": directory}
            result = subprocess.run(command, capture_output=True, text=True,
                                    env=environment, check=False)
        self.assertEqual(result.returncode, 2)
        self.assertIn("required: --authorized-isolated", result.stderr)

    def test_load_failure_preserves_original_and_attempts_independent_cleanup(self):
        script = Mock()
        script.load.side_effect = RuntimeError("load failed")
        script.unload.side_effect = RuntimeError("unload failed")
        session = Mock()
        session.create_script.return_value = script
        session.detach.side_effect = RuntimeError("detach failed")
        frida = Mock()
        frida.attach.return_value = session

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "fixture"
            target.write_bytes(b"offline data fixture")
            args = argparse.Namespace(target=target, pid=42)
            with self.assertRaisesRegex(RuntimeError, "load failed") as caught:
                collector.capture(frida, args)

        script.unload.assert_called_once()
        session.detach.assert_called_once()
        notes = "\n".join(caught.exception.__notes__)
        self.assertIn("unload failed", notes)
        self.assertIn("detach failed", notes)
        self.assertIn("unload", notes)
        self.assertIn("detach", notes)


if __name__ == "__main__":
    unittest.main()
