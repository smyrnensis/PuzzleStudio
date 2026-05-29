# 3D MVP Conformance Cases

These cases define target behavior before implementation. They should become
automated tests only after PS Next integration points are confirmed locally.

## Case 1: 2D Compatibility Baseline

Given a normal 2D PuzzleScript Sokoban file, compiling and playing it in 2D mode
must behave exactly as upstream PS Next does.

## Case 2: Explicit 3D Mode

Given a source containing the `three_dimensions` prelude flag and ordinary
`LEVELS`, the compiler should enter 3D mode only for that game and should not
reinterpret ordinary 2D `LEVELS` blank lines.

## Case 3: 3D Dimensions

Given a 3D level with two slices of equal width and depth, the compiled level
should expose width, depth, and height separately.

## Case 4: Slice Shape Error

Given a 3D level where one slice has a different row count or row width, compile
should fail with a source-facing diagnostic.

## Case 5: Six Direction Movement

Given a 3D rule using directional expansion, `left`, `right`, `front`, `back`,
`up`, and `down` should be valid semantic directions in 3D mode.

## Case 6: Unsupported Feature Guard

Given a 3D source using a PS Next feature outside the MVP support set, compile
should fail explicitly instead of silently lowering to incorrect behavior.
