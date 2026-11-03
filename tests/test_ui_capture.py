"""Timing, evidence boundaries and browser-index safety without a live game."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import queue
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.ui_capture.common import Timeline, external_output
from tools.ui_capture.audio import Audio, SampleClock
from tools.ui_capture.media import chapters_file
from tools.ui_capture.review import build, event_time, index_events


class CaptureTests(unittest.TestCase):
    def test_audio_encoder_receives_real_samples_after_an_explicit_gap(self):
        class Sink:
            def __init__(self):
                self.data = bytearray()
                self.closed = False

            def write(self, data):
                self.data.extend(data)

            def close(self):
                self.closed = True

        sink = Sink()
        blocks = queue.Queue()
        blocks.put((2, b'1234'))
        blocks.put(None)
        writer = SimpleNamespace(rate=48000, channels=2, queue=blocks,
                                 process=SimpleNamespace(stdin=sink), error=None)
        Audio._write(writer)
        self.assertEqual(sink.data, b'\0' * 8 + b'1234')
        self.assertTrue(sink.closed)
        self.assertIsNone(writer.error)

    def test_silent_wasapi_packet_gap_does_not_compress_later_sound(self):
        clock = SampleClock(48000)
        self.assertEqual(clock.advance(100.0, 480), 0)
        self.assertEqual(clock.advance(100.01, 480), 0)
        self.assertEqual(clock.advance(100.09, 480), 3360)
        self.assertEqual(clock.advance(100.10, 480), 0)

    def test_original_events_use_capture_clock_not_delayed_delivery(self):
        row = dict(qpc_ticks=105_000_000, host_received_perf_ns=99_000_000_000)
        self.assertAlmostEqual(event_time(row, 10_000_000_000, 10_000_000), 0.5)
        self.assertIsNone(event_time(row, 0, None))

    def test_output_cannot_enter_source_checkout(self):
        root = Path(tempfile.gettempdir()).resolve() / 'source'
        with self.assertRaises(ValueError):
            external_output(root / 'captures', root)
        self.assertEqual(external_output(root.parent / 'private', root), root.parent / 'private')

    def test_chapters_escape_labels_and_keep_nonempty_intervals(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'chapters.ffmeta'
            chapters_file(path, [dict(t=1.5, label='Menu\nkey=value;bad')], 2)
            text = path.read_text()
            self.assertIn('START=1500\nEND=2000', text)
            self.assertIn('title=Menu key value bad', text)

    def test_review_keeps_raw_event_time_and_escapes_embedded_html(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            timeline = Timeline(folder)
            timeline.emit('marker', label='</script><script>bad()</script>')
            timeline.file.close()
            manifest = dict(state='finished', clock=timeline.header(), media={'format': {'start_time': '0.25'}})
            (folder / 'session.json').write_text(json.dumps(manifest))
            events = build(folder, preview=False)
            self.assertEqual(len(events), 1)
            self.assertNotIn('</script><script>bad()', (folder / 'review.html').read_text())
            self.assertEqual(json.loads((folder / 'review-summary.json').read_text())['media_origin_seconds'], 0.25)


if __name__ == '__main__':
    unittest.main()
