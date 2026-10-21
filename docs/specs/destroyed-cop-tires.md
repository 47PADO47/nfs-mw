# Destroyed police coasting tires

Destroyed non-player police using the Simple chassis select a caller-configured
no-powered-slip tire response. Players, live police, ordinary traffic and trailers
retain their existing choices. Retirement controls remain zero gas, steering and
handbrake; the original quarter brake above 2.5 m/s is unchanged. Body sleeping
remains a separate support/contact policy.

For this response ordinary brakes act directly on all wheels, with authored brake
torque scaled by 10 rather than the racer scale 4. Engine braking is split equally
between supported tires of each authored axle; racer differential/traction assists
are bypassed. Loaded tires retain rolling angular velocity and zero longitudinal
slip. Lateral grip ramps with the quantized absolute slip angle to 12 degrees,
times grip boost, 1.54, load, authored grip scale and traction boost. Combined
forces are capped at load times static grip times traction boost times
(1 + 2.5 times brake). Corner drag uses absolute forward speed above 1 m/s and
coefficient 0.5; below that, lateral force scales by min(abs lateral speed, 1).
Surface drive/lateral multipliers apply only to unbraked rear tires. Ordinary
brakes never invoke racer wheel-lock/history or friction-ellipse integration.

This is the native AllowSlip=false, handbrake=false loaded branch, selected only
for retirement. It adds no artificial drag, speed cap, increased retirement brake,
or forced stop. Unsupported tire spin remains the existing implementation and is
not an assertion of complete Simple suspension parity. The existing common
chassis speed scales are retained. Native rear oversteer correction, powered
Simple tire slip/wheel integration, complete Simple engine/gearbox behavior,
catch-up aerodynamics and suspension DoSleep remain separate gaps. Whole pursuit
trajectories and the user's highway incident are not proven by scalar vectors.
