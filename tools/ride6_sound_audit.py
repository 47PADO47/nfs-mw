#!/usr/bin/env python3
"""Audit and repair the engine layers of the converted RIDE 6 bikes (docs/specs/ride6-engine-sound.md, section 9).

Reads the manifests of an installed `ride6-bikes` mod folder (the `[engine_sound]` tables and the WAV layers they name),
never writes into it. Needs numpy.

  audit      --mod <mod folder>                  levels, pitch coverage and layers (under load and off load) of every engine
  harmonics  --mod <mod folder>                  the share of each layer's energy on the harmonics of its engine cycle
  fix-levels --mod <mod folder> --out <folder>   write the layers whose level is stale, levelled, under <folder>/files/...
  sweep      --mod <mod folder> --car <id> [--set under|off] [--wav out.wav] [--step 250] [--rate-range 0.3,3.3]
                                                 render the game's layer mix over an rpm sweep and print the pitch error
(--mod is a mod folder or the sound-bikes output: `cars/*.toml` with `[engine_sound]` tables and `files/`.)
"""

import argparse
import glob
import math
import os
import re
import tomllib
import wave

import numpy as np

# The playback rate range of the game's mixer (`RATE_RANGE` of crates/nfsmw/src/audio/car/own_engine/mixing.rs; the range
# before the fix was 0.55 to 1.8, `--rate-range 0.55,1.8`).
RATE_RANGE = [0.3, 3.3]
OFF_LEVEL_DB = 3.0
OFF_LOAD = set()  # the files of the layers off load (`load = "off"`), filled by engines()
PEAK = 0.9


def load_wav(path):
    with wave.open(path) as w:
        frames, rate, channels = w.getnframes(), w.getframerate(), w.getnchannels()
        data = np.frombuffer(w.readframes(frames), dtype="<i2").astype(np.float64) / 32768
    return data.reshape(-1, channels).mean(axis=1), rate


def save_wav(path, samples, rate):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    pcm = np.clip(np.round(samples * 32768), -32768, 32767).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(pcm.tobytes())


def manifests(mod):
    for path in sorted(glob.glob(os.path.join(mod, "cars", "*.toml"))):
        with open(path, "rb") as f:
            table = tomllib.load(f)
        sound = table.get("engine_sound")
        if sound:
            yield os.path.basename(path), table, sound


def engines(mod):
    """engine folder -> (first manifest name, sound table, {layer index, file: rpm})"""
    found = {}
    for name, _, sound in manifests(mod):
        first = sound["layers"][0]["file"]
        folder = re.search(r"ENG/([^/]+)/", first)
        key = folder.group(1) if folder else first
        entry = found.setdefault(key, [name, sound, {}])
        for index, layer in enumerate(sound["layers"]):
            entry[2][(index, layer["file"])] = layer["rpm"]
            if layer.get("load") == "off":
                OFF_LOAD.add(layer["file"])
    return found


def is_off(layer):
    return layer.get("load") == "off"


def db(x):
    return 20 * math.log10(float(np.sqrt((x**2).mean())) + 1e-12)


def reference_level(levels):
    """The level of the engine: the median of its layers after the first two (the idle and the first loop)."""
    body = [v for (index, _), v in sorted(levels.items()) if index >= 2]
    return float(np.median(body if body else list(levels.values())))


def audit(args):
    stale = holes = without_off = 0
    two_stroke = []
    worst_gap = (0.0, "")
    levels_all = []
    for key, (name, sound, files) in engines(args.mod).items():
        levels = {k: db(load_wav(os.path.join(args.mod, "files", k[1]))[0]) for k in files}
        ref = reference_level(levels)
        levels_all += [v - ref for v in levels.values()]
        off = [(k[0], round(v - ref, 1)) for k, v in sorted(levels.items()) if abs(v - ref) > OFF_LEVEL_DB]
        stale += len(off)
        holes += any(d < -6 for _, d in off)
        under = sorted(r for k, r in files.items() if k[1] not in OFF_LOAD)
        free = sorted(r for k, r in files.items() if k[1] in OFF_LOAD)
        without_off += not free
        ratio = under[-1] / sound["redline_rpm"]
        if ratio > 1.5:
            two_stroke.append((key, round(ratio, 2)))
        gap = max(b / a for a, b in zip(under, under[1:]))
        worst_gap = max(worst_gap, (gap, key))
        spread = max(levels.values()) - min(levels.values())
        print(
            f"{key:45} layers {len(under):2}+{len(free):2} level {ref:6.1f} dB spread {spread:4.1f} dB "
            f"off-level {off} widest gap x{gap:.2f} top/redline {ratio:.2f}"
        )
    print(f"off-level layers (more than {OFF_LEVEL_DB} dB from the engine's): {stale}; engines with a layer more than 6 dB under: {holes}")
    print(f"engines without layers off load: {without_off}; widest gap between two layers under load: x{worst_gap[0]:.2f} ({worst_gap[1]})")
    print(f"level of every layer against its engine: min {min(levels_all):.1f} dB, max {max(levels_all):.1f} dB")
    print(f"top loop above 1.5 x redline (cycle counted as one turn?): {two_stroke}")


def harmonics(args):
    """Section 9.2: the share of a layer's energy on the harmonics of its engine cycle (bins that are multiples of the cycle
    count, one bin either side); a layer whose exhaust was at another note has most of it on another cycle count."""
    low = total = fractional = long_clips = 0
    worst = []
    for key, (name, sound, files) in engines(args.mod).items():
        unit = 120.0
        top = max(r for k, r in files.items() if k[1] not in OFF_LOAD)
        if top / sound["redline_rpm"] > 1.5:
            unit = 60.0
        shares = []
        for (index, path), rpm in sorted(files.items()):
            if index == 0 or path in OFF_LOAD:
                continue
            x, rate = load_wav(os.path.join(args.mod, "files", path))
            exact = rpm / unit * len(x) / rate
            cycles = round(exact)
            if abs(exact - cycles) > 0.1:
                fractional += 1  # a loop of whole turns at 60 per cycle per second: the cycle count is not an integer
                continue
            if cycles > 100:
                long_clips += 1  # a long clip rather than a loop: too many bins to tell the comb from the rest
                continue
            spectrum = np.abs(np.fft.rfft(x)) ** 2
            bins = np.arange(cycles, len(spectrum) - 1, cycles)
            on = sum(spectrum[bins + d].sum() for d in (-1, 0, 1)) + spectrum[0]
            shares.append(on / spectrum.sum())
        low += sum(share < 0.5 for share in shares)
        total += len(shares)
        worst.append((min(shares) if shares else 1.0, key))
    worst.sort()
    print(f"layers with under half of their energy on the engine's harmonics: {low} of {total} (idle layers not counted; "
          f"{fractional} layers with a fractional cycle count and {long_clips} clips of over 100 cycles skipped)")
    print("lowest shares:", [(round(share, 2), key) for share, key in worst[:5]])


def fix_levels(args):
    written = 0
    for key, (name, sound, files) in engines(args.mod).items():
        audio = {k: load_wav(os.path.join(args.mod, "files", k[1])) for k in files}
        levels = {k: db(a[0]) for k, a in audio.items()}
        ref = reference_level(levels)
        for k, (samples, rate) in audio.items():
            if abs(levels[k] - ref) <= 1.0:
                continue
            gain = min(10 ** ((ref - levels[k]) / 20), PEAK / max(float(np.abs(samples).max()), 1e-9))
            save_wav(os.path.join(args.out, "files", k[1]), samples * gain, rate)
            written += 1
    print(f"{written} layers written under {os.path.join(args.out, 'files')}")


def mix(rpms, rpm):
    """The game's mix of layers by speed (`by_speed` of mixing.rs): [(index, gain, rate)]."""
    n = len(rpms)
    rate = lambda i: min(max(rpm / rpms[i], RATE_RANGE[0]), RATE_RANGE[1])
    above = next((i for i, r in enumerate(rpms) if r >= rpm), None)
    if above is None:
        return [(n - 1, 1.0, rate(n - 1))]
    if above == 0:
        return [(0, 1.0, rate(0))]
    below = above - 1
    t = min(max(math.log(rpm / rpms[below]) / math.log(rpms[above] / rpms[below]), 0), 1)
    return [(below, math.cos(t * math.pi / 2), rate(below)), (above, math.sin(t * math.pi / 2), rate(above))]


def render(layers, rpm_of_t, seconds, rate, block=480):
    rpms = [r for r, _ in layers]
    out = np.zeros(int(seconds * rate / block) * block)
    pos = [0.0] * len(layers)
    for b in range(len(out) // block):
        for i, gain, speed in mix(rpms, rpm_of_t(b * block / rate)):
            x = layers[i][1]
            p = pos[i] + np.arange(block) * speed
            i0 = np.floor(p).astype(np.int64)
            a, c = x[i0 % len(x)], x[(i0 + 1) % len(x)]
            out[b * block : (b + 1) * block] += gain * (a + (c - a) * (p - i0))
            pos[i] = (pos[i] + block * speed) % len(x)
    return out


def cycle_frequency(x, rate, f0):
    """The engine-cycle frequency near f0 by a harmonic comb: (Hz, share of the energy on the comb)."""
    spectrum = np.abs(np.fft.rfft(x * np.hanning(len(x))))
    best = (0.0, 0.0)
    for f in np.arange(f0 * 0.7, f0 * 1.4, f0 * 0.0015):
        index = np.round(np.arange(1, int(3500 / f) + 1) * f * len(x) / rate).astype(int)
        energy = sum(spectrum[np.clip(index + d, 0, len(spectrum) - 1)] ** 2 for d in (-1, 0, 1)).sum()
        best = max(best, (f, energy), key=lambda pair: pair[1])
    return best[0], best[1] / (spectrum**2).sum()


def sweep(args):
    _, table, sound = next(
        (m for m in manifests(args.mod) if m[0] in (args.car + ".toml", args.car + ".sound.toml")), (None, None, None)
    )
    if sound is None:
        raise SystemExit(f"no manifest {args.car}.toml or {args.car}.sound.toml with an [engine_sound] in {args.mod}")
    layers = []
    rate = 48000
    for layer in sorted(sound["layers"], key=lambda item: item["rpm"]):
        if is_off(layer) != (args.set == "off"):
            continue
        samples, rate = load_wav(os.path.join(args.mod, "files", layer["file"]))
        layers.append((layer["rpm"], samples))
    print(f"{table.get('name', args.car)}: idle {sound['idle_rpm']} redline {sound['redline_rpm']}, {args.set}-load layers at {[round(r) for r, _ in layers]}")
    print("rpm   level_dB  cycle_Hz  expected  error_cents  comb_share")
    rpm = sound["idle_rpm"]
    held = []
    while rpm <= sound["redline_rpm"]:
        y = render(layers, lambda t: rpm, 1.3, rate)[int(0.8 * rate) :]
        f, share = cycle_frequency(y, rate, rpm / 120)
        print(f"{rpm:6.0f} {db(y):7.1f} {f:9.2f} {rpm / 120:9.2f} {1200 * math.log2(f / (rpm / 120)):8.1f} {share:8.2f}")
        held.append((rpm, db(y), 1200 * math.log2(f / (rpm / 120))))
        rpm += args.step
    levels = np.array([level for _, level, _ in held])
    low, high = np.percentile(levels, [2, 98])
    # The idle recording of most engines has no clear period (spec 9.5): its measured pitch is not an error of the mix, so
    # the pitch is judged from the first loop up (the rows at or above the second layer) and from the second loop up.
    def worst(first):
        errors = [abs(error) for rpm, _, error in held if rpm >= layers[min(first, len(layers) - 1)][0]]
        return (max(errors), float(np.median(errors))) if errors else (0.0, 0.0)

    (first_max, first_median), (second_max, _) = worst(1), worst(2)
    print(
        f"summary: level min {levels.min():.1f} max {levels.max():.1f} dB (2nd-98th percentile {low:.1f}..{high:.1f}, "
        f"span {high - low:.1f} dB); pitch error from the first loop up: max {first_max:.0f} cents, median {first_median:.0f}; "
        f"from the second loop up: max {second_max:.0f}; rate range {RATE_RANGE[0]}..{RATE_RANGE[1]}"
    )
    if args.wav:
        span = sound["redline_rpm"] - sound["idle_rpm"]
        full = render(layers, lambda t: sound["idle_rpm"] + span * min(t / 20, 1), 20, rate)
        save_wav(args.wav, full / max(np.abs(full).max(), 1e-9) * PEAK, rate)
        print(f"sweep of 20 s written to {args.wav}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["audit", "harmonics", "fix-levels", "sweep"])
    parser.add_argument("--mod", required=True)
    parser.add_argument("--out")
    parser.add_argument("--car")
    parser.add_argument("--wav")
    parser.add_argument("--set", choices=["under", "off"], default="under", help="sweep: the layers under load or off load")
    parser.add_argument("--step", type=float, default=250.0)
    parser.add_argument("--rate-range", default="0.3,3.3", help="lowest,highest playback rate of a layer")
    args = parser.parse_args()
    RATE_RANGE[:] = [float(x) for x in args.rate_range.split(",")]
    if args.command == "fix-levels" and not args.out:
        parser.error("fix-levels needs --out")
    if args.command == "sweep" and not args.car:
        parser.error("sweep needs --car")
    {"audit": audit, "harmonics": harmonics, "fix-levels": fix_levels, "sweep": sweep}[args.command](args)


if __name__ == "__main__":
    main()
