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
- common game SFX types: random, jump, pickup, hit, explosion, laser, powerup,
  select, error, and category-less `Wild`
- deterministic tone, noise, and transient layers
- pitch sweeps and short tonal phrases
- seeded synthesis profile for arcade, soft synth, bit-crush, and toy-speaker
  style variation
- compact JSON that can be stored instead of an audio asset

The Music screen uses the latest clean loop generator in `seeded_music.mjs`. It
expands a seed plus music-facing controls into:

- user-facing tone, BPM, and volume controls
- playback-ready score JSON
- fixed thirty-two-bar loop length
- BPM
- key and scale
- generated form, phrase, hook, and arrangement debug data
- chord progression
- lead, counter, harmony, bass, and drum events
- generated drum parameters and hit placement, exposed in debug data rather than
  selected from named kit templates
- timbre map for direct WebAudio scheduling

The browser demo makes randomize-and-play the primary action in both screens.
SFX type selection is a target for random seed creation, but the seed remains
plain numeric. `Random` means no type override: the seed hash deterministically
resolves to one concrete game SFX type. Concrete types such as `pickup` and
category-less `Wild` are explicit type overrides that should be written
separately from the seed when the result is moved into PuzzleScript-style data.
Music keeps loop controls such as tone, BPM, and volume on the Music screen only.
Detailed synthesis choices stay seeded instead of becoming low-level knobs.

Run the structural tests:

```sh
node tools/music_generator/test/seeded_sfx.test.mjs
node tools/music_generator/test/seeded_music.test.mjs
node tools/music_generator/test/seeded_music_lab.test.mjs
node tools/music_generator/test/audio_export.test.mjs
```

Try the browser demo:

```sh
cd tools/music_generator
python3 -m http.server 8875
```

Then open `http://localhost:8875`.

The music lab keeps the editor-integrated generator untouched and loads the
experimental music path from `seeded_music_lab.mjs`:

```txt
http://localhost:8875/lab.html
http://localhost:8875/music_lab.html
```
