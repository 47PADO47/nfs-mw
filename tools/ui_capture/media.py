"""Window-only video, separate durable sound, and an aligned viewing master."""
import json
import re
import subprocess
import threading
import time

from .common import hidden_process


class Video:
    def __init__(self, folder, timeline, ffmpeg, hwnd, fps=60):
        self.folder, self.timeline = folder, timeline
        self.first_frame = None
        self.last_progress = None
        self.frames = 0
        # gfxcapture's source PTS begins at zero. Timestamp the demuxed frames against
        # the recorder's UTC anchor, retaining the measured arrival time in the file.
        # This measures frame receipt, not the original game's Present timestamp.
        source = f'gfxcapture=hwnd={hwnd}:max_framerate={fps}:capture_border=0:resize_mode=scale_aspect'
        epoch = timeline.unix_ns / 1e9
        filters = f'setpts=PTS-{epoch:.9f}/TB'
        self.command = [ffmpeg, '-hide_banner', '-loglevel', 'info', '-n', '-copyts',
                        '-use_wallclock_as_timestamps', '1', '-f', 'lavfi', '-i', source,
                        '-an', '-vf', filters, '-c:v', 'h264_nvenc', '-preset', 'p4', '-tune', 'hq',
                        '-rc', 'vbr', '-cq', '18', '-b:v', '0', '-maxrate', '40M', '-bufsize', '80M',
                        '-g', str(fps * 2), '-fps_mode', 'vfr', '-avoid_negative_ts', 'disabled',
                        '-cluster_time_limit', '1000', '-flush_packets', '1', '-progress', 'pipe:1',
                        '-stats_period', '0.25', str(folder / 'video.mkv')]
        self.log = (folder / 'video-ffmpeg.log').open('wb')
        self.started_t = timeline.seconds()
        self.process = hidden_process(self.command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                      stderr=self.log, text=True, encoding='utf-8')
        self.reader = threading.Thread(target=self._progress, daemon=True)
        self.reader.start()

    def _progress(self):
        progress = {}
        for line in self.process.stdout:
            key, _, value = line.strip().partition('=')
            progress[key] = value
            if key != 'progress':
                continue
            self.frames = int(progress.get('frame', 0))
            if self.frames:
                self.last_progress = self.timeline.seconds()
                if self.first_frame is None:
                    self.first_frame = self.last_progress
            self.timeline.emit('video_progress', **progress)
            progress = {}

    def stop(self):
        if self.process.poll() is None:
            self.process.stdin.write('q\n')
            self.process.stdin.flush()
        try:
            self.process.wait(timeout=20)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=5)
        self.reader.join(timeout=2)
        self.log.close()
        return dict(command=self.command, exit=self.process.returncode, frames=self.frames,
                    start_t=self.started_t, first_progress_t=self.first_frame,
                    last_progress_t=self.last_progress,
                    clock='frame_demux_wallclock_relative_to_session_utc',
                    limitation='Frame receipt time includes uncalibrated WGC/driver/encoder pipeline latency')


def probe(ffprobe, path):
    process = hidden_process([ffprobe, '-v', 'error', '-show_streams', '-show_format', '-of', 'json', str(path)],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    output, error = process.communicate(timeout=30)
    if process.returncode:
        raise RuntimeError(error)
    return json.loads(output)


def chapters_file(path, markers, duration):
    rows = [';FFMETADATA1']
    for index, marker in enumerate(markers):
        start = max(0, int(marker['t'] * 1000))
        end_t = markers[index + 1]['t'] if index + 1 < len(markers) else duration
        end = max(start + 1, int(end_t * 1000))
        label = re.sub(r'[\r\n=;#\\]', ' ', marker['label'])
        rows.extend(['[CHAPTER]', 'TIMEBASE=1/1000', f'START={start}', f'END={end}', f'title={label}'])
    path.write_text('\n'.join(rows) + '\n', encoding='utf-8')


def mux(folder, ffmpeg, audio_start, markers, duration):
    if audio_start is None:
        raise RuntimeError('No audio callbacks were captured; raw video is preserved')
    metadata = folder / 'chapters.ffmeta'
    chapters_file(metadata, markers, duration)
    command = [ffmpeg, '-hide_banner', '-loglevel', 'warning', '-n', '-copyts',
               '-i', str(folder / 'video.mkv'), '-itsoffset', f'{audio_start:.9f}',
               '-i', str(folder / 'audio.flac'), '-f', 'ffmetadata', '-i', str(metadata),
               '-map', '0:v:0', '-map', '1:a:0', '-map_metadata', '2', '-map_chapters', '2',
               '-c', 'copy', '-avoid_negative_ts', 'disabled', str(folder / 'recording.mkv')]
    with (folder / 'mux.log').open('wb') as log:
        process = hidden_process(command, stdout=log, stderr=log)
        code = process.wait()
    if code:
        raise RuntimeError(f'Mux failed ({code}); original video/audio remain intact')
    return command
