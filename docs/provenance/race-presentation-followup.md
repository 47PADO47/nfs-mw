# Race presentation follow-up evidence

The user's modded PC recordings on 2026-11-05 show the Carrera GT as the primary
intro subject. At finish the driving view precedes the moving finish camera,
then the photo artwork and result text enter in stages. These agree with the
two 0.75-second phases described by the existing finish-effects specification.
Capture A/V discontinuities limit exact sound-onset comparisons.

The CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reference,
`Frontend/MenuScreens/InGame/PhotoFinish.cpp`, describes the phase timers,
appearance scripts and misc sound 2 at entrance. The installed
`InGamePhotoMaster.fng` primary picture/group `APPEAR` scripts hold their visible
state while child text/footers animate. Its separate 1800 ms fade ends hidden;
its event-handler entrance takes 500 ms. The host had immediately forced picture and group alpha
to 255 every frame and started only part of this sequence.

Ghidra identity was checked before queries: PC executable SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
`SFX_Common::MsgPlayMiscSound` at `0x004ebd90` selects `FX_Camera` for message 2,
initializing eight words with selector zero and the sound group's volume.
The CC0 front-end handler only implements message zero; it does not resolve
this cue. No decompiled implementation is copied into public sources.

The same photo constructor requests the `game_camera` moment stream before
the shutter. Its generated attribute-name table identifies that recording.
The host previously omitted it. Static camera research establishes a special
camera state over the first interval, not the exact post-finish vehicle controls.
The host's simulation ramp is explicitly a reconstruction policy.

The physical-grid channel mapping can assign `car1` to an AI rival when
the human starts farther back. The primary-actor swap and fallback staging of
untracked racers are host policies. Hiding all untracked rivals caused empty
intros. Neither policy changes physical grid slots.

Follow-up listening evidence rejects the selected NIS end-section recording as
the required countdown effect. The user confirmed normal menu sounds work and
that neither the current two-channel clip nor an all-channel diagnostic fold
contains the effects heard in the captured OG countdown. The existing positive
PCM test established stream delivery, not cue identity. Decoding was separately
checked against vgmstream; changing playback rate or discarding fewer channels
is not an established repair.

The subsequent identity-guarded PC runtime capture recorded the selector call
and native speech sample requests. `IntroNis06` requests event `0x4000`, scene
32, track 8192 and sections 1 then 4. The relative sample offsets uniquely match
the `0x4023` bank's takes 18 and 19. The input index, event table and stream file
identities were captured and checked. This corrects the inferred start/end bank
assignment and parameter order. The selected cue is now tied to the original
runtime request rather than inferred from the number of takes in a bank.
The user confirmed that the diagnostic clip of this selected take, using the
host's current channel output, contains the correct countdown effects. Installed
regressions check the native bank and take before testing playback output.
