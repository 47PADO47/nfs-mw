"""Index a continuous recording without requiring another game recording."""
import argparse
import html
import json
from pathlib import Path
import subprocess

from .common import hidden_process, write_json
from .media import probe


def event_time(event, origin_ns, frequency):
    if 'qpc_ticks' not in event or not frequency:
        return None
    return event['qpc_ticks'] / frequency - origin_ns / 1e9


def index_events(folder, manifest):
    result = []
    with (folder / 'events.jsonl').open(encoding='utf-8') as stream:
        for line in stream:
            item = json.loads(line)
            if item['kind'] in ['marker', 'input', 'pointer', 'xinput', 'audio_gap', 'audio_status']:
                result.append(item)
    original = folder / 'og-ui.jsonl'
    if not original.exists():
        return sorted(result, key=lambda e: e['t'])
    frequency = None
    pending = []
    with original.open(encoding='utf-8') as stream:
        for line in stream:
            item = json.loads(line)
            if item['kind'] == 'probe_clock_anchor':
                frequency = item['qpc_hz']
            if item['kind'] not in ['probe_identity', 'package_state', 'heartbeat', 'probe_clock_anchor', 'probe_summary']:
                pending.append(item)
    for item in pending:
        timestamp = event_time(item, manifest['clock']['origin_perf_ns'], frequency)
        if timestamp is None:
            continue
        item['t'] = timestamp
        result.append(item)
    return sorted(result, key=lambda e: e['t'])


def make_preview(folder, ffmpeg):
    path = folder / 'review.mp4'
    if path.exists():
        return
    command = [ffmpeg, '-hide_banner', '-loglevel', 'warning', '-n', '-i', str(folder / 'recording.mkv'),
               '-map', '0:v:0', '-map', '0:a:0', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k',
               '-movflags', '+faststart', str(path)]
    with (folder / 'preview.log').open('wb') as log:
        result = hidden_process(command, stdout=log, stderr=log).wait()
    if result:
        raise RuntimeError('Review MP4 failed; lossless master remains available')


PAGE = '''<!doctype html><meta charset="utf-8"><title>UI reference session</title>
<style>
body{font:16px system-ui;background:#17191e;color:#eee;margin:24px;max-width:1600px}
video{width:100%;max-height:65vh;background:#000}button,input,select{font:inherit;margin:8px;padding:7px}
.layout{display:grid;grid-template-columns:2fr 1fr;gap:20px}#rows{max-height:70vh;overflow:auto}
button.event{display:block;text-align:left;width:96%;background:#252b36;color:#eee;border:1px solid #576}
pre{white-space:pre-wrap;font-size:12px}p{color:#bbb}
</style>
<h1>UI reference session</h1>
<p>__STATE__. Native video + lossless audio are in recording.mkv. This MP4 uses AAC for browser playback.
Event times use the session clock. Frame capture and audio latency are not calibrated; requests do not prove playback.</p>
<div class="layout"><section><video id="player" controls preload="metadata" src="review.mp4"></video>
<p id="clock"></p><pre id="detail">Select an event to seek and inspect its recorded fields.</pre></section>
<section><input id="search" placeholder="Find a package, message or marker">
<select id="kind"><option value="navigation">Screens, focus and markers</option><option value="sound">Sound requests</option>
<option value="script_start">Animation starts</option><option value="input">Keyboard/pointer input</option>
<option value="xinput">Physical XInput</option><option value="all">All indexed events</option></select>
<div id="rows"></div></section></div>
<script>
const data=__DATA__, origin=__ORIGIN__, player=document.querySelector('#player');
const detail=document.querySelector('#detail'), rows=document.querySelector('#rows');
function label(e){return e.label||e.package?.name||e.package?.resolved_name||e.package?.name_hash||e.message||e.object?.name||''}
function render(){rows.replaceChildren();const query=document.querySelector('#search').value.toLowerCase();
const kind=document.querySelector('#kind').value;let count=0;
for(const e of data){if(kind==='navigation'&&!['marker','package_load','package_unload','focus','audio_gap','audio_status'].includes(e.kind))continue;
if(!['navigation','all'].includes(kind)&&e.kind!==kind)continue;
if(query&&!JSON.stringify(e).toLowerCase().includes(query))continue;
if(++count>1500){rows.append('Refine the search to see more events.');break;}
const button=document.createElement('button');button.className='event';
button.textContent=`${e.t.toFixed(3)}s  ${e.kind}  ${label(e)}`;
button.onclick=()=>{player.currentTime=Math.max(0,e.t-origin-0.25);detail.textContent=JSON.stringify(e,null,2)};
rows.append(button)}}
document.querySelector('#search').oninput=render;document.querySelector('#kind').onchange=render;
player.ontimeupdate=()=>{document.querySelector('#clock').textContent=`Session ${(player.currentTime+origin).toFixed(3)}s`};render();
</script>'''


def build(folder, ffmpeg='ffmpeg', ffprobe='ffprobe', preview=True):
    folder = Path(folder).resolve()
    manifest = json.loads((folder / 'session.json').read_text(encoding='utf-8'))
    events = index_events(folder, manifest)
    write_json(folder / 'index.json', events)
    if preview:
        make_preview(folder, ffmpeg)
    media = manifest.get('media', {})
    origin = float(media.get('format', {}).get('start_time', 0))
    serialized = json.dumps(events, separators=(',', ':')).replace('<', '\\u003c')
    page = PAGE.replace('__DATA__', serialized).replace('__ORIGIN__', str(origin))
    page = page.replace('__STATE__', html.escape(manifest['state']))
    (folder / 'review.html').write_text(page, encoding='utf-8')
    write_json(folder / 'review-summary.json', dict(events=len(events), media_origin_seconds=origin,
               review_mp4=(folder / 'review.mp4').exists(), note='Event seek includes a 250 ms lead-in'))
    return events


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('session', type=Path)
    parser.add_argument('--no-preview', action='store_true')
    args = parser.parse_args()
    build(args.session, preview=not args.no_preview)


if __name__ == '__main__':
    main()
