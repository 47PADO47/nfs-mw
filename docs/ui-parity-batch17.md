# UI parity repair candidates: batch 17

Six bounded source repairs continue the open playtest backlog. They are not a full parity sign-off.
The earlier gauge-lettering correction remains included in the source and is not counted among these six.

1. **UI-045, repeated garage work:** retain shadow and planar inputs while geometry and the effective camera
   stay unchanged. Visibility, uploaded resources and stage changes invalidate the cache. The original fixture
   lacked submission tracking; the corrected before-test recorded seven submissions instead of two. This proves
   redundant work, not the cause or removal of every visible shadow flicker.
2. **UI-049, career car switches:** retain the room and old car during a same-stage replacement. Publish the new
   car only after upload and explicitly release old car resources. Different stages still follow normal scene changes.
3. **UI-050, tire contact:** remove the extra stage sink. The assembled model already includes a 0.025 m sink;
   applying it again produced 0.05 m. Rendered wheel contact remains an acceptance check.
4. **UI-035, exit input and sound:** expose the pending-exit input gate through the Safe House wrapper and the
   EA Trax/tuning screen owners. The screen host can then reject pointer/pad input and automatic feedback during exit.
5. **UI-041, dim stat fills:** keep lit customization/My Cars strips in front of the dim base. Preserve native
   artwork colors rather than inventing a replacement palette. The recorded reference shows a bright lit extent.
6. **UI-015/051, camera requests during intro:** a later screen request updates the deferred target without
   interrupting the authored intro path. After the intro, the latest screen receives its normal camera transition.
   This fixes interrupted motion; it does not establish the separate environment/wall-clipping report is closed.

## Validation and acceptance

Before-tests reproduced the retarget, duplicate sink, redundant submission, wrapper gate, stat layer and interrupted
intro defects. Focused final results are retained in the local validation record. Checks use isolated settings/saves.

The wider EA Trax check still expected the superseded Re-order/MenuButton1 footer. Its fixture now checks
Grab and the dedicated JukeboxGrab binding. This test maintenance is not counted as another product fix.

The historical empty-profile Alias Load, off-window pointer fixture and Challenge Series assertion failures remain
separate validation findings. They are not counted as user-visible fixes. The full workspace/GPU workflow is not
claimed by these focused checks. Physical controller, audible feedback, rendered comparisons and frame pacing still
need playtest acceptance. No playable build, commits or publication are part of this source batch.
