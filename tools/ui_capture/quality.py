"""Check recorded media rather than trusting a recorder process exit code."""
import argparse
import json
import math
from pathlib import Path
import re
import statistics
import subprocess

from .common import hidden_process, write_json
from .media import probe


def inspect(folder, ffmpeg='ffmpeg', ffprobe='ffprobe'):
    folder = Path(folder)
    manifest = json.loads((folder / 'session.json').read_text(encoding='utf-8'))
    media = probe(ffprobe, folder / 'recording.mkv')
    video = next(s for s in media['streams'] if s['codec_type'] == 'video')
    audio = next(s for s in media['streams'] if s['codec_type'] == 'audio')
    frames_command = [ffprobe, '-v', 'error', '-select_streams', 'v:0', '-show_entries',
                      'frame=best_effort_timestamp_time', '-of', 'csv=p=0', str(folder / 'recording.mkv')]
    process = hidden_process(frames_command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    output, error = process.communicate()
    if process.returncode:
        raise RuntimeError(error)
    stamps = []
    for row in output.splitlines():
        if not row.strip():
            continue
        try:
            stamps.append(float(row.split(',')[0]))
        except ValueError:
            stamps.append(math.nan)
    gaps = [b - a for a, b in zip(stamps, stamps[1:])]
    timing_valid = (len(stamps) > 1 and all(math.isfinite(t) for t in stamps)
                    and all(math.isfinite(gap) and gap > 0 for gap in gaps)
                    and math.isfinite(stamps[-1] - stamps[0]))
    fps = (len(stamps) - 1) / (stamps[-1] - stamps[0]) if timing_valid else None
    median = statistics.median(gaps) if timing_valid else None
    if timing_valid and not all(math.isfinite(value) for value in (fps, median)):
        timing_valid = False
        fps = median = None
    command = [ffmpeg, '-hide_banner', '-nostats', '-i', str(folder / 'recording.mkv'),
               '-vn', '-sn', '-af', 'astats=metadata=0:reset=0', '-f', 'null', '-']
    process = hidden_process(command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    _, log = process.communicate()
    (folder / 'audio-analysis.log').write_text(log, encoding='utf-8')
    peaks = [float(v) for v in re.findall(r'Peak level dB: ([-+\w.]+)', log)]
    rms = [float(v) for v in re.findall(r'RMS level dB: ([-+\w.]+)', log)]
    audible_energy = bool(peaks) and any(math.isfinite(v) for v in peaks)
    report = dict(width=video['width'], height=video['height'], decoded_frames=len(stamps),
                  nominal_rate=video.get('r_frame_rate'),
                  video_timing_valid=timing_valid,
                  measured_mean_fps=fps, median_frame_interval=median,
                  largest_frame_interval=max(gaps) if timing_valid else None,
                  audio_codec=audio['codec_name'], audio_channels=audio['channels'],
                  audio_nonzero=audible_energy,
                  peak_db=peaks[-1] if peaks and math.isfinite(peaks[-1]) else None,
                  rms_db=rms[-1] if rms and math.isfinite(rms[-1]) else None,
                  audio_decode_exit=process.returncode,
                  audio_clock_drift_seconds=manifest['audio'].get('clock_drift_seconds'),
                  audio_discontinuities=manifest['audio'].get('discontinuities'),
                  audio_inserted_silence_frames=manifest['audio'].get('inserted_silence_frames'),
                  audio_content_coverage=manifest['audio'].get('evidence_complete', 'not_measured_by_old_capture'),
                  notes=['VFR gaps may reflect unchanged/paused windows; nominal rate is not measured coverage',
                         'Nonzero audio does not prove the expected effect was audible or correctly mixed',
                         'A/V onset alignment still needs calibration; this report does not claim it'])
    report['media_checks_passed'] = timing_valid and audible_energy and process.returncode == 0
    write_json(folder / 'quality.json', report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('session', type=Path)
    args = parser.parse_args()
    print(json.dumps(inspect(args.session), indent=2))


if __name__ == '__main__':
    main()
