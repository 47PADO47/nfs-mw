# AI racers: catch-up

The driver adds a signed catch-up offset to its base skill. Its consumers are in
[driver speed and skill](ai-driver-speed-skill.md), section 8. PC identity and
evidence are recorded in [provenance](../provenance/ai-racers-catchup.md).

## 1. Custom-race switch

Use the selected switch when present, otherwise the stored race switch. Off
leaves a physically simulated rival's offset at zero. Native off-world emulation
can force quick-race catch-up regardless of the switch; retain sampled history
and clear the forced offset when the car returns to physical Off simulation.
The base-skill table still distinguishes Hard with On and Off. Split-screen
races retain their separate policy of disabling catch-up between people.

## 2. Error history

Each non-human rival records an error about once per second while racing and
released from staging **[decomp, PC static]**:

```
error = route_length / 100 * (average_human_percent - rival_percent)
error *= 1 + (1 - human_top_speed_rating) * 0.5
```

Retain ten trapezoid areas and their sample intervals, plus five error slopes.
Start previous error at zero. For a sample after `dt` seconds:

```
area = (previous_error + error) / 2 * dt
slope = (error - previous_error) / dt
integral = sum(retained_areas) * sample_count / sum(retained_intervals)
derivative = mean(retained_slopes)
```

The frequency divisor is one. This is a finite history, not an integral over the
whole race. The original randomizes the initial sampling phase; the host uses
its own independent seeded stream. Evaluate output with current race progress between samples
as well; progress changes the tables **[PC static]**.

## 3. Ordinary quick-race output

These five-point tables span rival completion 0..100 percent, with points at
0, 25, 50, 75 and 100 percent **[PC static]**:

| Table | Values |
|---|---|
| Easy spread, metres | 1000, 900, 750, 600, 800 |
| Hard spread, metres | 300, 275, 250, 150, 250 |
| Easy skill amplitude | 0.75, 0.75, 0.75, 0.50, 0.25 |
| Hard skill amplitude | 0.75, 0.75, 0.75, 0.75, 0.25 |

Interpolate Easy to Hard with difficulty Easy=0, Medium=0.5, Hard=1. An emulated
off-world rival forces this factor to one.

```
spread = lerp(easy_spread(percent), hard_spread(percent), difficulty)
amplitude = lerp(easy_amplitude(percent), hard_amplitude(percent), difficulty)
normalized = clamp(2 * error / max(100, spread)
                   + 0.00005 * integral + 0.01 * derivative, -1, 1)
raw_skill_offset = normalized * amplitude
```

Amplitude limits both positive catch-up and negative slowdown. A Hard rival early
in the race has raw offset in `[-0.75, 0.75]`, not `[-1, 1]`
**[PC static, recorded runtime]**. This is not an extra top-speed multiplier.

Simulated rivals apply the existing positive/negative base-skill scaling.
Speed-trap races instead scale either sign by 0.5. Emulated rivals keep the raw
offset. The driver clamps `base + offset` to 0..1 and derives its force cheat
separately. Do not clamp away a positive offset before deriving that cheat.

## 4. Current limits

Quick races reuse `blackbox_driver::PidError`. Career races retain the older
`blackbox_race::CatchUp` approximation. Career adaptive difficulty, custom override
tables and the exact native random sequence still need separate integration.
Ordinary quick-race error conversion uses the existing normalized garage rating
of the human's actual performance package. Career retains its previous rating
fallback until per-stat adaptive matching is integrated.

## 5. Checks

Check finite-history expiry, derivative averaging, all difficulty endpoints,
progress interpolation, both offset signs and late-race amplitude reduction.
Check Off in a loaded race. A trajectory probe must report simulation elapsed
time as well as external time. Course completion alone does not prove pace parity.
