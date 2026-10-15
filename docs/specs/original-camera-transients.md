# Original gameplay camera transient offsets

Apply these retained responses after reading current follow cubics and mirroring
local horizontal offsets for lookback, before transforming eye and target to the
world. They precede under-vehicle displacement, noise and clearance. They do not
modify the follow spline targets or the rendered FOV.

## Collision response

Use the instantaneous nonlinear world acceleration response from the follow
spec, before its five-sample average. If `max(abs(x),abs(y)) >= 24` and the
anchor's normalized world collision damping exceeds the retained peak, replace
the peak and restart a one-second remaining timer. While peak and remaining
time are positive, sample this linear response with remaining time as its input:

| Remaining seconds | 0 | 0.25 | 0.5 | 0.75 | 1 |
|---|---|---|---|---|---|
| Response | 0 | 0.02 | 0.25 | 0.99 | 0 |

Let `amount = response * peak * 0.25`. Multiply local eye.x by `1-amount` and
local target.x by `1+amount`. Then subtract real camera elapsed from the timer
when positive. Clear the peak when the resulting timer is at or below zero.
This response applies to both inside and outside modes. A weaker impact does
not restart an already retained stronger pulse.

The stock update also retains a vertical acceleration peak
`clamp((mapped_acceleration.z-24)/6,0,1)` with a one-second timer. Its five-point
vertical displacement response is identically zero in this PC executable, and
has no write references. Preserve its stock zero visual effect. The normalized
ground/object impact anchor fields do not feed this checked displacement block;
do not substitute them for the actual acceleration producer.

## Gear response

The camera action compares the current transmission gear integer with its
previous value. Any change produces an edge; gear numbering and shift direction
do not otherwise enter this effect. Seed the host's previous gear when attaching
to a car. Only outside camera modes admit and apply the pulse. If its elapsed
time is at least 1.5 seconds, a gear edge at speed strictly above 10 m/s starts
the pulse at zero. Edges during an active pulse are discarded.

Sample this linear response at elapsed seconds:

| Seconds | 0 | 0.1875 | 0.375 | 0.5625 | 0.75 | 0.9375 | 1.125 | 1.3125 | 1.5 |
|---|---|---|---|---|---|---|---|---|---|
| Response | 0 | 0.2 | 0.13 | -0.08 | -0.2 | -0.13 | -0.07 | -0.03 | 0 |

Add `response * abs(local_eye.x) * 0.2 * (1-clamp(speed*0.01,0,1))`
to local eye.x. Target remains unchanged. A pulse admitted on the current update
has no immediate displacement; the first nonzero response occurs on a later
update. Elapsed uses the real camera clock, independently of Speedbreaker world
time. The original clock has 0.00025-second timestamp granularity; the host uses
continuous fixed-step elapsed, consistent with the follow clock adaptation.
