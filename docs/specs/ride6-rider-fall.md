# RIDE 6, the rider in a fall: leaving the bike, the ragdoll, the way back (clean-room spec)

What the shipping executable (build `5.6.1-1697004`) and its data do when the fall tests of [the chassis](ride6-chassis-round6.md)
fire, and what this project does with it. Addresses are in [the provenance page](../provenance/ride6-physics-D.md). Tags: **[B]**
read from the binary, **[D]** from the data assets, **[S]** from the class dump (field names and offsets only), **[I]** inferred,
**[M]** a choice of this project's model. Units as in the data: km/h and degrees where the data have them, metres, kilograms and
seconds in the formulas.

## 1. What RIDE 6 does

**The chain.** The handler's fall tests (section 3 of the chassis page) end in `StartFall` [B]: it sets the handler's fallen flag, records the
vehicle's transform of that moment, queues one call on the physics scheduler and fires the handler's `BikeInputFallenEvent`. The bike component turns that into its `BikeFallenEvent(Sender, FallType)`; the fall type is a name,
`HighSide` or `LowSide` (any other name is "unspecified"). The kind of the test (the codes 1, 2, 3 of the chassis page, [S]
`EFallType`: generic, low side, high side) is what names it **[I]**: the broadcast name was not traced from the code.

**The rider component** (`UIgnitionRiderComponent`, the one on every rider) is the listener [B]. Its `OnBikeFallen` does three things:
it marks a ragdoll **pending**, stores the fall type (`HighSide` 2, `LowSide` 3, otherwise 1), and returns. The pending fall is taken
at the next tick of the component, which

1. reads the bike's state from a ring buffer the component keeps of recent frames, the **oldest** entry of a ring of
   `RagdollVelocityFramesDistance` entries (2 by default [B, constructor]); the point is to use the speed *before* the impact or
   the slide that the fall tests noticed, not the already ruined speed of the fall;
2. works out the rider's launch velocity from it: the bike's velocity, turned about the vertical by a random angle between the
   two numbers of `RagdollVelocityRandomRotationRange` (degrees; the sign follows the bike's lean when it is beyond
   `RagdollVelocityRandomRotationSignLeanLimit` of its range), plus an offset taken from the damper compression
   (`FrontDamperCompressionVelocityOffset` or the rear one, whichever damper is compressed more, if that is beyond
   `MinDamperCompression`); the result is multiplied by `RagdollVelocityMultiplier`. The constructor's defaults are **a range of
   (0, 0), a sign limit of 0.5, a multiplier of 1.0, a minimum compression of 0 and both offsets zero** [B], and the rider blueprint
   overrides none of them [D]: **in the game as shipped the rider leaves at exactly the velocity the bike had two frames before**;
3. starts the ragdoll: `IgnitionRagdollComponent::EnableSimulation(velocity, angular velocity, fall type)`, with the bike's
   angular velocity from the same buffered state.

**The ragdoll component** [B, D]. `EnableSimulation` makes the rider's skeletal mesh simulate physics, **detaches the root** (the
rider no longer follows the bike: `bDetachRoot`, `bSimulateRoot` are set, the root body is the pelvis [D]) and gives every body the
launch velocity. The velocity is in km/h and becomes cm/s by 100/3.6. For a fall type with a **fall configuration** [D]
(`StartFallConfigurations`; a type with none, the generic fall, gets the plain velocity and no spin) the velocity is

```
v' = v + R a + w + R ( m (.) (R^-1 v) )         a  FallAdditionalVelocity            (km/h, in the rider mesh's frame, x forward, z up)
spin' = f spin                                  m  FallVelocityMultiplierAdditional  (a factor per axis of that frame)
                                                w  FallAdditionalVelocityWorld       (km/h, in the world)
                                                f  FallAngularVelocityFactor
```

with `R` the mesh's orientation. The two configurations of the rider blueprint:

| | `a` | `m` | `w` | `f` |
|---|---|---|---|---|
| high side | (0, 0, +6) | (0.05, 0, -0.28) | 0 | 0.5 |
| low side | 0 | (0.05, 0, +0.32) | 0 | 0.5 |

(The vertical factor acts on the vertical part of the velocity, which is nearly nothing on a road; so a high side throws the rider 6 km/h
(1.7 m/s) upward, and both add 5 percent to the forward speed.)

**The rider does not collide with the bike** [D]: the mesh's collision responses ignore vehicles, pawns and physics bodies and block
only the world. The rider's physical material is `PM_RiderRagdoll_CB`: friction and static friction **0.62**, restitution **0**
(minimum combine for the restitution), sleep thresholds 5 cm/s linear and 1 rad/s angular; the road's is the material default 0.7
(`PM_T_Tarmac` sets no friction), and the two average (the default combine mode).

**The physics asset** `RiderGP_PhysicsAsset` [D]: 11 bodies, each one capsule on a bone, with a mass override (kg):
pelvis 7.0, `spine_02` 20.4 (the shoulders: radius 19.2 cm), head 4.3, upper arms 4.2, lower arms 2.3, thighs 7.0, calves 4.0, 66.7 kg
in all; and 10 joints, each with a constraint frame on the child bone (default, the bone's own) and one on the parent bone (a
primary and a secondary axis), a cone limit (swing 1 about the frame's Z, swing 2 about its Y), a twist limit (about X) and an
angular drive:

| joint (parent > child) | swing 1 | swing 2 | twist |
|---|---|---|---|
| pelvis > spine_02 | 20 | 15 | 25 |
| spine_02 > head | 15 | 25 | 25 |
| spine_02 > upper arm (each) | 40 | 30 | 20 |
| upper arm > lower arm (each) | 30 | locked | locked |
| pelvis > thigh (each) | 50 | 5 | 5 |
| thigh > calf (each) | 50 | locked | locked |

(degrees). Linear damping is the default 0.01 (the pelvis 0.02), angular damping 0.

**What the game adds on top, not modelled here** [D, B]: the ragdoll is not a bare simulation. The body drives itself towards a pose
(`RagdollStrengthCurve` is the constant 1,000,000 [D]; the component keeps a "kinetic" and an "animation" strength and blends
them), and the rider's animation blueprint plays a **fall animation** chosen by how he lies (flags `Ragdoll_Fall_LeftRight`,
`Ragdoll_UpRollDown`, `Ragdoll_isFlying`, `Ragdoll_isLowSpeed`, `Ragdoll_Airbag_Alpha`, `IsFallen`; an airborne distance over 1,
ramped to full at 2.5, decides "flying", a speed under 0.5 "low speed", in the blueprint's units) out of 25 clips: low-side and high-side falls (face down, face up, front, side),
sliding, rolling, and the get-up clips (side left/right, back, face down). When the fall is over a **raise-bike** component runs a
sequence [S] (`ERaiseBikePhase`: riding, fallen, before get-up, get-up, walk) in which the rider gets up, walks to the bike and
lifts it, with durations and collision channels in its data; the respawn logic of the bike is also in the blueprint. Which of the
vehicle's own mass and inertia change when the rider leaves was not read from the binary.

## 2. What this project does

**The bike** (`libs/blackbox-vehicle/src/bike/release.rs`, `fall.rs`). The fall tests run in the order of the chassis page and the
first one that fires gives the kind: the lean on the ground and the angular speed a **low side**, the velocity against the heading a
**high side** when the bike's lean is under `fall.highside_lean_rad` (`FallHighsideLeanLimit`, 70 degrees for the KTM [D]; 0, the
default, never) and a low side otherwise, the lean in the air and the pitch a **generic** fall. At that step the rider is let go:

- the bike's mass becomes its own (`chassis.mass`, the inertia scaled with it), its centre of gravity the bike's alone, and the
  velocity of that point is what the same point of the frame had (`v += w x (R (cog_new - cog_old))`), so nothing jumps; his spring
  and his turning stop acting, his air drag stops, he holds no lever and no bar (the handlebar swings back to the middle) [M];
- a `RiderRelease { kind, position, velocity, angular velocity, orientation }` is made once: `position` is the rider's centre of
  mass in the world, `velocity` and `angular velocity` the bike's from **two steps before** (the ring of section 1) with the
  velocity moved to his place (`v + w x (p - cog)`).

Placing the bike again (`place_on_ground`, the stand-up) puts his mass and his place back.

**The ragdoll** (`libs/blackbox-vehicle/src/ragdoll/`, generic): capsule segments, swing-and-twist joints, position-based dynamics in
8 sub-steps of a 60 Hz step (section 3 below for the numbers). It is the rider as a free body, and it inherits the velocity at each
body's place from the release (`v + spin x (x - centre)`), falls, slides and rolls on the ground with Coulomb friction, restitution 0.

**The game** (`crates/nfsmw/src/scenes/world/drive/rider/fall*.rs`). The mod's converter writes `RIDER/ragdoll.json` from the
physics asset and the blueprint (`ride6-rider-ragdoll`): bodies with a mass, a radius, half the capsule's length, its centre and axis
in the bone's frame; joints with the two constraint frames and the three limits in degrees (locked 0, free 180); the friction
(`(0.62 + 0.7) / 2 = 0.66`); the two fall configurations in m/s; the velocity multiplier. At the release the game

1. builds the ragdoll in the pose the rider had on the bike (the bones' places from the riding animation, the bike's pose), the
   shared point of a joint being the child bone's origin, the limits in each joint's constraint frames;
2. gives it the launch velocity of section 1 (the bike's, times the multiplier, changed by the configuration of the kind) and the
   spin times `f`;
3. steps it every physics step over the world's ground, and each frame poses the skeleton: a bone with a body goes where its body is,
   every other bone follows its parent as it was in the riding pose (a hand follows its lower arm, the neck the shoulders) [M: the
   reference blends a pose towards its animation instead];
4. when the bike is stood up (`rider_off` false) blends him back onto the seat over 0.25 s and the rider is attached again.

A rider without `ragdoll.json` keeps to his bike as before. A car never sees any of this.

**The bike slides on its side** without the rider on its collision box: the double meets the ground (`Vehicle::finish_body_step`) and
only the ground's friction slows it (the first version took 8 percent of its speed per step, which stopped it in about a second [M]).
A bike thrown down at 20 m/s slides several metres on its side (test: more than 8 m in the first second).

## 3. Choices, and what is not modelled

| Choice | Value | Why |
|---|---|---|
| Joint limits eased in | a joint beyond its limit when the ragdoll starts (the knee is bent 90 degrees on a bike, its limit is 50) has the excess closed with a time constant of 0.3 s | the asset's limits are relative to the reference pose; a seated rider starts outside them and must not snap |
| Joint damping | 4 /s on the angular velocity of a child against its parent | the asset's drives (position and velocity springs of the physical animation) are not modelled; a limp limb must not swing for ever |
| Solver | 8 sub-steps, one joint pass before and one after the contacts, end spheres of the capsules as the contact shapes | stable for 11 thin bodies at 60 Hz |
| Capsule inertia | a solid cylinder of the capsule's length | the engine uses the shape's own; the difference is small for these bodies |
| Friction | 0.66 on a road, times the ground's multiplier | the average of the rider's 0.62 and the road's 0.7, as the engine's default combine mode; other surfaces scale it |
| Velocity moment | two steps back | the ring of the reference, of 2 entries |
| Rider's mass leaves the bike | at the release | not read from the binary; the rider is a separate simulated body there |
| Bike stood up | after 2 s down, once slower than 1.5 m/s | the reference's get-up, walk and raise take longer and are not modelled |
| Blend back | 0.25 s, smoothstep, in the world | no counterpart; the reference walks the rider to the bike |
| Pose of the bones with no body | their local transforms from the riding pose, after their parent | physical animation is not modelled |

Not modelled: the physical-animation drives and the fall animations; the random rotation and the damper offsets of the launch
velocity (zero in the data); the airbag; the get-up, the walk and the raising of the bike; collision of the rider with the bike, with
props and with walls (the ragdoll meets the road only); the `FallHighsideEffect` and `FallSteerComponent` of the handler; the
upside-down, rider-force and collision tests of the chassis page.

## 4. Verified by tests

`libs/blackbox-vehicle`: the release is made once at the fall with the kind, the mass, the centre of gravity and the velocity of the
bike's own point unchanged through it, the rider's velocity is the bike's two steps before, the bike drives and steers no more,
placing it again restores the rider, high side and low side by the test that fired; the ragdoll inherits a velocity, slides to rest
by Coulomb friction (12.1 m for 12 m/s and 0.6, the exact 12.2), keeps its joints together while tumbling, holds a swing limit,
eases a joint that starts beyond its limit, damps. `crates/nfsmw`: the file parses and refuses nonsense; the launch velocity by kind
and in a turned bike's frame; the ragdoll starts at the seat, falls and slides, what hangs on a body follows it, the blend back ends
in the riding pose; `CarSim`: a bike that goes over hands its rider over once, slides more than 8 m, is stood up with its rider;
a car never has a rider to lose. `mods/ride6-bikes`: the converter on a small asset and on the real one.
