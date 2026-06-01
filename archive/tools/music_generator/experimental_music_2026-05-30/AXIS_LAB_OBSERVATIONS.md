# Timbre Axis Lab Observations

These notes record listening feedback from the Axis Lab. They should guide the
control model before any axis is promoted into a named timbre.

## 2026-05-23

### Harmonicity

- Harmonicity behaves as expected.
- Odd partials read more electronic.
- Inharmonic partials clearly read as dissonant.

Consequence: harmonicity is a valid axis, but inharmonicity is high-risk in
melodic roles and should usually be restrained or backgrounded.

### Noise Role

- Attack noise reads like a keyboard-like attack.
- Sustained noise reads flute-like.
- Noise carrier is too quiet and reads as thin/scratchy.

Consequence: noise role is a valid axis. Noise carrier needs independent
loudness calibration before it can be judged fairly.

### Attack Identity

- Soft air reads closer to pipe-organ-like than breath-like.
- Pluck impulse reads like striking a resonant container.
- Strike is small and high, like a metal stick hitting a plastic plate.
- Gated oscillator reads synthetic and hard to describe.
- Later clarification: the resonant/watery/container material quality of Pluck
  impulse is not automatically a defect. Attack identity may actually be
  exposing material identity, which is a useful axis.

Consequence: do not "correct" material quality away just because it is not the
expected label. Record the material impression first. If attack and material
cannot be isolated, treat this as an attack/material axis rather than forcing a
pure attack axis.

### Brightness Decay

- Amplitude decay only is springy, somewhere between string and piano.
- Filter closes is nearly the same as amplitude-only.
- Upper partials decay faster reads more like a strongly tensioned string.
- After making the lowpass sweep stronger, Filter closes was still not
  distinguishable.

Consequence: filter cutoff decay is not currently a reliable audible axis in
this setup. Separate partial decay is a clearer operation than a simple lowpass
sweep. Before promoting filter-envelope control into timbres, the lab needs a
stronger or longer isolated example.

### Resonance / Body

- Dry tone reads slightly piano-like and slightly flute-like.
- Body bump is far too loud and startling.
- Comb body sounds essentially the same as body bump.
- After reducing wet gain, Body bump and Comb body still sounded unchanged.
- Implementation check found that the body branch was not actually routed to
  the audible envelope/output; the direct path was being heard instead.

Consequence: body/resonance was not a fair listening test. The routing must be
fixed before evaluating the axis. Body effects should remain controlled, but not
silenced, because the point is to hear what material/body does.

### Pitch Stability

- Stable, vibrato, and jitter were not detectably different.
- After strengthening the effect, vibrato is only slightly felt.
- Pitch jitter reads like a ghost voice.

Consequence: pitch motion is audible, but subtle. Jitter creates a strong
material/character association rather than merely "instability", so it should be
used deliberately and probably not as a neutral melody default.

### Foreground Distance

- Close reads confident and present, like a strongly played keyboard.
- Background has its own world/character rather than merely being farther away.

Consequence: foreground distance is not only distance. It also changes world
and material impression, so it should not be treated as a neutral gain axis.
