# Per-car environment lighting

In F8, under Car materials and lighting, enable **Per-car environment lighting**.
The console accepts `set per_car_environment on` and `off`. The default is off.

Each visible car gets nine fixed upward collision rays reaching 22 metres.
Nearby ceilings block more fill than distant surfaces. Authored tunnel and garage
polygons provide a 45% occlusion floor where the roof has no collision mesh.
The lookup respects each zone’s elevation limit. Each car starts at its
sampled value. Later samples approach shelter faster than they return to daylight.
An exposed car can gain up to 47% when the player capture is sheltered.
Enhanced glossy shading applies the result to indirect light and reflections.

The CPU queries resident collision geometry on all GPUs, including RT hardware.
At 30 updates per simulated second this costs up to 270 segment queries per
visible vehicle per second. The shader reads an existing instance attribute lane.
There is no additional GPU pass. Frame time has not been measured.

Outside tagged enclosures, render-only scenery cannot block these rays. The shared reflection image retains
its original viewpoint and hue. The estimate applies to the main and split-screen
views. Independent captures do not update the estimate.

Sources are the project's collision segment queries, [track path zone format](formats/road-network.md)
and native glossy material implementation. The sampling pattern and temporal filter are original project code.
