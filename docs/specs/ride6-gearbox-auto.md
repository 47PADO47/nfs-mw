# RIDE 6 automatic gearbox decision and clutch feathering (clean-room spec)

The rules of the input handler (`UInputHandler`, the bike handler derives from it) that choose the gear and drive the clutch
input, written in our own words. It completes the "Automatic shifting" and "Launch clutch" parts of
[ride6-engine-drivetrain.md](ride6-engine-drivetrain.md) (the shift timing, clutch/throttle phases, the engagement model and
the table setup stay there). Data fields: [formats/ride6-physics-elements.md](../formats/ride6-physics-elements.md). Sources
(build `5.6.1-1697004`, image-relative): tick `0x1450d7000`, table generation `0x1450efa50`, table display setup `0x1450f2c70`,
rpm to speed `0x1450cdea0`, reference speed `0x1450f1d20`, shift request `0x1450c9e10`, hold-off factors `0x1450f1c20`,
launch clutch `0x1450d7cd0`, feathering `0x1450c0290`, engagement `0x1451502d0`, constructor `0x1450bd6d0`. Tags: **[B]**
binary, **[D]** data, **[I]** inferred. All speeds inside are cm/s, rpm is engine rpm, `MaxRPM` the engine's `MaxRPM`.

## 1. The shift points are generated, not read (when `bAutomaticGearChangePoints`)

The KTM 450 SMR uses `Motard_InputHandler`, which sets `TransmissionSettings.bAutomaticGearChangePoints = true`. At setup the handler **overwrites** the
`GearChangePoints` rows with points computed from the torque curve and the gear ratios [B: `0x1450efa50`]. The data values
`ChangeUp = 0`, `ChangeDown = 0` are placeholders, and `ChangeUpWithThrottle` (0.9) is **not used** in this mode (section 2).
Let `T(w)` be the normalised torque curve (the external curve `KTM_SMR_450_2022`, linear), `G_n` the forward ratios,
`r_n = G_(n+1) / G_n` (< 1), `w0 = MaxTorqueRPM` of the engine (the rpm of the curve maximum, 7159 for the KTM [I]),
`dw = (MaxRPM - MinRPM) * 0.01`, and `UpTol[n]`, `DownTol[n]` the `ChangeUpTolerance`, `ChangeDownTolerance` of gear `n`
(1-based). For each gear `n = 1 .. N-1` the sweep `w = w0, w0 + dw, ... < MaxRPM` computes:

```
ChangeUp[n]      = 1.0 initially
ChangeDown[n+1]  = 0 initially
for each w:   t1 = T(w) ;  w2 = w r_n ;  t2 = T(w2) r_n          # relative wheel force now and after the upshift
    if ChangeUp[n] == 1.0 and t1 UpTol[n] < t2:   ChangeUp[n] = w / MaxRPM                       # first rpm where the next gear wins
    tol = DownTol[n]  if bUseDeprecatedAutomaticTransmission (the C++ default, true)   else  DownTol[n+1]
    if ChangeDown[n+1] == 0  or  t2 < t1 tol:      ChangeDown[n+1] = w2 / MaxRPM                 # last rpm where the next gear loses
```

`ChangeUp[N]` and `ChangeDown[1]` stay as in the data (0). The `*WithThrottle` columns keep their data values. Afterwards
`GearChangeOverLimitEnabled` is copied to `bGearUpOverLimitEnabled`. [B]

KTM 450 numbers (ratios 2.0, 1.625, 1.333, 1.142, 0.954, tolerances up 1 / down 0.4, 0.9, 1, 1, 1, `MinRPM 2300`,
`MaxRPM 11000`), computed with the equations above and a linear torque curve (derived, not read from the running game):

| Gear | `ChangeUp` | rpm | `ChangeDown[n+1]` (deprecated mode) | rpm |
|---|---|---|---|---|
| 1 -> 2 | 0.9435 | 10378 | 0.5288 | 5817 |
| 2 -> 3 | 0.9435 | 10378 | 0.7285 | 8014 |
| 3 -> 4 | 0.9355 | 10291 | 0.7947 | 8742 |
| 4 -> 5 | 0.9435 | 10378 | 0.7815 | 8597 |

## 2. The decision (every tick, automatic transmission, clutch not held) [B: `0x1450d7000`]

Preconditions for any automatic change: not in the starting sequence (`InStartingSequence` false), current gear `g > 0`,
`TotalGearChangeTime <= 0` (no change in progress). Quantities:

```
v_fwd   = |VehicleForwardSpeed| ;  Speed = total speed
v       = v_fwd + (Speed - v_fwd) * AutomaticTransmissionSpeedFactor          # default 0.5, KTM handlers 0 -> v = v_fwd
Tq      = 0                                  if bAutomaticGearChangePoints (KTM)
        = the throttle action of the rider    otherwise                        # the data's with-throttle columns only act then
speed(rpm, n) = r_eff * rpm * (2 pi / 60) / (G_n * FinalRatio)               # cm/s;  0 if n <= 0
r_eff   = effective radius of the LAST wheel of the vehicle (the rear wheel) if bUseDeprecatedAutomaticTransmission (default true),
          otherwise the mean of the tractive wheels' effective radius ; at least 1e-4
```

Reference speeds [B: `0x1450f1d20`]: by default (the three switches `bEnableChangeSpeedCorrectionBasedOnSlip`,
`bEnableReferenceSpeedCorrectionBasedOnSlip`, `bEnableWheelSpeedChangeLogic` false, which is the case for the KTM)
`V_up = V_down = v`. With `bEnableChangeSpeedCorrectionBasedOnSlip` the references are `v (1 + k_up s)` and `v (1 + k_down s)`,
`s` the mean over the tractive wheels of the wheel's longitudinal slip clamped to its peak slips `[sx-, sx+]`, `k` =
`ChangeUp/DownSpeedCorrectionFactor` (default 1). The two other modes (a per-wheel slip-direction correction, and a wheel
speed versus slip threshold blend) are not needed for the bikes read.

Upshift (`g < N` and `NoGearUpChangeTimer == 0`):

```
a      = ChangeUp[g] + (ChangeUpWithThrottle[g] - ChangeUp[g]) Tq
a'     = a + (1 - a) PreventGearUpChangeFactor
V_thr  = speed(a' MaxRPM, g)                                 # speed at which the engine would reach a' MaxRPM in the CURRENT gear
request g+1 (change time GearUpChangeTime) if  V_thr < V_up
   or ( bGearUpOverLimitEnabled and TimeOverLimit > OverLimitChangeTime and TimeSinceLastGearChange > OverLimitChangeTimeThreshold and not burnout )
```

Downshift (`g > 1`, `TotalGearChangeTime <= 0`, `NoGearDownChangeTimer == 0`):

```
c      = ChangeDown[g] + (ChangeDownWithThrottle[g] - ChangeDown[g]) Tq
P      = PreventGearDownChangeFactor
f      = MaxRPM c + (MinRPM - MaxRPM c) P            if P >= 0
       = ((c - 1) P + c) MaxRPM                      if P < 0              # a negative factor makes a downshift more likely
n_g    = g - 1  if bUseDeprecatedAutomaticTransmission (default)  else g
V_thr  = speed(f, n_g)
request g-1 (change time GearDownChangeTime) if  V_thr > V_down
```

So with the generated points a KTM shifts up at about 94 % of `MaxRPM` of the *current* gear (kinematic rpm from the road speed,
not the measured engine rpm) and down when the speed falls below the road speed at which the next-lower gear would turn at
`ChangeDown[g] MaxRPM`. A request is accepted by `0x1450c9e10` only when no other request is pending (requested gear equals
the current one) or its duration does not exceed the running one; it sets `TotalGearChangeTime = duration`, `RequestedGear`,
`bChangedUp`, and resets `TimeSinceLastGearChange` and `TimeOverLimit`.

Hold-off factors, each tick [B: `0x1450f1c20`, the lean value is the normalised lean of the bike input handler, `0xC38`, [I]]:

```
PreventGearUpChangeFactor   += lean^PreventGearUpChangeLeanLinearity * PreventGearUpChangeLeanFactor
PreventGearDownChangeFactor  = clamp01( PreventGearDownChangeFactor + lean^PreventGearDownChangeLeanLinearity * PreventGearDownChangeLeanFactor
                                         + throttle^PreventGearDownChangeThrottleLinearity * PreventGearDownChangeThrottleFactor )
```

(the exponent of the throttle term is the field next to the factor; C++ defaults factor 0, linearity 2.) KTM: up 0.5 / 30, down
0.5 / 50, throttle factor 1 / linearity 0.5: with the throttle open the downshift is held off (`P` near 1 puts the threshold at
`MinRPM`).

**Not recovered:** the code that raises `TimeOverLimit` and `Prevent...` resets, the `NoGear*ChangeTimer` writers and the
forced downshift under braking (`ForceGearDownChangeBrake*`: tried the handler slots near `0x1450f1c20`, `0x1450d84a0` and the
bike handler offsets `0x1858` to `0x3114`; the offsets of those SDK fields belong to a class whose methods I did not match). The
neutral/stopped logic (`0x1450d84a0`: shift to 1 below `NeutralGearMaxSpeed`/`ChangeUpMinSpeed`, to neutral when stopped) is
read but not written up (manual-clutch branch and `GearWhenStopped`).

## 3. Launch clutch and the feathering controller

`ClutchFeatheringOnStart.bEnable` false [B: `0x1450d7cd0`]: `clutch = max(clutch, (1 - clamp01((rpm - Min MaxRPM)/((Max - Min) MaxRPM))) * (1 - clamp01((v_kmh - R.x)/(R.y - R.x))))` with
`Min = ClucthMinRPMPercentage` (**C++ default 0.4**, not 0), `Max = ClucthMaxRPMPercentage` (0.98 in the KTM data), `R =
ClucthReleaseVelocityRange` (0..50 km/h; the second factor is skipped when `R = (0,0)`), `v_kmh = Speed * 0.036`; `clutch = 1` is
fully pulled. If the accelerator is not pressed and the clutch is below 1 the clutch goes to 1 and the starting sequence is reset.

`bEnable` true (KTM) calls the feathering controller [B: `0x1450c0290`; the struct it reads is `ClutchFeatheringOnStart`, [I]].
Struct fields: `MaxRPMFactor a`, `PeakTorqueRPMFactor b`, `TargetRPMTolerance tol`, `TargetRPMExitTolerance xtol`, `ThrottleCap cap`,
`MinSlip ms`, `DeltaTime dt`, `RPMRaiseFactor rr`, `ClutchReleaseFactor cr`. C++ defaults: `a 0, b 1, tol 0.6, xtol 0.8, cap 1,
ms 0.1, dt 0.01, rr 0.05, cr 1`; KTM data: `a 0.7, b 0.85, dt 0.05`, so `tol 0.6, xtol 0.8, cap 1, ms 0.1, rr 0.05, cr 1`.

```
R        = MaxRPM a + MaxTorqueRPM b                       # normalising rpm (KTM: 11000*0.7 + 7159*0.85 = 13785)
slip_n   = (EngineRPM - ClutchRPM) / R ;   rpm_n = EngineRPM / R
active  if  IsAccelerating and clutch < 1  and ( slip_n > ms  or  rpm_n <= tol  or  Timer > 0 ) ; otherwise Timer = 0, no action
rpm_w    = SpeedRear / r_eff_rear * GearRatio * 9.549296      # wheel-side rpm (engine rpm if the clutch were locked); ClutchRPM if the rear wheel is invalid
rpm_ref  = max(MinRPM, rpm_w)
if Timer == 0: Timer = tiny positive (feathering starts)
elif Timer > 0.3 s and slip_n < 0.01 and rpm_ref / R >= xtol:  stop (Timer = 0, no change)           # clutch fully engaged
else:
    c_req   = Engage( (R - EngineRPM) rr , dt )                       # section 4
    lim     = clamp01( 1 - rpm_ref cr / R )                           # release as the wheel catches up
    clutch  = max( clutch , clamp01( lim c_req ) )
    throttle = min( throttle , cap )
```

The controller raises (never lowers) the clutch input; the engine is held near `R` while the wheel catches up, and it ends once
the slip is below 1 % of `R` with the wheel-side rpm at least `xtol R`. `Timer` (the "feathering timer") is only reset or set to a
tiny value here; it is advanced elsewhere (not found), so the 0.3 s condition is read as "after 0.3 s of feathering".

## 4. Required clutch input [B: `0x1451502d0`]

With `T_max = ClutchMaxTorque(dt)` (the helper of the drivetrain spec), the engine inertia `I = MOI * 10000`, the engine torque
`T_e` (`EngineTorque`), all in Unreal torque units (1 N m = 10000), the engine speed `w_e` (rad/s) and `w_w` the wheel side speed
referred to the engine (`WheelsMomentum * GearRatio / WheelsInertia`):

```
w_e'  = w_e + dt T_e / I
x     = max( 0 , sgn(w_e' - w_w) * ( dt T_e - I * dw_target ) / ( T_max dt ) )            # dw_target = (R - EngineRPM) rr * 2 pi / 60
clutch input = 1 - x ^ (1 / ClutchLinearity)
```

(`0` if `|T_max dt|` is below 1e-8; the unit factor 10000 is the float at `0x148f94fe0`.) Read: the clutch torque needed so that the
engine gains `dw_target` over `dt`, as a share of the maximum clutch torque, converted to an input through the linearity. [B]

## 5. Constructor defaults of `UInputHandler` [B: `0x1450bd6d0`]

`SteerK 400`, `SteerDamp 70`, `GearUpChangeTime 0.1`, `GearDownChangeTime 0.3`, `ClutchTimingOnGearChange (0,0)`,
`ThrottleTimingOnGearChange (0,0)`, `ReleaseThrottleOnShift false`, `ModulateThrottleOnUpShift true`, `GearWhenStopped 0`,
`bAutomaticGearChangePoints false`, `ChangeUp/DownSpeedCorrectionFactor 1`, `GearChangeOverLimitEnabled true`,
`OverLimitMinChangeSpeed 10`, `OverLimitChangeTime 1.0`, `OverLimitChangeTimeThreshold 0.5`, `OverLimitRpmPercentage 0.99`,
`AutomaticGearOverrideTime 0.5`, `NeutralGearMaxSpeed 50`, `ChangeUpMinSpeed 10`, `AutomaticTransmission true`,
**`bUseDeprecatedAutomaticTransmission true`**, `AutomaticTransmissionSpeedFactor 0.5`, `PreventGearDownChangeThrottleFactor 0`,
`...Linearity 2`, `ClucthMinRPMPercentage 0.4`, `ClucthMaxRPMPercentage 0.7`, `MaxThrottleInReverse 1`, feathering structs as in
section 3. The KTM handler overrides most of these (see the elements format page).
