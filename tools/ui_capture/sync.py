"""Estimate audio and video delay of a recorded session from original UI events.

For each original sound request, the audible onset is the first rise of the recorded envelope above the
level just before the request. For each focus change, the visible response is the first frame that differs
from the frame before the event. The median of the measured lags is an estimate of the capture/mix pipeline
delay. It is not a calibration of the original game's own latency and cannot prove which sound played.
"""
import argparse
from array import array
import json
import re
from pathlib import Path
import statistics
import subprocess

from .common import hidden_process, write_json
from .review import index_events

AUDIO_RATE = 8000
ENVELOPE_MS = 2
SEARCH_SECONDS = 0.4
VIDEO_SIZE = (64, 36)


def envelope(samples, rate=AUDIO_RATE, step_ms=ENVELOPE_MS):
    step = max(1, rate * step_ms // 1000)
    return [max((abs(s) for s in samples[i:i + step]), default=0) for i in range(0, len(samples), step)]


def onset(env, request_index, search, floor_bins=25, ratio=4.0, minimum=300):
    """First envelope bin at or after the request clearly above the quiet level just before it."""
    before = env[max(0, request_index - floor_bins):request_index]
    if not before:
        return None
    floor = statistics.median(before)
    threshold = max(minimum, floor * ratio)
    for index in range(request_index, min(len(env), request_index + search)):
        if env[index] >= threshold:
            return index
    return None


def read_audio(ffmpeg, path, start, seconds):
    command = [ffmpeg, '-v', 'error', '-ss', f'{max(0, start):.3f}', '-t', f'{seconds:.3f}', '-i', str(path),
               '-ac', '1', '-ar', str(AUDIO_RATE), '-f', 's16le', 'pipe:1']
    output = hidden_process(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL).communicate(timeout=60)[0]
    samples = array('h')
    samples.frombytes(output[:len(output) // 2 * 2])
    return samples


def audio_lags(events, audio_path, audio_origin, ffmpeg, windows):
    lags = []
    step = ENVELOPE_MS / 1000
    for start, end in windows:
        requests = [e['t'] for e in events if e['kind'] == 'sound' and start <= e['t'] <= end]
        if not requests:
            continue
        lead = 0.2
        samples = read_audio(ffmpeg, audio_path, start - audio_origin - lead, end - start + lead + SEARCH_SECONDS)
        env = envelope(samples)
        base = start - lead
        for t in requests:
            index = round((t - base) / step)
            found = onset(env, index, round(SEARCH_SECONDS / step))
            if found is not None:
                lags.append(dict(t=t, lag=(found - index) * step))
    return lags


def read_frames(ffmpeg, path, start, seconds):
    """Small greyscale frames with their own presentation times, from `showinfo` on the same decode."""
    width, height = VIDEO_SIZE
    command = [ffmpeg, '-hide_banner', '-copyts', '-ss', f'{max(0, start):.3f}', '-i', str(path),
               '-t', f'{seconds:.3f}', '-fps_mode', 'passthrough',
               '-vf', f'showinfo,scale={width}:{height},format=gray', '-f', 'rawvideo', 'pipe:1']
    raw, log = hidden_process(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE).communicate(timeout=180)
    times = [float(m.group(1)) for m in re.finditer(rb'pts_time:\s*([0-9.]+)', log)]
    size = width * height
    frames = [raw[i:i + size] for i in range(0, len(raw) - size + 1, size)]
    return list(zip(times, frames))


def difference(a, b):
    return sum(abs(x - y) for x, y in zip(a, b)) / len(a)


def video_lags(events, video_path, ffmpeg, windows, threshold=1.5):
    lags = []
    for start, end in windows:
        focus = [e['t'] for e in events if e['kind'] == 'focus' and start <= e['t'] <= end]
        if not focus:
            continue
        frames = read_frames(ffmpeg, video_path, start - 0.3, end - start + 0.3 + SEARCH_SECONDS)
        for t in focus:
            before = [f for f in frames if f[0] <= t]
            after = [f for f in frames if t < f[0] <= t + SEARCH_SECONDS]
            if not before:
                continue
            reference = before[-1][1]
            changed = next((f for f in after if difference(reference, f[1]) >= threshold), None)
            if changed:
                lags.append(dict(t=t, lag=changed[0] - t))
    return lags


def summary(lags):
    values = [x['lag'] for x in lags]
    if not values:
        return dict(count=0)
    quartiles = statistics.quantiles(values, n=4) if len(values) > 1 else [values[0]] * 3
    return dict(count=len(values), median=statistics.median(values), q1=quartiles[0], q3=quartiles[2],
                minimum=min(values), maximum=max(values))


def marker_windows(events, label='sync', seconds=30.0):
    """Windows after markers naming `label`; otherwise the opening window, where the sync routine is recorded."""
    markers = [e for e in events if e['kind'] == 'marker']
    chosen = [e for e in markers if label in str(e.get('label', '')).lower()] or markers[:1]
    return [(e['t'], e['t'] + seconds) for e in chosen]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('session', type=Path)
    parser.add_argument('--window', nargs=2, type=float, action='append', metavar=('START', 'END'),
                        help='Session-second windows; default: 30 s after each "sync" marker, else after session start')
    parser.add_argument('--no-video', action='store_true')
    parser.add_argument('--ffmpeg', default='ffmpeg')
    args = parser.parse_args()
    folder = args.session.resolve()
    manifest = json.loads((folder / 'session.json').read_text(encoding='utf-8'))
    events = index_events(folder, manifest)
    windows = args.window or marker_windows(events)
    if not windows:
        parser.error('No windows: add a "Sync" marker while recording or pass --window')
    audio = audio_lags(events, folder / 'audio.flac', manifest['audio']['first_t'], args.ffmpeg, windows)
    video = [] if args.no_video else video_lags(events, folder / 'video.mkv', args.ffmpeg, windows)
    result = dict(windows=windows, audio_request_to_onset=summary(audio), focus_to_frame_change=summary(video),
                  audio=audio, video=video, note=__doc__.strip().splitlines()[0] + ' Estimates only.')
    write_json(folder / 'sync.json', result)
    print(json.dumps({k: result[k] for k in ['audio_request_to_onset', 'focus_to_frame_change']}, indent=2))


if __name__ == '__main__':
    main()
