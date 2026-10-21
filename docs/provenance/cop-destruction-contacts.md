# Cop destruction collision evidence

- **2026-10-21 health-to-lamp adapter:** the previously matched PC light query
  checks the destroyed predicate before its independent lamp bits. Ordinary
  damage accumulation and explicit Destroy have different visual effects:
  the former retains per-hit zone updates; the latter fills every zone. The
  host now forwards the first health-destroyed predicate to lamps without
  inventing explicit-destruction visual tiers or a destruction event at equality.

- **Spec:** [Cop destruction contact semantics](../specs/cop-destruction-contacts.md).
- **Sources:** read-only inspection of the user's hashed PC 1.3 executable on
  2026-10-20, installed vehicle/damage/event records, and the CC0
  [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `Physics/Behaviors/DamageVehicle.cpp`, `DamageVehicle.h`, `DamageCopCar.cpp`
  and `AI/Actions/AIActionTooDamaged.cpp`.
  Raw instructions, addresses, installation identity and extracted records stay
  private. No original code or binary data is included.
- **Established:** collision damage reads incoming closing velocity separately
  from body-specific resolved impulse; object and world contacts use different
  multipliers. Source zones use inward-facing corner normals. The destroyed
  predicate accepts total damage equal to one, although ordinary impact event
  emission requires crossing strictly above one. Explicit destruction also
  updates the visual zones. The inspected retirement action zeroes pursuit
  controls, adding quarter brake only outside shock and above its speed gate;
  it does not request full brake or an engine shutdown.
- **Damage-tier follow-up:** fresh PC impact instructions confirm a shared
  truncated visual severity capped separately at six impact tiers and two
  ordinary stored damage tiers. The matching zone table and dispatch loops
  establish that collision and impact stimuli precede only newly crossed damage
  stimuli, using the same callback context and allow-queue mode. Generic paint
  wear and health accumulation do not implement those stored zone transitions.
- **Notification and parts:** the inspected vehicle-destroyed handler posts the
  gameplay notification and handles race/player consequences, without directly
  dispatching a vehicle sequencer destruction stimulus. Installed cop sequences
  request named-joint fragments. The original joint factory uses authored
  female/male bounds and their nested constraint geometry; a free-body fallback
  is not evidence of the original hood/door behavior.
- **Callback magnitude follow-up:** `RigidBody.cpp` and `Collision.h` identify
  the callback impulse as the normal-force accumulator magnitude times the
  reciprocal effective moment mass, with the other body's reciprocal mass used
  for an immobile body. Previously recorded isolated PC solver executions expose
  separate normal and friction accumulators and distinguish the reported normal
  impulse from the final restitution-adjusted velocity change. This establishes
  the callback quantity without inferring it from final body velocity.
- **Native callback-frame follow-up:** the original object resolver separates
  bodies before forming local callback arms, retaining the collider's original
  point separately from the solver's moved plane point. Its closing vector comes
  from the modified contact moments. The native adapter now retains incoming
  linear velocities while computing callback arms from separated positions and
  closing magnitude from the solver's initial normal speed.
- **Captured normal-speed correction:** passive original-game captures on
  2026-10-20 establish that the damage callback's closing vector is the solver's
  normal component, while tangential sliding is retained in a separate field.
  Exact byte comparisons matched both vectors for 3,973 object callbacks in the
  larger driving capture. Of these, 1,911 had normal speed below the damage gate
  despite total normal-plus-sliding speed exceeding it. The original left shock,
  health and last-impact velocity unchanged below that gate. Bounded fresh PC
  dispatcher instruction inspection corroborates the two independent vector
  copies. This corrects the earlier total-speed interpretation; world/ground
  adapters likewise retain successful-reaction normal speed and incoming linear
  velocity. Captures and instruction bytes remain private.
- **Ground and attachment follow-up:** fresh PC ground-handler inspection
  corroborates the matching reference's retained local ground vertex and
  incoming linear velocity before the reaction and later penetration push.
  Reconstructing the local arm after that push changes the callback frame.
  Bounded PC dynamics-body immobility instructions independently confirm the
  source immobility flag or world-attachment bit. Cop eligibility now retains
  attachment after the response, while complete deferred lookup after further
  contacts remains unverified. The prop adapter also preserves the separate
  normal-accumulator shock input before light-prop and no-car-effect filters.
- **Inspection limits:** some named routines were not functions in the saved
  analysis database. Their bounded executable bytes and the matching reference
  were inspected without modifying the binary or rerunning whole-program
  analysis. Flipped behavior and causality follow the matching reference; a
  recent-player-hit timeout does not establish original causal attribution.
- **Data coverage:** inherited damage tuning was checked for resolved cop sets.
  Vehicle variants with unresolved direct links require the existing ancestry
  resolution path; a hardcoded model-name table is not equivalent evidence.
  The inspected cop event sequence does not establish a generic wreck smoke/fire
  rule. A subsequent independent audit established the `CarRenderConn` health
  effect path; see [police appearance provenance](cop-visible-damage.md).
- **Validation boundary:** deterministic contact fixtures and installed source
  checks establish the rules above. No matched live original/remake cop impact,
  flip or destroyed-car render capture was recorded in this investigation.

- **Rendering follow-up:** fresh inspection of the matching PC ordinary-car
  damage-render routine confirms overlay level above zero and window level
  above one. The matching reference `World/CarRender.cpp` routes supported cop
  models into `VehiclePartDamageBehaviour`; `World/VehiclePartDamage.cpp` maps
  source zone levels to damage slots, selects damaged part models and drives
  window texture state from window-part damage. The remake's generic aspect
  counters and per-zone glass mask were approximate presentation. The subsequent
  [police appearance implementation](cop-visible-damage.md) uses the verified
  mapping, installed successor parts and direct emitter-group health effects.
  Ordinary-car cosmetics and articulated panel animation remain incomplete.
