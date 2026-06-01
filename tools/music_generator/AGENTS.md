# Music Generator Agent Notes

This folder owns the standalone seeded audio generator experiments used by the
editor sound tools. These files are not `.puzzle` language semantics. Keep
changes local to generated sound structure, browser labs, export helpers, and
their tests unless the task explicitly asks to change editor integration.

## Core Principle

The music generator must produce varied output, produce randomized output, and
make that randomness trustworthy to the user.

Trustworthy randomness means the user can believe that changing a seed explores
a real musical space, not a small set of disguised templates or hand-tuned
presets. A generated result should feel authored by the generator's stochastic
model, not by an agent's one-off adjustments.

This principle has three separate obligations:

- Diversity: many seeds should produce meaningfully different form, rhythm,
  contour, instrumentation, density, and texture.
- Randomness: those differences should come from seeded stochastic choices and
  continuous distributions, not from a fixed catalog with cosmetic variation.
- User trust: debug data and tests should expose enough structure to show why
  the result changed and to catch collapse into repeated patterns.

## What To Avoid

Do not solve music-quality complaints by adding fixed song templates, named form
recipes, or a few handcrafted "good" patterns. If a pattern is useful, express
the underlying distribution or constraint that lets many related patterns exist.

Do not add piles of tiny AI-tuned knobs to chase one pleasing example. Controls
visible to users should be broad musical intent controls. Internal parameters
should be derived from the seed, the form state, or a documented stochastic
model.

Do not make a change that only improves one seed. When a seed sounds bad, treat
it as evidence of a generator rule, distribution, boundary condition, or feedback
gap. Fix the mechanism that produced the bad case and check sibling seeds.

Do not hide determinism behind a random label. Seeded output must remain
repeatable for the same seed and options, and visibly different for seed changes
that are meant to vary content.

## Implementation Guidance

Prefer seeded probability fields, weighted choices, trajectories, and generated
phrase shapes over hand-authored tables. Small fixed candidate sets are allowed
only when they represent a stable musical vocabulary and the surrounding
selection, timing, contour, or transformation remains stochastic enough to avoid
template lock-in.

When changing section behavior, preserve the distinction between form-level
state and bar-level realization. A chorus-like peak may be strong, but its entry
should be prepared by phrase shape, carrier continuity, pickup, density ramp, or
other local transition behavior rather than by globally weakening all peaks.

When adding a new generated field, expose it in debug data if it helps users or
tests evaluate variety, continuity, or structure. Debug data should make the
generator's decisions inspectable without becoming a user-facing control panel.

When changing playback, keep musical intent separate from synthesis rendering.
Generated structure should remain meaningful before WebAudio timbre details are
applied.

## Tests And Evidence

Use distribution-style tests for generator behavior. Good tests sample many
seeds and check that outputs do not collapse to a small set of signatures, fixed
first bars, repeated section halves, or one dominant carrier.

For a targeted complaint, add tests that protect the underlying principle, not
just the observed seed. Examples:

- A harsh section entry should produce tests for transition preparation and
  entry ramp behavior across many seeds.
- A repetitive melody should produce tests for rhythm-profile and contour
  variation across active melodic sections.
- A too-obvious template should produce tests for signature diversity and maximum
  dominance of any repeated pattern.

Run the relevant local structural tests after changes:

```sh
node tools/music_generator/test/seeded_music.test.mjs
```

Also run adjacent music or synthesis tests when touching shared export,
timbre, transient, or legacy generator code.
