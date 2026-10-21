# Particle control and count interpretation

Spec: `docs/specs/particle-emission-control.md`. On 2026-10-20, read-only
inspection of the matching PC 1.3 executable identified the emitter controller,
group update and spawn-count routines. Reference emitter sources supplied field
and state names. Isolated x86 instruction execution used synthetic object and
attribute state to verify zero/positive cycles, delay rollover, one-shot state
changes and life-scaled counts with variance. The original file was only read.

The public implementation follows the plain-language contract; no executable
bytes, extracted original tables or decompiled code are included. Synthetic tests
exercise the verified boundary inputs and outputs. Zero-cycle forced sources
remain on, rather than being converted into a guessed finite burst. The reachable
ordinary prop effects inspected use positive cycles for detached bursts and
zero cycles for keyed trails; a test-only sequence also uses zero-cycle bursts.

This establishes control and count rules, not pixel parity, the complete original
global random stream or live original-game rendering. Global scheduling/resource
competition and matched live captures remain separate validation work.
