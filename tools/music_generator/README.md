# Seeded Game Audio Generator Experiment

This is a small standalone experiment for PuzzleStudio game audio generation.
It is not part of the `.puzzle` language yet.

The prototype now has two seeded paths:

- save a compact seed instead of an audio file
- deterministically expand that seed into author-visible structure
- play the result with lightweight browser synthesis

The browser demo has two switchable generator screens. The SFX screen focuses
on one-shot game sound effects. A seed plus an optional type target expands into:

- compact numeric seeds
- user-facing volume control
- common game SFX types: random, jump, pickup, hit, drag, lock, explosion,
  laser, powerup, select, error, and category-less `Wild`
- deterministic tone, noise, and transient layers
- pitch sweeps and short tonal phrases
- seeded synthesis profile for arcade, soft synth, bit-crush, and toy-speaker
  style variation
- compact JSON that can be stored instead of an audio asset

The Music screen uses the canonical seeded composition generator in
`seeded_music.mjs`. It
expands a seed plus music-facing controls into:

- user-facing height, bars, BPM, and volume controls
- playback-ready score JSON
- selectable 8, 16, 32, or 64 bar loop length
- BPM
- key and scale
- generated form, roles, phrase state, and arrangement debug data
- chord progression
- role-based melodic, tonal, rhythmic, motion, color, boundary, and drum events
- generated stochastic timbre and transient fields exposed in debug data rather
  than selected from named kit templates
- timbre map for direct WebAudio scheduling

The browser demo makes randomize-and-play the primary action in both screens.
SFX type selection is a target for random seed creation, but the seed remains
plain numeric. `Random` means no type override: the seed hash deterministically
resolves to one concrete game SFX type. Concrete types such as `pickup` and
category-less `Wild` are explicit type overrides that should be written
separately from the seed when the result is moved into PuzzleScript-style data.
Music keeps loop controls such as height, bars, and BPM on the Music screen only.
Detailed synthesis choices stay seeded instead of becoming low-level knobs.

`drag` is for pulling a box one cell across a floor. It should read as a
successful movement sound, not impact: a short static-friction break, sustained
low or low-mid floor rub, a low crate body layer, and a dull settle near the
end. Drag variants should not use click transients, highpass noise, or melodic
generic variation as their main character.

Run the structural tests:

```sh
node tools/music_generator/test/seeded_sfx.test.mjs
node tools/music_generator/test/seeded_music.test.mjs
node tools/music_generator/test/audio_export.test.mjs
```

Build the standalone music listener when you want to test music without the
editor server:

```sh
node tools/music_generator/build_music_listen_page.mjs
```

Then open `tools/music_generator/music_listen.html` directly in a browser. The
page is generated as one self-contained file from the current music generator
modules, so it does not require the Rust editor server or any editor pane.

Older music lab pages, the previous clean-loop generator, and timbre observation
notes are archived under `archive/tools/music_generator/experimental_music_2026-05-30`.
