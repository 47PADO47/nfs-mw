# Subtractive texture blend provenance

- The PC texture format defines `AlphaBlendType` 3 as subtractive in the
  CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reference,
  `Speed/Indep/Src/Ecstasy/Texture.hpp`.
- Static inspection of the user's retail PC executable identifies the texture
  setup branch on that field. Case 3 selects D3D9 blend factors ZERO and
  INVSRCCOLOR. The same routine configures the world effect's blend-state
  parameters. This is static evidence, not a matched in-game capture.
- The installed safe-house ground material uses blend type 3. Its texture
  and model remain in the user's installation and are not included here.
