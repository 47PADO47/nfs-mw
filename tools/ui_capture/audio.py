"""Lossless speaker-output capture with a sample-index/monotonic-clock ledger."""
import math
from pathlib import Path
import queue
import subprocess
import threading
import time

from .common import hidden_process


class SampleClock:
    """Preserve elapsed time when WASAPI omits a block without an overflow flag."""
    def __init__(self, rate):
        self.rate = rate
        self.previous_adc = None
        self.previous_count = 0

    def advance(self, adc, count):
        gap = 0
        if self.previous_adc is not None:
            missing = adc - self.previous_adc - self.previous_count / self.rate
            if missing > 0.025:
                gap = round(missing * self.rate)
        self.previous_adc, self.previous_count = adc, count
        return gap


class Audio:
    def __init__(self, folder, timeline, ffmpeg, device=None):
        import pyaudiowpatch as pa
        self.pa = pa
        self.manager = pa.PyAudio()
        self.device = self.manager.get_default_wasapi_loopback()
        if device is not None:
            self.device = self.manager.get_device_info_by_index(device)
        if not self.device.get('isLoopbackDevice'):
            raise ValueError('Audio must use a speaker loopback endpoint, never a microphone')
        self.rate = int(self.device['defaultSampleRate'])
        self.channels = min(2, int(self.device['maxInputChannels']))
        self.timeline = timeline
        self.queue = queue.Queue(maxsize=512)
        self.frames = 0
        self.captured_frames = 0
        self.inserted_silence_frames = 0
        self.discontinuities = 0
        self.clock = SampleClock(self.rate)
        self.dropped_frames = 0
        self.status_count = 0
        self.first_t = None
        self.last_t = None
        self.error = None
        self.last_log = -1
        self.log = (folder / 'audio-ffmpeg.log').open('wb')
        self.command = [ffmpeg, '-hide_banner', '-loglevel', 'warning', '-n', '-f', 's16le',
                        '-ar', str(self.rate), '-ac', str(self.channels), '-i', 'pipe:0',
                        '-c:a', 'flac', str(folder / 'audio.flac')]
        self.process = hidden_process(self.command, stdin=subprocess.PIPE, stdout=self.log, stderr=self.log)
        self.writer = threading.Thread(target=self._write, daemon=True)
        self.writer.start()
        self.stream = self.manager.open(format=pa.paInt16, channels=self.channels, rate=self.rate,
                                        input=True, input_device_index=int(self.device['index']),
                                        frames_per_buffer=480, stream_callback=self._callback, start=False)

    def _callback(self, data, count, timing, status):
        now = self.timeline.seconds()
        delta = timing['input_buffer_adc_time'] - timing['current_time']
        valid = math.isfinite(delta) and abs(delta) < 5 and timing['input_buffer_adc_time'] > 0
        start = now + delta
        if not valid:
            start = now - count / self.rate
        if self.first_t is None:
            self.first_t = start
            self.timeline.emit('audio_start', sample_t=start, callback_t=now, portaudio=timing,
                               clock='portaudio_adc_correlated_to_qpc' if valid else 'callback_minus_block')
        padding = 0
        if valid:
            padding = self.clock.advance(timing['input_buffer_adc_time'], count)
        if padding:
            self.discontinuities += 1
            self.inserted_silence_frames += padding
            self.timeline.emit('audio_gap', sample_index=self.frames, silence_frames=padding,
                               seconds=padding / self.rate,
                               evidence='Unobserved WASAPI interval; silence preserves timing, content unknown')
        if status:
            self.status_count += 1
            self.timeline.emit('audio_status', status=status, frame=self.frames)
        try:
            self.queue.put_nowait((padding, data))
        except queue.Full:
            self.dropped_frames += count
        self.frames += count + padding
        self.captured_frames += count
        self.last_t = start + count / self.rate
        if int(now) != self.last_log:
            self.last_log = int(now)
            self.timeline.emit('audio_clock', sample_index=self.frames - count, sample_t=start,
                               callback_t=now, portaudio=timing, status=status)
        return (None, self.pa.paContinue)

    def _write(self):
        try:
            while True:
                block = self.queue.get()
                if block is None:
                    break
                padding, data = block
                # Bounded writes avoid allocating one huge buffer after a paused endpoint.
                silence = b'\0' * (self.rate * self.channels * 2) if padding else b''
                while padding:
                    frames = min(padding, self.rate)
                    self.process.stdin.write(silence[:frames * self.channels * 2])
                    padding -= frames
                self.process.stdin.write(data)
        except Exception as exc:
            self.error = str(exc)
        finally:
            self.process.stdin.close()

    def start(self):
        self.stream.start_stream()

    def stop(self):
        self.stream.stop_stream()
        self.stream.close()
        self.manager.terminate()
        self.queue.put(None, timeout=5)
        self.writer.join(timeout=15)
        if self.writer.is_alive():
            self.process.kill()
            self.error = 'Audio writer did not drain'
        self.process.wait(timeout=15)
        self.log.close()
        drift = None
        if self.first_t is not None and self.last_t is not None:
            drift = (self.last_t - self.first_t) - self.frames / self.rate
        return dict(device=self.device, rate=self.rate, channels=self.channels,
                    scope='selected_speaker_endpoint_all_applications_no_microphone', first_t=self.first_t,
                    frames=self.frames, seconds=self.frames / self.rate, clock_drift_seconds=drift,
                    captured_frames=self.captured_frames, inserted_silence_frames=self.inserted_silence_frames,
                    discontinuities=self.discontinuities,
                    evidence_complete=not (self.discontinuities or self.dropped_frames or self.status_count),
                    dropped_frames=self.dropped_frames, callback_status_count=self.status_count,
                    encoder_exit=self.process.returncode, error=self.error)
