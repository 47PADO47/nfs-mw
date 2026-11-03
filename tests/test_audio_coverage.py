import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("audio_coverage", Path(__file__).parents[1] / "tools/audio_coverage.py")
coverage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(coverage)


class CoverageTests(unittest.TestCase):
    def test_unknown_original_modules_and_absent_remake_voices_remain_gaps(self):
        report = coverage.summarize([{"module": "unknown"}, {"module": "CAR"}], [
            {"sequence": 1, "event": "coverage", "class": "CAR", "bank": "authored"},
            {"sequence": 2, "event": "module_tick", "class": "CAR", "params": [1]},
            {"event": "session_end", "dropped": 0, "write_failed": False},
        ])
        by_name = {f["module"]: f for f in report["families"]}
        self.assertEqual(by_name["unknown"]["route"], "unknown")
        self.assertEqual(by_name["CAR"]["remake_voice_starts"], 0)
        self.assertTrue(report["trace_complete"])
        self.assertEqual(report["parity"], "not_established")

    def test_drops_missing_end_and_sequence_gaps_are_incomplete(self):
        cases = [[], [{"event": "session_end", "dropped": 1}],
                 [{"sequence": 2, "event": "voice_start", "class": "CAR"},
                  {"event": "session_end", "dropped": 0}]]
        for rows in cases:
            self.assertFalse(coverage.summarize([], rows)["trace_complete"])

    def test_failures_override_loaded_or_started_coverage(self):
        report = coverage.summarize([], [
            {"sequence": 1, "event": "voice_start", "class": "CAR", "instance": 4, "player": 0},
            {"sequence": 2, "event": "voice_failure", "class": "CAR"},
            {"sequence": 3, "event": "module_drop", "class": "CAR", "instance": 4},
            {"event": "session_end", "dropped": 0},
        ])
        self.assertEqual(report["families"][0]["status"], "runtime_failure")
        self.assertEqual(report["active_at_last_record"], 0)

    def test_session_end_must_be_terminal_and_unique(self):
        end = {"event": "session_end", "dropped": 0, "write_failed": False}
        tick = {"sequence": 1, "event": "module_tick", "class": "CAR"}
        for rows in ([end, tick], [end, end], [tick, end, end]):
            with self.subTest(rows=rows):
                report = coverage.summarize([], iter(rows))
                self.assertFalse(report["trace_complete"])
                self.assertEqual(report["records_after_end"], 1)
        report = coverage.summarize([], iter([tick, end]))
        self.assertTrue(report["trace_complete"])
        self.assertEqual(report["records_after_end"], 0)

    def test_later_end_cannot_replace_failed_end_evidence(self):
        failed = {"event": "session_end", "dropped": 3, "write_failed": True}
        clean = {"event": "session_end", "dropped": 0, "write_failed": False}
        report = coverage.summarize([], iter([failed, clean]))
        self.assertFalse(report["trace_complete"])
        self.assertEqual(report["session_end"], failed)

    def test_ui_sound_schema_is_rejected_alone_and_when_mixed(self):
        sound = {"kind": "sound", "message": "0x12345678"}
        for rows in ([sound], [{"module": "CAR"}, sound]):
            with self.subTest(rows=rows), self.assertRaisesRegex(ValueError, "UI sound.*module"):
                coverage.summarize(iter(rows), [])

    def test_empty_metadata_and_unknown_modules_remain_supported(self):
        for rows in ([], [{"kind": "probe_identity"}]):
            self.assertEqual(coverage.summarize(iter(rows), [])["families"], [])
        report = coverage.summarize(iter([{"kind": "probe_identity"}, {"module": "UNKNOWN"}]), [])
        self.assertEqual(report["families"][0]["module"], "UNKNOWN")
        self.assertEqual(report["families"][0]["original_events"], 1)

    def test_cli_rejects_ui_sound_without_writing_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            original, remake, output = (root / name for name in ("original.jsonl", "remake.jsonl", "report.json"))
            original.write_text(json.dumps({"kind": "sound", "message": "0x12345678"}) + "\n", encoding="utf-8")
            remake.write_text("", encoding="utf-8")
            result = subprocess.run([sys.executable, str(Path(coverage.__file__)), "--original", str(original),
                                     "--remake", str(remake), "--output", str(output)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("UI sound", result.stderr)
            self.assertIn("module", result.stderr)
            self.assertFalse(output.exists())
