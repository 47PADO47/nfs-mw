# Pass A+B playable build: playtest follow-up

Reported: 2026-11-01, after the Pass A+B update to the existing UI parity test build.
Scope: Original UI. Both reports are open and need reproduction. Earlier focused tests and
startup/return checks do not override these findings. No fixes are claimed in this record.

- [ ] **UI-060 - Performance Tuning hangs after visiting other menus.** The user visited
  unspecified menus, then attempted Performance Tuning and became stuck. S45 shows the Pause
  Menu header, Performance Tuning caption and navigation arrows, with an empty category-icon
  area instead of the tuning controls. The exact preceding route, input device, recovery behavior
  and whether only the UI or the whole application hangs are not yet established. Reproduce from
  gameplay through prior menu visits, capture the transition/stack and logs, and check repeated
  entry plus Back. Treat this as a new playtest regression; do not infer a deadlock from the still.
- [ ] **UI-061 - Controller vertical inspection controls are reversed.** In car viewer/inspection,
  controller Up and Down move the camera in the opposite expected direction. The precise viewer
  routes and active inversion setting are not yet recorded. Compare default and saved inversion
  settings, stick and D-pad inputs, and standalone/career inspection. Check keyboard and mouse
  separately before changing a shared sign or applying inversion twice.

## Evidence

S45 is the supplied screenshot of the stuck Performance Tuning selection. It is preserved in the
private evidence archive with the tested build identity. No recording or screenshot accompanies
UI-061; it remains a valid user-reported input failure.

Follow-up ownership: [repair plan](ui-parity-repair-plan.md), P03/B1 for tuning entry/return
reproduction and P11/B2 for inspection input direction. Root causes remain unknown.

Repair candidates for both reports are in [batch R1](ui-parity-r1.md). They remain open for acceptance.
