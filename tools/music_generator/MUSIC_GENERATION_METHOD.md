# Current Music Generation Method

This document explains how the current music generator works, with emphasis on
where the randomness comes from and what is still hand-authored. It is meant to
make the generator inspectable, not to make it sound more principled than it is.

## Short Description

The generator is a deterministic seeded stochastic system. Given the same seed
and options, it produces the same song. Given a different seed, it samples a new
combination of musical parameters from code-defined probability distributions.

It is not an AI model at runtime. It does not call a language model, search a
database of songs, or select from a library of complete human-written examples.
The output comes from JavaScript functions, seeded pseudo-random numbers,
weighted choices, generated numeric fields, and event rules.

That does not mean it is "pure random." The code contains human-authored musical
priors: scale names, role names, carrier names, weighted options, range limits,
and formulas for density, phrase energy, contour, timing, and velocity. The
randomness operates inside that authored space.

## What Is Random

The main random source is a seeded PRNG created from text seeds. The generator
uses it for:

- key and scale selection
- broad form parameters
- role carrier selection
- timbre fields
- harmonic progression
- section state trajectory
- motif family trajectory
- rhythm cells
- contour cells
- bar-level timing, count, duration, and pitch realization
- event velocities and textural details

Most choices are not uniform. They are weighted by current state. For example,
a sparse form state can make thinner carriers or fewer events more likely, and a
higher-tension bar can make denser or more active gestures more likely.

So the correct claim is:

> The generator explores a seeded stochastic space shaped by hand-written
> musical constraints.

The incorrect claim would be:

> The generator has no authored assumptions.

It has many authored assumptions.

## What Is Hand-Written

The following are hand-authored vocabulary or constraints:

- the available scale list
- the role names
- the carrier names
- the candidate carrier sets for each role
- the numeric weights used in weighted choices
- range limits for section state values
- the event-generating functions for each carrier
- the mapping from generated events to playback tracks
- the broad synthesis and timbre model

This is a real limitation. A carrier named `bass-riff` does not emerge from the
random field by itself. The code defines that such a carrier exists, then uses
random fields to decide how that carrier behaves in a particular song.

The current implementation is therefore not "AI-free" in the sense of having no
human judgment. It is AI-free in the runtime sense: no model is generating bars
from learned musical knowledge while the page runs.

## Templates Versus Vocabulary

The generator tries not to use fixed song templates or fixed note patterns.
However, it does use a fixed vocabulary.

Examples of fixed vocabulary:

- `melodic-line`
- `bass-riff`
- `harmony-arp`
- `rhythm-hook`
- `drum-grid`
- `air-pad`
- `contrast-note`

These are labels for generator behaviors, not complete phrases. For example,
`bass-riff` does not point to one stored riff. It points to code that samples a
rhythm cell, projects it into a bar, generates pitch contour, and emits bass
events.

This distinction matters, but it is not perfect. The more carrier-specific code
there is, the more the generator risks becoming a set of small disguised
templates. The current tests try to catch collapse into a small number of
dominant first-bar signatures, repeated section patterns, or one carrier taking
over too much of the output.

## Seed Layers

The seed is split into:

- `style`: a slower identity seed derived from the seed text after a short
  prefix.
- `variation`: the full seed text.

The style seed controls slower choices such as key, scale, form tendency, role
carriers, and timbres. The full seed controls more concrete composition
material such as progression, section trajectory, motif variants, and events.

This is a practical split, not a complete theory of style and variation. It
allows related seeds to preserve some high-level identity while changing
concrete notes and rhythms.

## Song-Level State

A song starts by choosing:

- key
- scale
- broad form parameters
- role carriers
- timbres
- harmonic progression

The role names are operational labels, not precise music-theory categories:

- `identity`: the material intended to be recognized as the main musical idea.
- `time`: pulse or bar-level timekeeping material.
- `tone`: harmonic support or tonal grounding.
- `motion`: secondary movement or answering material.
- `color`: texture, sustained color, or noise-like atmosphere.
- `boundary`: phrase-edge or section-edge material.

These labels are not always perfectly literal. For example, a `harmony-arp`
carrier can serve identity in one song and motion in another. A `boundary`
carrier may be very subtle. The names are useful for organizing generation, but
they should not be read as proof that the resulting audio clearly expresses
those exact functions.

## Section State

The form is represented by bounded numeric state variables. The main fields are:

- novelty
- stability
- density
- tension
- closure pressure
- memory distance

These values are generated through formulas, random draws, clamps, and rounding.
They are not continuous in a mathematically clean sense, and they are not named
song parts. They are control variables that influence later generation.

The section state affects:

- event density
- phrase energy
- register bias
- contour range
- closure behavior
- motif-family movement
- section transition context

This is one of the areas where the implementation is still more procedural than
elegant. Some parts are smooth fields; some parts are weighted choices; some
parts are explicit enforcement to keep the trajectory from becoming too flat.

## Role Carriers

Role carriers are selected once for the song and reused in every section.

This is a structural choice: section state changes how a carrier behaves, but it
does not replace the carrier with a different instrument role. A song whose
identity is `bass-riff` keeps that identity carrier throughout the form.

This improves continuity, but it also means the generator depends heavily on the
chosen carrier being able to carry enough variation across the whole song.

## Motif Families

Each section has a `motifVariant`, which identifies the motif family used by the
identity material in that section.

The motif-family sequence is generated from:

- number of sections
- a sampled family count
- sampled family IDs
- a sampled dwell rate
- phase
- state drift from tension, memory distance, closure, and progress

This produces low-frequency motif movement. It is not meant to change every bar.
It is also not meant to be a fixed named form. The generated shape can repeat a
family, introduce another, or return to an earlier family depending on the seed
and state trajectory.

The current implementation only models motif family as an ID plus generated
rhythm/contour cells. It does not understand an idea the way a composer would.

## Rhythm Cells

For identity material, motif rhythm is generated as a normalized rhythm cell.
That cell is then projected into each bar's usable step range.

Rhythm is treated as the strongest cue for motif identity. The current code
therefore keeps rhythm variation more conservative than pitch variation for
identity carriers.

This is implemented by:

- generating the cell from the motif family
- projecting the cell into bar space
- applying limited local shift and nudge
- avoiding large additions to the identity rhythm skeleton
- testing that same-family sections have closer coarse timing than
  different-family sections

This does not guarantee a memorable hook. It only makes the generated material
more internally consistent.

## Pitch And Contour

Pitch contour is also generated from motif-family data, but it is allowed to
move more than rhythm.

For pitched identity material, the generator combines:

- a generated contour motif
- local target pitch
- previous-pitch smoothness
- phrase closure
- register bias
- scale-degree candidates

This means two bars from the same motif family may share rhythm while changing
register or contour detail. That is intentional, but the current balance is
still empirical.

## Motif Accent

Identity events receive a motif accent derived from the same rhythm cell. This
applies across identity carriers, including melody, bass, harmony arpeggios, and
rhythmic hooks.

The purpose is to make the motif audible, not just structurally present in debug
data. The accent is still subtle and may not be enough to create a memorable
phrase by itself.

## Harmony

Harmony is comparatively simple. The generator chooses a seeded scale-degree
progression, starts from home, and derives chords from the selected scale. Some
section states shift or offset the progression.

This is not a strong functional harmony model. It does not deeply model cadence,
voice-leading, modulation, or harmonic expectation. It provides a tonal frame
for the generated events.

## Timbre

Timbres are generated from stochastic spectral fields. The generator does not
pick from a small list of named synth presets. Each role gets generated timbre
parameters from a seeded field.

This is one of the stronger randomness claims in the current system: timbre is
not a hand-picked preset per seed. It is generated from numeric distributions.

## Playback Controls

The visible music controls affect broad playback and input normalization:

- height
- tone/brightness
- bpm
- volume
- bars

These controls do not rewrite the composition algorithm. They influence the
generated score and playback rendering within fixed ranges.

## Debug And Tests

The generator exposes debug data such as:

- seed split
- key
- scale
- form
- roles
- timbres
- progression
- section plan
- bar plan
- track mapping

The tests sample many seeds and check structural properties:

- deterministic output for the same seed
- variation across different seeds
- carrier diversity
- stable carriers across sections
- motif-family diversity without collapsing to one pattern
- same-family rhythm being closer than different-family rhythm
- broad progression and phrase-shape diversity
- generated timbres not falling back to old named palettes

These tests support the claim that the generator is seeded and stochastic. They
do not prove that the result is good music.

## Honest Weak Points

The generator is still a procedural music generator, not a composer.

The role labels are useful implementation categories, but they do not guarantee
that a listener hears exactly those functions.

The carrier vocabulary is hand-written. Even though the concrete events are
generated, the set of possible behaviors is constrained by the code's carrier
list.

Some code still uses explicit corrective logic to keep the form from becoming
too flat or too unstable. That is not the same as pure continuous generation.

The motif model is mechanical. It can preserve rhythm identity better than
before, but it does not evaluate memorability directly.

Harmony is shallow compared with the rhythmic and textural systems.

Most importantly, the generator has no listening loop. It can expose structure
and pass distribution tests, but human listening feedback is still required to
judge whether the result feels musical.

## Practical Trust Claim

A careful trust claim for this generator is:

> The output is deterministic seeded randomness inside a hand-authored musical
> grammar. It is not selected from a library of finished phrases, and it is not
> generated by an AI model at runtime. The code defines the vocabulary and
> probability fields; the seed selects one realized path through them.

That is the honest version of the randomness story.
