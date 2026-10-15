# Player options provenance

- The user's original-game screenshot identifies Player as the screen containing
  Transmission, Camera and Gauges in that order.
- The local behavioral reference `UIOptionsMain` includes Player in the category
  strip; `OMPlayer` chooses the Player category. `UIOptionsScreen` selects its
  main/pause header and adds Transmission, Camera, then Gauges. Its widget
  declarations identify the Camera language label. These references establish
  behavior only; no reference implementation is copied.
- The remake's read-only `strings` command resolved both header labels and the
  Camera label from the installed English language table. `dump-screen` confirmed
  the shared Options package and its reusable row/header objects. Installed
  labels, art and package data remain runtime inputs from the user's install.
- Independently written Rust enables the existing Player icon, routes it through
  the existing category logic and moves already implemented controls. Unsupported
  original rows and multiplayer selector behavior are not added as placeholders.
