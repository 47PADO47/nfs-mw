# Original UI continuation handoff

This contribution preserves the accumulated UI repair candidates, regression coverage and capture tools.
It remains a partial implementation. All 44 reports in the [playtest backlog](ui-parity-playtest.md) remain
open for acceptance. The latest eight screenshots are retained privately; no game images are included here.

## Included work

- Retained menu returns, empty-name rejection, high-frame-rate held navigation, binding-aware prompts and slider
  texture repetition; transition input gates also cover pointer and supplemental garage actions.
- Controller remapping for buttons, triggers and signed axes, with neutral-release capture and saved bindings.
- EA Trax scrolling, mode editing, preview/reorder actions and temporary playback restoration.
- Garage presentation/camera/return candidates, bounded recent-car preview reuse, tuning Help/exit behavior,
  native pause background, career grids/Bio and the supported free-roam Safe House action.
- Scene effect and police-speech cleanup, opt-in remake traces, and continuous Windows reference capture/review
  tools. Original probes, media, profiles, builds and installation data remain outside this contribution.

The [batch 12 record](ui-parity-batch12.md) distinguishes each candidate from its remaining acceptance checks.
The PR preparation applies these changes onto current upstream and fixes the six locations that produced the
eleven new lint reports in the earlier run. It does not implement the later playtest requests below.

## Next repair tasks

1. **Navigation and career state:** reproduce Vinyls Back jumping to the main menu (UI-040), then the distinct
   career My Cars route (UI-043). Check preview/selection, names/manufacturer, valid options, prompt icons and return.
2. **Loading:** replace full-screen My Cars waiting with a preview-scoped pending state where needed (UI-018),
   and measure first unseen-car and repeated-car stalls independently (UI-017). The three-preview cache does not
   satisfy the user's smooth-switching report.
3. **Garage presentation:** repair the spoiler carousel (UI-036), cart checkmark alignment (UI-038), one Custom
   paint page (UI-039), stat colors (UI-041) and display names (UI-042). Keep each task bounded and capture before/after.
4. **Scrollbars and career artwork:** audit overflowing option categories, including Graphics (UI-037); establish
   the correct bounty-grid icons (UI-030) and Blacklist Race Events category icon (UI-044). The latter is distinct
   from per-event race-list artwork (UI-031).
5. **Other acceptance gaps:** retain the earlier backlog, missing [options inventory](ui-options-coverage.md),
   EA Trax fresh/reset defaults and mode order, gauge speed/gear targeting, camera/vignette matching, and actual
   controller/audio/animation checks. A narrow passing regression does not close a user report.

## Validation history and blockers

Before current-upstream integration, the complete native-only run passed 10,317 workspace tests, Python tests,
licenses, guards, formatting and seven headless Vulkan/DX12 checks. Focused new/previous UI checks passed against
both modded and original-style installations; five controlled restored defects failed their corresponding tests.
These are historical results, not fresh results for this PR revision. The PR description records current outcomes.

The earlier complete run still failed overall: 34 Clippy reports included eleven new reports at six locations,
and the installed frontend stage had 163 passes and seven failures. Preserve these failure names for comparison:

- `frontend::bust_resume_tests::installed_bust_ticket_and_payment_return_to_driving_through_the_front_end`
- `frontend::bust_resume_tests::installed_career_bust_cash_payment_leads_to_the_car_select_the_safe_house_and_driving`
- `frontend::bust_resume_tests::installed_career_bust_third_strike_impounds_the_car_and_the_player_must_choose_another`
- `frontend::pointer::tests::game_pointer_is_a_16_pixel_arrow_with_its_tip_inside`
- `frontend::tests::click_tests::icon_click_tests::a_native_carousel_can_reopen_a_question_after_no_or_back`
- `frontend::tests::click_tests::icon_click_tests::clicking_a_windowed_category_opens_its_bound_action`
- `frontend::tests::icon_transition_tests::every_icon_of_the_main_menu_can_be_selected_but_only_the_working_ones_do_anything`

The cash-payment and question-reopen names were additional failures versus the previous recorded run; their
cause/onset was not established. Do not label all seven as pre-existing upstream failures. Modded cursor artwork
can differ from stock; both installations need explicit identities. No full physical-controller, audible or
matched rendered-reference sign-off has been obtained.

For every continuation task, retain a failing reproduction, implement one issue or proven shared cause, check
the actual menu route and repeated return, then retain unresolved acceptance explicitly. See the backlog's
[closure rules](ui-parity-playtest.md#evidence-and-closure-rules).
