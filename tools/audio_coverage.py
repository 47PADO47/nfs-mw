"""Reconcile original module observations with remake controller evidence.

This reports implementation/runtime coverage. Unmatched drives never pass parity.
"""
import argparse
import collections
import json
from pathlib import Path


def records(path):
    with Path(path).open(encoding="utf-8-sig") as stream:
        for number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{number}: malformed trace") from error


def summarize(original, remake):
    native = collections.Counter()
    for record in original:
        module = record.get("module")
        if module:
            native[module] += 1
    inventory, events = {}, collections.defaultdict(collections.Counter)
    failures, fallbacks, first_inputs = [], [], {}
    starts = collections.Counter()
    active = set()
    end, sequence, gaps = None, 0, 0
    for record in remake:
        event = record.get("event")
        if event == "session_end":
            end = record
            continue
        current = record.get("sequence", 0)
        if current != sequence + 1:
            gaps += 1
        sequence = current
        module = record.get("class", "")
        if event == "coverage":
            inventory[module] = {"route": record.get("bank"), "limits": record.get("detail")}
            continue
        if module:
            events[module][event] += 1
        if event == "module_tick" and module not in first_inputs:
            first_inputs[module] = record.get("params")
        if event == "voice_start":
            starts[module] += 1
            active.add((record.get("instance"), record.get("player")))
        if event in ("voice_stop", "voice_end"):
            active.discard((record.get("instance"), record.get("player")))
        if event == "module_drop":
            active = {voice for voice in active if voice[0] != record.get("instance")}
        if event in ("voice_failure", "module_failure", "module_unavailable", "wet_patch_failure"):
            failures.append(record)
        if event == "fallback_command":
            fallbacks.append(record)
    complete = bool(end and not end.get("dropped") and not end.get("write_failed") and not gaps)
    families = []
    for module in sorted(set(native) | set(inventory) | set(events)):
        implementation = inventory.get(module, {"route": "unknown", "limits": "not classified in remake inventory"})
        status = "unobserved"
        if native[module]:
            status = "original_observed_remake_not_replayed"
        if events[module]["module_tick"]:
            status = "controller_replayed_original_comparison_pending"
        if starts[module]:
            status = "voices_started_original_comparison_pending"
        if any(f.get("class") == module for f in failures):
            status = "runtime_failure"
        families.append({"module": module, **implementation, "status": status,
                         "original_events": native[module], "remake_events": dict(events[module]),
                         "remake_voice_starts": starts[module], "first_class_words": first_inputs.get(module)})
    return {"schema": 1, "parity": "not_established", "trace_complete": complete,
            "sequence_gaps": gaps, "session_end": end, "families": families,
            "failures": failures[:100], "fallback_count": len(fallbacks),
            "first_fallbacks": fallbacks[:25], "active_at_last_record": len(active),
            "limits": ["Voice callback events are not counts of driving events.",
                       "Independent captures do not establish matched scenario or PCM parity.",
                       "Requested voice words do not establish equivalent effective backend DSP.",
                       "Absence in a bounded capture does not establish absence from the game."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original", required=True, type=Path)
    parser.add_argument("--remake", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    report = summarize(records(args.original), records(args.remake))
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"{len(report['families'])} families; trace complete={report['trace_complete']}; parity not established")


if __name__ == "__main__":
    main()
