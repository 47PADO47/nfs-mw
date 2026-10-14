import importlib.util
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
