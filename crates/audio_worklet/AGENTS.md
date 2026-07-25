# Agent Notes

This crate owns the dedicated AudioWorklet-side Rust renderer.

- Consume only versioned, source-free music assets from `puzzle-audio`.
- Keep synthesis and sample-cursor execution in Rust.
- JavaScript beside this crate may register the processor, transport typed
  messages, and copy rendered channel buffers only. It must not interpret
  scores, synthesize audio, or own playback lifecycle policy.
- The processor must render bounded quanta without allocating.
- Do not add a `ScriptProcessorNode` compatibility path.

## Commands

```bash
cargo test -p puzzle-audio-worklet
cargo check -p puzzle-audio-worklet --target wasm32-unknown-unknown
```
