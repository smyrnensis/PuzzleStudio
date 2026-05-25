# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A turn-based, puzzle-based, rule-driven puzzle game framework inspired by PuzzleScript. The goal is a framework where **data alone defines the game** — no code changes needed to create new puzzles.

## Key Documents

| Document | Purpose |
|---|---|
| `設計思想.md` | Design philosophy — the authoritative spec for game mechanics and rule system behavior |
| `docs/フレームワーク計画.md` | **Implementation plan** — complete technical spec with type definitions, algorithms, JSON schemas, module APIs, test cases, and phase-by-phase build instructions. **Read this before implementing anything.** |

## Project Status

**Implementation ready.** Technology stack is chosen (TypeScript + pnpm monorepo). See `docs/フレームワーク計画.md` for the full implementation plan.

## Technology Stack

- **Language**: TypeScript (strict mode, ES2022+)
- **Package management**: pnpm workspaces (monorepo)
- **Build**: tsup (libraries) + Vite (apps)
- **Test**: Vitest (target: 90% coverage for engine)
- **Rendering**: HTML5 Canvas 2D
- **Distribution**: vite-plugin-singlefile (single HTML file)

## Core Design Principle

```
Game = (InputEvent, GameState) -> GameState
```

All game logic is a pure function: deterministic, testable independently of any engine, and reproducible via input history.

## Architecture

Five core components:

| Component | Role |
|---|---|
| **GameManager** | Central controller; sole owner of GameState; handles input, level management |
| **RuleProcessor** | Stateless engine: takes GameState + rules, returns new GameState |
| **ObjectDB** | Master registry of object definitions; resolves properties and tags |
| **HistoryManager** | Undo/redo via GameState snapshot stacks (undo survives restart) |
| **LevelLoader** | Parses level data (ASCII art + legend) into initial GameState |

**Ownership rule:** Only GameManager may own/mutate GameState. RuleProcessor returns new state; all other components get read-only copies.

## Key Design Constraints

- **Puzzle-only** — no free coordinates; all positions are puzzle cells
- **Turn-only** — no real-time elements
- **Integer-only** — Bool and Int types only (no floats)
- **Immutable state** — every state change produces a new GameState instance
- **All-or-nothing rules** — each rule application fully succeeds or fully fails; no observable intermediate states
- **Deterministic execution** — pattern matching scans left-to-right, top-to-bottom with fixed direction priority

## Rule System Summary

Rules are declarative data (JSON), not code. Format: `Pattern_Before -> Pattern_After`.

- **Application modes:** `"once"` (first match only) or `"until_stable"` (repeat until no change — for gravity/cascades)
- **Object-level replacement:** only matched objects in a cell are replaced; other objects in the same cell are untouched
- **Directions:** `none`, `up`, `down`, `left`, `right`, `vertical`, `horizontal`, `any` (auto-generates rotated variants)
- **Cell conditions:** `objects` (match & replace), `has_objects` (existence check only), `no_objects` (absence check)
- **Tag binding:** `$variable` syntax inherits tag values across before/after patterns (e.g., `Player:$color -> Box:$color`)
- **Multiple pattern blocks:** one rule can match multiple independent puzzle locations; all must match for any to apply
- **Rule groups:** ordered groups with their own application mode; callable via `call` effect
- **Global conditions/effects:** conditions check global variables; effects include `set`, `change`, `sound`, `message`, `call`

## Data Structures

- **GameState** = `global_state` (Map<String, int|bool|String>) + `puzzle_state` (2D puzzle where each cell holds a list of objects)
- **Objects** are strings: `"Name:tag1:tag2"` (e.g., `"Player:right"`, `"Box:on_goal"`)
- **Properties** (`@Movable`, `@Solid`, `@Falling`, etc.) allow rules to target categories of objects without naming each one
- **Levels** use ASCII art maps with a legend mapping single characters to object arrays

## Package Structure

```
packages/engine/    — Core rule engine (zero dependencies, pure TypeScript)
packages/renderer/  — Web Canvas renderer (depends on engine)
apps/playground/    — Development web app (depends on both)
games/              — Pure JSON game data
```

**Dependency rule:** engine has ZERO external dependencies. renderer depends only on engine. Never add DOM/browser APIs to engine.

## Implementation Phases

Follow the phases in `docs/フレームワーク計画.md` strictly in order:
1. Project scaffold + type definitions
2. ObjectDB + LevelLoader
3. Pattern matching (most complex module)
4. Rule processor
5. GameManager + HistoryManager
6. Web renderer
7. Game builder (single HTML output)

**Each phase must pass its tests before moving to the next.**

## Input System Design

Player movement is **rule-driven**, not hardcoded. GameManager sets `_input_direction` global variable, then rules react to it. This keeps movement logic in data, not code.

## Pipeline

PuzzleScript syntax (human-readable) → Parser → JSON rule data (machine-readable) → Rule engine

**Note:** PuzzleScript parser is OUT OF SCOPE for initial implementation. Rules are defined directly in JSON.
