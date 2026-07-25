# Agent Notes

This crate owns the browser WebAudio device adapter.

- Consume only `puzzle-audio` catalog assets and device commands.
- Do not parse source, resolve asset names, synthesize score semantics, or own
  play/pause/resume/stop policy.
- Stream canonical music samples; never materialize a complete long-form music
  loop as PCM.
- Surface browser capability and device failures explicitly so the audio
  runtime can reconcile them without invalidating the game session.

## Generated Worklet Bundle

`generated/puzzle_audio_worklet.js` is generated from
`../audio_worklet/src/lib.rs`, `../audio_worklet/worklet.js`, and wasm-bindgen
output by `tools/build_wasm_player.sh`. Never edit it directly. The generated
bundle is one declared player-build output even though it contains both binding
glue and the minimal processor transport.
