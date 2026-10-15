# Original camera obstruction provenance

Behavior is specified in [original-camera-obstruction](../specs/original-camera-obstruction.md).

## Sources

- User-owned original PC executable; identity was confirmed with Ghidra program
  metadata before analysis. SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Matched PC Cubic update: outside POV condition; order of feedback, noise,
  wall correction and secondary vehicle avoidance.
- PC Cubic vtable: OutsidePOV, RenderCarPOV, IsHoodCamera, look-back setter and
  getter, and changed-view selection.
- PC CameraMover wall clearance and its two-ray helper: exact FOV factors,
  splayed endpoints, normal weighting and retained half-weight history.
- PC CameraMover floor stage: normal query height, minimum clearance and
  conditional ceiling.
- PC tunnel/overpass getter and precipitation state producer, corroborated
  by the reference's AmIinATunnel function and named track-zone types. The
  state includes qualifying garages and checks anchor height, not only 2D
  polygon membership.
- PC vehicle flare rendering: RenderCarPOV and matching camera-anchor world ID
  suppress the anchor's flares without hiding other vehicles.
- The CC0 `dbalatoni13/nfsmw` reference headers and CarRenderConn /
  VehicleRenderConn source were used to identify concepts and candidate calls.
  All constants and gate tables above were rechecked against PC instructions.
  Reference code was not copied into the implementation.

## Method and limits

A follow-up query-producer check executed the unchanged native barrier routine
for 27 synthetic height, direction, slope and tilted-instance cases, plus four
instance-collector cases with explicit group activation and exclusion state.
These establish that inactive groups are rejected before barrier testing even
with a zero exclusion mask. Installed road probes compare the former unfiltered
caster with the corrected free-roam caster across sloping road-network samples;
synthetic height sweeps check that inactive barriers cannot toggle camera
obstruction and that ungrouped walls remain effective.

On 2026-10-15, unchanged original x86 instructions were executed in isolated
memory with synthetic camera state. The probes covered 14 visibility/look-back
cases, 49 view selections, 12 static clearance cases, 16 floor-height cases
and two aim-reconstruction matrices.
Twenty-one further native body-avoidance cases supplied synthetic IBody
transform, velocity and half dimensions while executing the original shape
and influence functions.
Twenty-three vehicle-underpass predicate cases supplied synthetic vehicle,
simable and rigid-body interfaces to the unchanged original routine. Fourteen
further cases executed the unchanged underpass displacement arithmetic for
inside/outside POVs and several elapsed times.
Four additional timer-boundary cases checked ticks 3838 through 3841,
including the exact suppression change between ticks 3839 and 3840.
Sixteen unchanged precipitation zone-state cases checked the tunnel, overpass,
small-overpass and garage flags, strict anchor-height comparison and nearest
cover selection. Nine further cases executed the complete native state getter,
including a missing precipitation object. Zone lookup replies were synthetic;
these cases establish flag arithmetic, not live track-region classification.
The wall oracle replaces only the external world query with explicit synthetic
hit positions and normals; original vector math and clearance recurrence run.
The floor oracle executes the original arithmetic span with a supplied sample.
The floor candidate collection was additionally traced through the native
cached-region and instance collectors. It matches the shared predictive
horizontal-region policy; the nominal sphere/cylinder selector is unused.
Fresh point-height queries reuse that candidate policy while starting with
no selected face, and impose no artificial downward ray depth.
Aim reconstruction runs the original matrix helper, with the external D3DX
matrix/vector operations supplied by their documented multiplication contract.

Static disassembly and isolated execution establish the equations and gates.
They do not establish world query coverage, asset streaming, complete native
camera output or a live visual comparison. Original executable bytes, raw
disassembly, private native probes and game assets remain outside the public
repository.

The CameraAIAvoidables registration list is populated by simulation body
attachment and by hierarchy props with CAMERA_AVOIDABLE enabled. Its producer
and remove paths were checked; generic IBody headers corroborate callback
roles. The host must preserve this eligibility rather than avoid every prop.

Native clearance halves are per update. Using a retained clock in a high-FPS
host is an explicit adaptation, not a claim that the original PC game was
render-rate independent. Live body registration, crowded-scene output and
underpass scene behavior still require validation beyond isolated math.
