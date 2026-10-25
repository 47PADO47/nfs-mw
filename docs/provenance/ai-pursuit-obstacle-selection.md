# Pursuit obstacle selection and roadblock road walks

Date: 2026-10-25.

Specification: [obstacle selection](../specs/ai-pursuit-obstacle-selection.md).

- Supported original PC program identity was confirmed before inspecting its
  roadblock builder and road navigator's avoidable fetch routine. The builder
  confirms zero heading weight, cop path type, a zero advance direction hint,
  and decision-segment rejection at the destination. The avoidable fetch reads
  the vehicle's neighbour list and stops at the supplied capacity.
- The CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reconstruction
  provides corresponding `AICopManager`, `WRoadNav` and `AIVehicle` context.
  Its vehicle neighbourhood sweep supplies the radius/speed rule. Nearest-body
  retention is explicitly our adapter, not a claim about original list ordering.
- Original ground support selection skips heavy options when a roadblock is
  attached. The inspected PC selection routine also writes a 15-second roadblock
  timer whenever a heavy option is selected, including ram support. The remake
  previously forwarded that reset only for heavy roadblocks. Do not increase
  support chances to conceal an unresolved lifecycle,
  asset-loading or placement failure.

Implementation follows the behavioral specification. Native code, executable
addresses, installed assets and private captures stay outside public changes.
