# Melody Timbre Knowledge Plan

The goal is not to copy external samples. The goal is to give the generator a
credible knowledge base for constructing diverse, familiar, musically pleasant
timbres with the Web Audio building blocks it already has: oscillators, noise,
filters, envelopes, gain, delay-like buffers, and modulation.

Random timbre selection is valuable only if it can be explained as random choice
inside a reasonable classification. A user should be able to trust that the
generator is choosing from meaningful, independent categories rather than many
near-duplicates named after instruments.

## Problem Statement

The current lab can produce sound, but many melody timbres share the same small
set of synth behaviors:

- oscillator lead with envelope
- plucked buffer with similar decay
- bright bell/pluck variant
- saw/noise color

That is not enough. The generator needs a text-backed model of how familiar
instrument families differ in acoustic construction, so local synthesis can be
guided by real timbre patterns rather than by names.

## Priority

First priority is external textual knowledge, not external audio assets.

Use sources that explain:

- spectral structure: harmonic vs inharmonic, odd/even harmonic emphasis,
  brightness, formants, noise components
- excitation: pluck, bow, breath, reed, strike, electronic oscillator
- resonator/body: tube, string/body, bar, membrane, filter/formant
- time behavior: attack, decay, sustain, release, brightness over time
- performance behavior: vibrato, pitch instability, breath/noise, velocity
  response, note transitions

Only after those patterns are represented should we decide whether samples,
physical modeling, or local synthesis is the right implementation.

## Classification Axes

Melody timbres should be selected using independent axes, not a flat list.

### 1. Excitation

- breath jet
- reed
- pluck
- strike
- oscillator
- noise burst

This axis determines the attack and whether noise is part of the identity.

### 2. Resonator

- open/closed air column
- string plus body
- bar/idiophone
- electronic filter
- no stable acoustic resonator

This axis determines harmonic emphasis and which filters/modulations are
reasonable.

### 3. Envelope

- sustained
- decaying
- short struck
- gated electronic
- swelling/slow attack

This axis prevents every sound from becoming a short synth blip or a continuous
lead.

### 4. Spectrum

- sine-like/fundamental-heavy
- odd-harmonic
- full harmonic/saw-like
- noisy/breathy
- inharmonic/metallic
- hollow/body-heavy

This is the main axis for audible diversity.

### 5. Familiarity

- familiar acoustic
- stylized acoustic
- explicitly synthetic
- abstract texture

This is important for user trust: if a sound is synthetic, it should be named
and selected as synthetic, not disguised as a real instrument.

### 6. Foreground Distance

- close lead
- middle melodic line
- background color

Some timbres are intrinsically close even at the same gain. This axis should
control register, gain, brightness, and density.

## Source Types Needed

### Acoustics References

These explain why instrument families sound different.

- UNSW Music Acoustics sound spectrum material explains harmonic spectra and
  how spectra relate to timbre.
  - https://phys.unsw.edu.au/jw/sound.spectrum.html
- UNSW flute acoustics explains flute harmonic behavior, breath/air-column
  behavior, and how loudness changes higher harmonic content.
  - https://phys.unsw.edu.au/jw/fluteacoustics.html
- UNSW flute vs clarinet/open vs closed pipes explains why flute-like and
  clarinet-like sounds should not be treated as the same oscillator patch.
  - https://www.animations2.physics.unsw.edu.au/jw/flutes.v.clarinets.html
- UNSW guitar acoustics should be used to understand string/body behavior
  before claiming guitar-like synthesis.
  - https://newt.phys.unsw.edu.au/music/guitar/

### Synthesis Construction References

These explain how to map acoustic patterns onto synth primitives.

- Sound On Sound Synth Secrets is a practical synthesis knowledge base. It is
  useful because it explicitly connects oscillators, filters, envelopes, and
  modulation to instrument-like sounds.
  - https://www.soundonsound.com/series/synth-secrets-sound-sound
- Envelope references are needed because timbre is time behavior, not only a
  waveform. Envelopes should control both loudness and brightness.
  - https://www.perfectcircuit.com/signal/learning-synthesis-envelopes-1
- Digital waveguide / Karplus-Strong references are needed for plucked-string
  behavior. A simple pluck buffer is not enough to justify guitar naming.
  - https://ccrma.stanford.edu/~jos/pasp/
  - https://crypto.stanford.edu/~blynn/sound/karplusstrong.html

### Implementation References

These constrain what the current browser implementation can actually do.

- Web Audio API references define the available primitives: OscillatorNode,
  BiquadFilterNode, GainNode, AudioBufferSourceNode, AudioWorklet if needed.
  - https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API

## Knowledge-To-Implementation Mapping

The generator should not start from an instrument name. It should start from a
model:

```txt
excitation + resonator + envelope + spectrum + modulation + distance
```

## Installed Knowledge Rules

These are the local rules that must drive playback timbre design. They are not
instrument-name claims; they are minimal audible obligations derived from the
references above.

### Timbre Category Legitimacy

A timbre category is not valid because it has a different name. It is valid only
when it changes at least one audible construction mechanism:

- excitation source
- resonator model
- harmonic vs inharmonic partial structure
- noise role
- amplitude envelope
- brightness envelope
- pitch instability or vibrato
- foreground distance / intrinsic gain

If two entries share the same mechanism, envelope, spectrum, and role distance,
they are parameter variants, not separate categories.

### Source-Family Obligations

| Family | Source rationale | Required local synthesis behavior |
| --- | --- | --- |
| breath column | Flute-like sources are low-harmonic air-column tones, with breath noise and brightness changing with playing intensity. | Sine/low partials, soft attack, breath noise, subtle vibrato, limited upper harmonics. |
| reed column | Clarinet-like closed cylindrical pipes emphasize odd harmonics compared with flute-like open pipes. | Odd-harmonic additive source, faster attack than breath, formant/lowpass filtering, slight noise/instability. |
| plucked body | Plucked-string models need an excitation plus a resonating, damped string/body response; a generic short oscillator envelope is not enough. | Noise/impulse excitation, body resonance, damping, decaying brightness, optional string/finger noise. |
| struck bar | Bar/idiophone sounds are not merely plucks; inharmonic partials and unequal partial decays are central. | Fast strike, inharmonic partial ratios, separate partial decay rates, restrained sustain. |
| soft oscillator | Synthetic sounds are valid when named honestly as synthetic. | Stable oscillator/filter behavior, no fake acoustic naming, balanced gain. |
| noise texture | Noisy timbres need noise as identity, not decoration. | Filtered noise carries the sound, with weak pitch only as support. |
| low body | Low support sounds must avoid foreground melodic brightness. | Fundamental-heavy, lowpass, slow enough envelope, controlled gain. |

### Consequences For Function Lab

- The Function Lab palette must expose source family and synthesis engine in
  Timbre Lab.
- `body-pluck` and `muted-body-pluck` may coexist only if their damping,
  brightness, and foreground distance are audibly different. Otherwise one must
  be removed.
- No functional melody timbre may borrow the old melody palette name unless its
  construction has been validated against the source-family obligation.
- A random timbre choice is explainable only if the debug data can say which
  family and engine were chosen.

Examples:

### Flute-like, if implemented locally

- excitation: breath jet
- resonator: open air column
- spectrum: fundamental-heavy at soft dynamics, more upper harmonics when loud
- envelope: soft attack, sustained
- required synth features: sine/low-harmonic oscillator, breath noise,
  amplitude envelope, filter envelope, subtle vibrato
- name rule: call it `flute` only if the result is validated against source
  knowledge; otherwise use an abstract name such as `airy-lead`

### Reed-like, if implemented locally

- excitation: reed-like nonlinear source
- resonator: air column
- spectrum: stronger odd-harmonic character than flute-like tones
- envelope: quicker attack than flute, sustained
- required synth features: square/clarinet-like harmonic source, lowpass/formant
  filtering, subtle noise and pitch instability
- name rule: do not call it `clarinet` without validation

### Plucked-string-like, if implemented locally

- excitation: short pluck/noise burst
- resonator: string plus body
- spectrum: decaying harmonics, body resonance, possible pick/finger noise
- envelope: fast attack, decay; brightness decays with amplitude
- required synth features: Karplus-Strong/waveguide or buffer pluck plus body
  filtering and damping
- name rule: do not call it `guitar` unless the string/body behavior is
  modeled or validated

### Struck-bar-like, if implemented locally

- excitation: strike
- resonator: bar/idiophone
- spectrum: inharmonic or non-string decay pattern
- envelope: sharp attack, decaying partials
- required synth features: additive/filtered partials, non-identical decay per
  partial
- name rule: `marimba` or `music-box` requires source validation; otherwise use
  abstract struck/pluck names

### Electronic lead

- excitation: oscillator
- resonator: electronic filter
- spectrum: intentionally synthetic
- envelope: short/gated/sustained according to role
- name rule: synthetic names are honest and acceptable, but intrinsic loudness
  must be balanced because some waveforms read closer than others

## Random Selection Policy

Randomness should be hierarchical:

1. Choose role family using musical context.
2. Choose one timbre model within that family.
3. Choose parameter variation inside the model.
4. Apply role-distance guardrails.

This preserves randomness while making it explainable:

```txt
This song chose a middle-distance melody.
It selected the plucked-string family.
Within that family it selected a warm decaying pluck model.
The model parameters were randomized within documented ranges.
```

The explanation matters because user trust comes from seeing that randomness is
structured by reasonable categories, not arbitrary AI taste.

## Current Lab Status

The current melody timbres are still mostly synthetic and too similar. The
Timbre Lab is useful because it reveals this directly. Do not treat the current
catalog as final.

Known issues:

- Melody has many entries, but fewer independent timbre families than the list
  suggests.
- Counter has too few timbre alternatives.
- Several local timbres are named or weighted in ways that reflect iteration
  history rather than final taxonomy.
- `chip-lead` is intrinsically prominent and should remain lower in instrument
  gain while the catalog is being redesigned.

## Immediate Next Work

1. Use `timbre_axis_lab.html` to verify the Web Audio control model before
   adding or naming more timbres.
2. For each perceptual axis, compare variants while holding pitch, duration,
   and gain mostly constant.
3. Promote an operation into a timbre model only after it creates an audible
   axis difference in isolation.
4. Compare loudness and perceived foreground distance only after the synthesis
   operation itself is distinguishable.

## Axis Lab Contract

`timbre_axis_lab.html` is not an instrument audition page. It is a control
surface for checking whether Web Audio operations actually move intended
perceptual axes.

Current axes:

- harmonicity: harmonic vs odd vs inharmonic partial ratios
- noise role: no noise vs attack noise vs sustained noise vs noise carrier
- attack identity: soft air vs pluck impulse vs strike vs gated oscillator
- brightness decay: static filter vs closing filter vs separate upper-partial
  decay
- resonance/body: dry vs formant body vs comb body
- pitch stability: stable vs vibrato vs jitter
- foreground distance: close vs middle vs background through gain, attack, and
  brightness

This page should be used before Function Lab timbres are redesigned again. If
an axis cannot be heard clearly in isolation, combining it into a named timbre
will only hide the failure.
5. Only then expose the models to random melody selection.

Do not add more names until the model table exists.
