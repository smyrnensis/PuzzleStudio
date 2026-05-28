# Agent Notes

This crate owns terminal adapter behavior.

## Scope

Terminal file selection, terminal key reading, and terminal screen refresh live
here. Parser/compiler semantics do not.

Default controls:

- `w/a/s/d` or arrow keys: move
- `r`: sends the standard non-direction `restart` input; it is not an automatic
  session restart effect
- `q`: quit

Use:

```bash
cargo run -p ascii-play -- games/spec_2d.puzzle
```
