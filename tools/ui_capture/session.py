"""One continuous reference session. All generated data lives outside Git."""
import argparse
from datetime import datetime, timezone
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import threading
import time

from .audio import Audio
from .common import Timeline, digest, external_output, write_json
from .media import Video, mux, probe


def installation_identity(exe):
    root = exe.parent
    candidates = [exe]
    for base in [root, root / 'scripts']:
        for pattern in ['*.asi', '*.dll', '*.ini', '*.json', '*.tpk']:
            candidates.extend(base.glob(pattern))
    for base in [root / 'FRONTEND', root / 'GLOBAL']:
        for pattern in ['*.bun', '*.lzc']:
            candidates.extend(base.glob(pattern))
    return [dict(path=str(path.relative_to(root)), bytes=path.stat().st_size, sha256=digest(path))
            for path in sorted(set(candidates)) if path.is_file()]


class Session:
    def __init__(self, args):
        from .windows import Hotkeys, Inputs, windows
        self.args = args
        matches = windows(args.exe)
        if len(matches) != 1:
            raise RuntimeError(f'Expected one visible game window, found {matches}')
        self.target = matches[0]
        checkout = Path(__file__).resolve().parents[2]
        root = external_output(args.output, checkout)
        root.mkdir(parents=True, exist_ok=True)
        self.folder = root / datetime.now(timezone.utc).strftime('session-%Y%m%dT%H%M%S.%fZ')
        self.folder.mkdir()
        self.timeline = Timeline(self.folder)
        self.stop_event = threading.Event()
        self.markers = []
        self.status = 'Preparing'
        self.video = self.audio = self.game_probe = self.hotkeys = None
        self.inputs = Inputs(self.target['hwnd'], self.timeline)
        self.manifest = dict(schema=1, state='preparing', reference=args.reference, window=self.target,
                             clock=self.timeline.header(), source=source_identity(checkout),
                             limitations=['Audio records the selected speaker output, including other applications',
                                          'Keyboard/pointer are sampled only while the target is foreground',
                                          'XInput observations do not cover DirectInput-only controllers',
                                          'Capture/audio timing requires an A/V calibration for frame-accurate claims'])
        write_json(self.folder / 'session.json', self.manifest)
        try:
            self.manifest['installation'] = installation_identity(args.exe)
            self.hotkeys = Hotkeys()
            self.audio = Audio(self.folder, self.timeline, args.ffmpeg, args.audio_device)
            self.audio.start()
            self.video = Video(self.folder, self.timeline, args.ffmpeg, self.target['hwnd'])
            if args.probe:
                spec = importlib.util.spec_from_file_location('private_game_probe', args.probe)
                module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(module)
                self.game_probe = module.GameProbe(self.target['pid'], self.folder / 'og-ui.jsonl', self._og_event)
                self.manifest['og_probe'] = self.game_probe.start()
            self.manifest.update(state='recording', commands=dict(video=self.video.command, audio=self.audio.command))
            self.marker('Session start')
            self.status = 'Recording'
            write_json(self.folder / 'session.json', self.manifest)
        except BaseException:
            self.finish('startup_error')
            raise

    def _og_event(self, item):
        # Keep full private records in og-ui.jsonl; the shared timeline indexes low-volume transitions only.
        if item.get('kind') in ['package_load', 'package_unload', 'focus', 'probe_error', 'probe_message']:
            self.timeline.emit('og_index', event=item)

    def marker(self, label=None):
        label = label or f'Marker {len(self.markers)}'
        item = self.timeline.emit('marker', label=label)
        self.markers.append(item)
        write_json(self.folder / 'markers.json', self.markers)

    def poll(self):
        from .windows import u
        for action in self.hotkeys.poll():
            if action == 1:
                self.marker()
            if action == 2:
                self.stop_event.set()
        self.inputs.sample()
        if not u.IsWindow(self.target['hwnd']):
            return 'game_closed'
        if self.video.process.poll() is not None:
            return 'video_encoder_exited'
        if self.audio.error or self.audio.process.poll() is not None:
            return 'audio_encoder_failed'
        if self.audio.dropped_frames:
            return 'audio_queue_overflow'
        if self.game_probe is not None and self.game_probe.status['state'] == 'error':
            return 'probe_failed'
        elapsed = self.timeline.seconds()
        if self.args.seconds and elapsed >= self.args.seconds:
            return 'duration_limit'
        if elapsed > 20 and self.video.first_frame is None:
            return 'no_video_frames'
        if elapsed > 5 and self.audio.first_t is None:
            return 'no_audio_callbacks'
        if int(elapsed) != getattr(self, 'last_second', -1):
            self.last_second = int(elapsed)
            free = shutil.disk_usage(self.folder).free
            drift = (time.time_ns() - self.timeline.unix_ns) / 1e9 - elapsed
            self.timeline.emit('health', free_bytes=free, wallclock_drift_seconds=drift)
            raw_bytes = sum(path.stat().st_size for path in [self.folder / 'video.mkv', self.folder / 'audio.flac']
                            if path.exists())
            # Finalization retains originals and writes a combined master and viewing copy.
            if free < self.args.reserve_gib * 1024 ** 3 + raw_bytes * 2:
                return 'disk_reserve'
            write_json(self.folder / 'live.json', dict(state=self.status, t=elapsed,
                       video_frames=self.video.frames, audio_frames=self.audio.frames,
                       markers=len(self.markers), free_gib=round(free / 1024 ** 3, 2)))
        return None

    def finish(self, reason):
        self.status = 'Finalizing'
        self.timeline.emit('stop', reason=reason)
        failures = []
        for name, component in [('og_probe', self.game_probe), ('video', self.video), ('audio', self.audio)]:
            if component is None:
                continue
            try:
                self.manifest[name] = component.stop()
            except Exception as exc:
                failures.append(f'{name}: {exc}')
        if self.hotkeys:
            self.hotkeys.close()
        duration = self.timeline.seconds()
        self.manifest.update(duration=duration, reason=reason, markers=self.markers)
        if self.video and (self.video.frames == 0 or self.video.process.returncode):
            failures.append('Video encoder failed or recorded zero frames')
        if self.audio and (self.audio.frames == 0 or self.audio.process.returncode or self.audio.error):
            failures.append('Audio encoder failed or recorded zero samples')
        if self.audio and (self.audio.dropped_frames or self.audio.status_count):
            failures.append('Audio callback reported loss/discontinuity; inspect the clock ledger')
        if self.video and self.audio and not failures:
            try:
                self.manifest['mux_command'] = mux(self.folder, self.args.ffmpeg, self.audio.first_t,
                                                  self.markers, duration)
                self.manifest['media'] = probe(self.args.ffprobe, self.folder / 'recording.mkv')
            except Exception as exc:
                failures.append(str(exc))
        self.manifest.update(state='finished' if not failures else 'incomplete', errors=failures)
        if reason not in ['user_stop', 'duration_limit', 'game_closed', 'disk_reserve']:
            self.manifest['state'] = 'incomplete'
        self.timeline.emit('finalized', state=self.manifest['state'], errors=failures)
        self.timeline.file.close()
        write_json(self.folder / 'session.json', self.manifest)
        if self.manifest['state'] == 'finished':
            try:
                from .review import build
                build(self.folder, self.args.ffmpeg, self.args.ffprobe)
            except Exception as exc:
                self.manifest['review_error'] = str(exc)
                write_json(self.folder / 'session.json', self.manifest)
        write_json(self.folder / 'live.json', dict(state=self.manifest['state'], errors=failures))
        self.status = self.manifest['state']
        return self.manifest


def source_identity(checkout):
    import subprocess
    def git(*words):
        result = subprocess.run(['git', '-C', str(checkout), *words], capture_output=True, text=True,
                                creationflags=subprocess.CREATE_NO_WINDOW)
        return result.stdout.strip()
    untracked = git('ls-files', '--others', '--exclude-standard').splitlines()
    return dict(head=git('rev-parse', 'HEAD'), status=git('status', '--short'),
                tracked_diff_sha256=__import__('hashlib').sha256(git('diff', 'HEAD').encode()).hexdigest(),
                untracked={name: digest(checkout / name) for name in untracked})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--reference', default='modded-pc')
    parser.add_argument('--ffmpeg', default='ffmpeg')
    parser.add_argument('--ffprobe', default='ffprobe')
    parser.add_argument('--audio-device', type=int)
    parser.add_argument('--probe', type=Path)
    parser.add_argument('--seconds', type=float, default=0)
    parser.add_argument('--reserve-gib', type=float, default=3)
    parser.add_argument('--headless', action='store_true')
    args = parser.parse_args()
    session = Session(args)
    print(json.dumps(dict(session=str(session.folder), target=session.target)), flush=True)
    if not args.headless:
        from .panel import run
        return run(session)
    reason = 'user_stop'
    try:
        while not session.stop_event.wait(1 / 120):
            problem = session.poll()
            if problem:
                reason = problem
                break
    except KeyboardInterrupt:
        pass
    result = session.finish(reason)
    print(json.dumps(dict(state=result['state'], folder=str(session.folder), errors=result['errors'])), flush=True)
    return 0 if result['state'] == 'finished' else 1


if __name__ == '__main__':
    sys.exit(main())
