# Retained garage carousel return

Source: user playtest and installed-package regression, 2026-10-27. This is a host robustness policy.
No translated executable implementation was used. See [garage UI](garage-game-ui.md) and
[provenance](../provenance/garage-game-ui.md).

A pushed child can leave its retained parent's authored chrome faded out. Logical input and the icon strip can
recover while the title, selected item text and footer stay invisible. A screen-name assertion does not detect it.

On return, restart that parent's authored INIT scripts, restore host selection and refresh contextual text/footer.
Keep its package identity and model; do not recreate the car, commit a preview, or purchase/save anything.

Validate repeated category/part/Back routes, a nondefault Performance/Transmission selection and Showcase return.
Check actual drawn text/footer and retained selection, plus input and unchanged transaction state.
Exact original return animation timing, other garage presentation and physical input remain unverified.
