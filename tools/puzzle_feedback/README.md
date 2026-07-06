# puzzle-feedback

Small state-dump tool for checking `.puzzle` rule changes from the command line.

Run from the repository root:

```bash
cargo run --manifest-path tools/puzzle_feedback/Cargo.toml -- games/TPGJ6/locked_fixed_space_detection.puzzle --level 0
```

Useful options:

```bash
--level N
--inputs right,down,left
--watch Locked,Open,Room,Player,Box:movable,Box:stack
--cells
```

The tool starts the requested level, runs level-start logic, applies any inputs,
then prints the active scene, level, variables, ASCII board, watched object/group
positions, and optionally all non-empty cells.
