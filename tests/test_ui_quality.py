"""Quality checks use decoded timestamps, including invalid timelines."""
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.ui_capture import quality


class QualityTests(unittest.TestCase):
    def inspect(self, stamps, audio_log='Peak level dB: -10.0\nRMS level dB: -20.0', audio_exit=0):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            (folder / 'session.json').write_text(json.dumps({'audio': {
                'clock_drift_seconds': 0.1, 'discontinuities': 2,
                'inserted_silence_frames': 3, 'evidence_complete': False}}), encoding='utf-8')
            media = {'streams': [{'codec_type': 'video', 'width': 100, 'height': 50, 'r_frame_rate': '60/1'},
                                 {'codec_type': 'audio', 'codec_name': 'flac', 'channels': 2}]}
            frames = SimpleNamespace(returncode=0, communicate=lambda: ('\n'.join(stamps), ''))
            audio = SimpleNamespace(returncode=audio_exit, communicate=lambda: ('', audio_log))
            with patch.object(quality, 'probe', return_value=media), patch.object(quality, 'hidden_process', side_effect=[frames, audio]):
                report = quality.inspect(folder)
            encoded = (folder / 'quality.json').read_text(encoding='utf-8')
            json.loads(encoded, parse_constant=lambda value: self.fail(f'nonfinite JSON: {value}'))
            self.assertEqual(report['audio_discontinuities'], 2)
            self.assertEqual(report['audio_inserted_silence_frames'], 3)
            self.assertEqual(len(report['notes']), 3)
            return report

    def test_invalid_timestamps_do_not_pass_or_emit_nonfinite_statistics(self):
        for stamps in (['0', '2', '1'], ['0', '0'], ['0', 'NaN', '1'], ['0', 'inf'], ['0', 'N/A', '1'],
                       ['-1e308', '1e308'], ['0', '5e-324']):
            with self.subTest(stamps=stamps):
                report = self.inspect(stamps)
                self.assertEqual(report['decoded_frames'], len(stamps))
                self.assertFalse(report['video_timing_valid'])
                self.assertFalse(report['media_checks_passed'])
                for name in ('measured_mean_fps', 'median_frame_interval', 'largest_frame_interval'):
                    self.assertIsNone(report[name])

    def test_valid_variable_rate_and_audio_failure_controls(self):
        valid = ['0', '0.02', '0.07', '0.1']
        report = self.inspect(valid)
        self.assertTrue(report['video_timing_valid'])
        self.assertTrue(report['media_checks_passed'])
        self.assertAlmostEqual(report['measured_mean_fps'], 30)
        csv_report = self.inspect(['0,extra', '', '0.02,extra', '0.07,extra'])
        self.assertTrue(csv_report['media_checks_passed'])
        self.assertEqual(csv_report['decoded_frames'], 3)
        for stamps in ([], ['1']):
            self.assertFalse(self.inspect(stamps)['media_checks_passed'])
        self.assertFalse(self.inspect(valid, audio_exit=1)['media_checks_passed'])
        self.assertFalse(self.inspect(valid, audio_log='Peak level dB: -inf')['media_checks_passed'])

    def test_nonfinite_audio_levels_are_null_in_strict_json(self):
        for level in ('-inf', 'inf', 'nan'):
            with self.subTest(level=level):
                report = self.inspect(['0', '0.1'],
                                      audio_log=f'Peak level dB: {level}\nRMS level dB: {level}')
                self.assertIsNone(report['peak_db'])
                self.assertIsNone(report['rms_db'])
                self.assertTrue(report['video_timing_valid'])
                self.assertFalse(report['audio_nonzero'])
                self.assertFalse(report['media_checks_passed'])
        report = self.inspect(['0', '0.1'])
        self.assertEqual(report['peak_db'], -10.0)
        self.assertEqual(report['rms_db'], -20.0)
