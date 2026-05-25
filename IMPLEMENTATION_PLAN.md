# Implementation Plan

この文書は、現時点の実装計画と実装済み範囲を定義する。

優先するものは、高速な状態遷移、表示層から独立した core、solver で大量実行できる runtime、AI が後から操作できる正規化 IR である。

## 0. Current Status

実装済み:

- Rust workspace
- `puzzle-core`
  - layer-slot `State`
  - `CompiledGame`
  - `Rule` / `Guard` / `Pattern` / `WriteOp`
  - all-or-nothing `Patch`
  - `transition_state`
  - `transition_trace`
- `puzzle-lang`
  - `.puzzle` parser/compiler
- `puzzle-play`
  - ASCII renderer
  - restart / next level / undo / redo
- `ascii-play`
  - keyboard / arrow input
  - terminal refresh
- `html-play`
  - browser test UI
- `games/spec_2d/game.puzzle`
  - objects
  - render overlays
  - input bindings
  - direction-expanded rules
  - multiple levels

Run:

```bash
cargo run -p ascii-play -- games/spec_2d/game.puzzle
```

Test:

```bash
cargo test
```

## 1. Language and Runtime

実装言語は Rust。

理由:

- 状態遷移を低 allocation で書ける
- packed state / hash / cache を制御しやすい
- solver と同じ core を共有しやすい
- 後で WASM に出せる

中心関数:

```txt
transition_state(compiled_game, state, input) -> next_state
transition_solver_state(compiled_game, state, input) -> next_state_without_visuals
transition_trace(compiled_game, state, input) -> step_trace
```

`transition_state` は通常の state-only runtime path。
`transition_solver_state` は solver 用 path で、visual object を除いた state を扱う。
`transition_trace` は debug / AI explanation / human inspection 用。

## 2. Repository Layout

```txt
crates/core/
  表示・入力・ファイル読み込みに依存しない transition core。

crates/lang/
  `.puzzle` parser/compiler。authoring syntax を low-level IR に落とす。

crates/play/
  ロード済みゲームの session 管理と state 描画 helper。parser は持たない。

crates/ascii_play/
  terminal adapter。ファイル選択、キー入力、terminal 表示を担当する。

crates/html_play/
  browser adapter。HTTP 経由で state と command をやりとりする。

games/
  `.puzzle` authoring files。

AUTHORING_SYNTAX.md
  現在の `.puzzle` 文法。
```

`core` は DOM、terminal、filesystem、network、`.puzzle` parser に依存しない。

`ascii-play` / `html-play` は adapter であり、parser/compiler を持たない。

## 3. Representation Layers

```txt
human / AI authoring syntax
  -> puzzle-lang compiler
  -> CompiledGame IR
  -> puzzle-play session/render helpers
  -> transition core
  -> ASCII play / future solver / future viewer
```

surface syntax は読みやすさを優先する。
core IR は決定論性、実行速度、差分検証を優先する。

`input` は canonical state ではない。
`transition(state, input)` に渡される transition context value であり、rule guard から参照される。

## 4. Canonical Gameplay State

canonical state は、ゲームロジックに影響する永続状態だけを持つ。

```txt
CanonicalState =
  board state
  + visible var state
```

現時点の実装では visible var state は rule effect から更新できる。

禁止:

- hidden persistent gameplay state
- 盤面や表示から説明できない var flag
- rule のためだけに残る invisible marker

許可:

- board 上の visible object
- UI に明示表示される visible var state
- board / visible var から計算可能な derived cache
- transition 中だけ存在する transition-local value
- solver だけが使う metadata

## 5. Cell Model

セルの意味モデルは multiset ではなく set。

```txt
Cell = finite set of visible objects
```

同じ cell の同じ layer には最大 1 object しか存在できない。

```txt
Invariant:
  at most one object per (cell, layer)
```

layer は描画順だけでなく、同居制約を定義する gameplay model の一部である。

同一オブジェクトの不可視な重なりは禁止する。
複数性が必要な場合は、`CoinPile:3` や visible var のように可視化する。

## 6. Runtime State Encoding

実装は layer-slot cell。

```txt
State {
  width: u16
  height: u16
  layer_count: u16
  slots: Vec<ObjectId>
  visible_vars: Vec<i64>
  derived_cache: object_counts
  hash: u64
}

slot_index = ((y * width + x) * layer_count + layer_id)
slots[slot_index] = object_id | EMPTY
```

`EMPTY = ObjectId(0)`。
concrete object は `ObjectId(1..)`。

`State` equality は canonical state を比較し、derived cache と hash は意味上の比較対象にしない。

## 7. Authoring Syntax Direction

現在の `.puzzle` 文法は、statement list と inline rewrite を中心にする。

```txt
main {
once input [ Player | Box | ] -> [ | Player | Box ]
once input [ Player | ] -> [ | Player ]
}
```

`main` は必須の entrypoint。単純な rule sequence は `main` に anonymous inline rewrite を直接並べる。`rule` は名前付き block abstraction が必要なときに使う。

`up` / `down` / `left` / `right` は標準 semantic input として扱い、cardinal direction set は省略時のデフォルトとして推論する。

`direction <alias> <up|down|left|right>` は、direction / input 文脈で使う別名を標準方向へ lower する。古い `direction <input_name> <dx> <dy>` は public syntax ではない。

`input [ ... ] -> [ ... ]` は、入力方向に連動する rewrite の標準形。parser はこれを `OrientationExpr::Input` として読み、lowering が input ごとの low-level rule variants に特殊化する。

compiler は `for <binding> in directions|horizontal|vertical { ... }` から statement variants を生成し、inline rewrite 差分から low-level IR を生成する。

value set / object schema は `puzzle-lang` で concrete object variants に展開する。

```txt
color = red blue
object player:color 1
legend p = player:red
legend q = player:blue
```

pattern 側では `player`, `player:red`, `player:color`, `player:left` のような selector を受け付ける。selector alternatives は lowering で concrete low-level rule variants に展開する。

## 8. Compiled Game IR

`CompiledGame` は transition が直接読む正規化済み構造。

現時点の主要構造:

```txt
CompiledGame:
  layer_count
  objects
  rules

Rule:
  id
  guards
  application: Once | UntilStable (default)
  pattern
  writes

Guard:
  InputIs(input_id)

Pattern:
  MatchCell[]

MatchCell:
  dx, dy
  require_objects
  forbid_objects

WriteOp:
  Add
  Remove
  Replace
```

今後の拡張候補:

- property matching
- named query
- visible var guard/write
- phase
- event emission

## 9. Transition Pipeline

概念上の transition:

```txt
input context setup
  -> rule loop
  -> guard
  -> match
  -> build patch
  -> validate/apply patch as a single unit
  -> for UntilStable, enqueue dirty origins from write/match footprint deltas
  -> update derived cache
```

Authoring syntax:

- named rule: `rule <name> [once | repeat]`
- anonymous inline rewrite: `[once | repeat] <orientation> [ ... ] -> [ ... ]`
- named rule application is block-level; rewrite application is line-level

概念境界:

- guard は transition context / state facts を見る
- matcher は patch を知らない
- patch は state 変更予定リスト
- patch apply が derived cache を更新する
- transition が全体を orchestrate する

solver 用 hot path では、意味を保ったまま一部を融合してよい。

## 10. Patch

patch は状態変更予定リスト。

```txt
PatchOp:
  add object at cell
  remove object at cell
  replace object at cell
```

現時点では visible var write は実装済みで、event は未実装。

patch の役割:

- ルール適用を all-or-nothing にする
- conflict を検出する
- trace の材料にする
- derived cache を差分更新する

## 11. Derived Cache and Queries

query は、現在の canonical state から計算できる読み取り専用の派生事実。

現時点では `State` が `object_counts` を derived cache として持つ。

方針:

- rule が count を直接 `+1` してはいけない
- patch apply が board delta から cache を更新する
- derived cache は canonical state の意味ではない
- debug build では再計算検証を追加したい

今後:

- `count(Box)`
- `exists(Goal) and count([ Goal no Box ]) == 0`
- local query
- indexed query

## 12. Play Boundary

`puzzle-lang` は authoring syntax の実験層。

持ってよいもの:

- `.puzzle` parser/compiler
- AST / validation / lowering

`puzzle-play` は core から独立した play helper 層。

持ってよいもの:

- ASCII renderer
- level switching
- simple goal display/check

`ascii-play` / `html-play` は adapter。

持ってよいもの:

- terminal/browser input
- state display
- file/server setup

持ってはいけないもの:

- core transition の真実
- solver が依存するゲーム固有ロジック
- `.puzzle` parser/compiler
- hidden gameplay state

## 13. Solver Interface Direction

solver が見る interface は狭くする。

```txt
step(compiled_game, state, input) -> state
hash(state) -> StateHash
equals(a, b) -> bool
is_goal(state) -> bool or external goal predicate
```

solver metadata は gameplay transition に入れない。

## 14. Near-Term Work

優先順:

1. `puzzle-lang` 内で parser / AST / lowering module を分割する
2. `main` / `rule` / `for` / `if` の statement AST を整理する
3. `no` / `group` を property matching に拡張する
4. core に property matching を入れる
5. core に phase を入れる
6. solver smoke test を追加する

## 15. Non-Goals for Now

- visual editor
- full PuzzleScript compatibility
- arbitrary scripting
- random / realtime behavior
- animation
- polished renderer
- large-level optimal solving

## 16. Core Design Rule

authoring は豊かにしてよい。
実行核は貧しく保つ。

人間と AI のための意味は authoring syntax / compiler metadata / trace / examples に置く。
solver が回す hot path は、整数 ID、layer-slot 配列、derived cache、patch、deterministic transition だけにする。
