# Agent Notes

This crate owns deterministic, source-free audio generation and playback
lifecycle semantics.

## Boundaries

- Consume validated typed sound recipes; do not parse `.puzzle` source.
- Keep seeded SFX/music generation, canonical sample streams, asset identity,
  and play/pause/resume/stop state deterministic and platform-independent.
- Do not access files, DOM, WebAudio, Bevy ECS, audio devices, clocks, or JSON.
- Backends consume resolved assets and `AudioDeviceCommand` values. They must
  not receive or reinterpret authoring seeds, type names, music grammar, or
  lifecycle policy.
- Device failure is reported as an audio diagnostic and must not invalidate an
  otherwise valid game session.

## Tests

```bash
cargo test -p puzzle-audio
```
