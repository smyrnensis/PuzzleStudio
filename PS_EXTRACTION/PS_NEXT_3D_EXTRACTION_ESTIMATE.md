# PS Next 3D Extraction Impact Plan

この文書は、3D extraction を PuzzleScript Next へ統合してもらう可能性を第一候補にしつつ、受け入れられない場合に互換的な fork として成立させるための切り分けと作業量見積もりである。

作業対象は、当面この `PS_EXTRACTION` フォルダ内の計画・仕様整理に留める。ここでは PS Next 本体やこのリポジトリの実装ファイルは変更しない。

## 現時点の前提

- PS Next 本体はこの workspace には含まれていない。
- 2026-05-27 に外部の `david-pfx/PuzzleScriptNext` repo を確認した範囲では、主な source は `src/` と `src/js/` 配下にあり、特に `parser.js`, `compiler.js`, `engine.js`, `graphics.js`, `inputoutput.js`, `editor.js`, `toolbar.js`, `buildStandalone.js`, `solver.js` が影響候補になる。
- PS Next の公開 docs は、section として `Prelude`, `Objects`, `Legend`, `Sounds`, `CollisionLayers`, `Rules`, `WinConditions`, `Tags`, `Mappings`, `Levels` を前提にしている。
- このリポジトリ側の 3D 実装では、方向・座標・frame は `crates/puzzle3d_model/src/model.rs`、3D parser は `parser.rs`、rule lowering は `selector.rs`、実行は `transition.rs`、level は `level.rs`、HTML runtime は `crates/html_play/static/puzzle3_app.js` と `puzzle3_visual_core.js` に分かれている。

不確実性:

- PS Next の関数名単位の最終判断は、PS Next を local checkout して `parser.js`, `compiler.js`, `engine.js` を直接読むまで確定しない。
- この文書の PS Next 側ファイル名は、repo tree と PS 系エンジンの責務分担からの影響候補であり、まだ patch-ready な行番号一覧ではない。

## 目標の切り分け

PS Next に提案しやすい最小統合単位は、「PuzzleScript のルールエンジンを 3D 対応に一般化する」ではなく、「2D 互換を壊さない 3D mode を追加し、その mode だけが 3D board/rule/render を使う」。

やる:

- 方向を 4 方向から 6 方向へ拡張できる表現を追加する。
- 3D level を読み、width x depth x height の board state を作る。
- 基本的な replacement rule を 3D board 上で match/replace できるようにする。
- 3D Sokoban 相当の `Player pushes Box into Goal` を reference example として固定する。
- 2D PuzzleScript の既存挙動を regression test で守る。

やらない:

- Prelude の大量設定整理。
- SFX、music、sound event の 3D 化。
- Tags/Mappings の高度な展開仕様の 3D 完全対応。
- Canvas sprite、tween、metadata twiddling、link/level branching、pause/menu、solver の 3D 完全対応。
- PS Next の editor 全面刷新。

## 統合方式の候補

### A. Upstream-friendly 3D mode

PS Next 本体に `3D` mode を足す。既存 2D path は原則そのままにし、3D source を検出したときだけ別の board dimension と renderer を使う。

利点:

- 既存ユーザーへの破壊的変更が少ない。
- review しやすい。
- PS Next 側で拒否されにくい。

欠点:

- 2D/3D の分岐が複数箇所に残る。
- 高度な PS Next features は 3D 非対応のまま明示する必要がある。

### B. Compatibility fork

PS Next の構文と基本挙動に寄せた fork として `PuzzleScript 3D` を出す。2D 互換は import/playable の範囲に留め、内部は 3D-first にする。

利点:

- 実装速度が速い。
- 3D data model を中心に設計できる。
- editor/runtime を薄くできる。

欠点:

- upstream への統合距離が広がる。
- 「PS Next compatible」と名乗る範囲を厳密に管理しないと、別言語に見える。

推奨:

まず A の形で設計し、実装作業は B でも使えるように isolated module に寄せる。つまり、PS Next 側には `parser/compiler/engine/renderer` へ小さな接続点を作るが、3D 固有処理は `*3d` または `dimensional_*` の別単位に置く。

## PS Next 側の影響候補

### 1. Parser: `src/js/parser.js`

目的:

- 3D mode の検出。
- 3D section または directive の受理。
- 3D level slice の読み取り。
- 6 方向 token の認識。
- 3D rule pattern の token 化。

最小変更:

- `front`, `back`, `up`, `down`, `left`, `right` を 3D direction set として扱う。
- 既存 2D の `up/down/left/right` との衝突を避けるため、3D mode でだけ `front/back` を direction として有効化する。
- `three_dimensions` prelude と ordinary `LEVELS` を唯一の author-facing 3D level surface にする。slice は standalone `;` で表す。
- 3D rule は最初は 1D directional rule に限定する。例: `[ > Player | Box ] -> [ > Player | > Box ]` を 6 方向へ展開できる形。

避ける:

- Prelude 全体の parser 拡張。
- `Sounds`, `Tags`, `Mappings` の 3D 拡張。
- dense 3D pattern を最初の upstream PR に含めること。

作業量:

- MVP: 3-5 日。
- edge diagnostics と docs examples 込み: 1-2 週間。

リスク:

- PS Next は blank line を level separator として使う。3D slice separator も blank line にすると互換性が壊れるため、3D slice は standalone `;` に固定する。

### 2. Compiler/lowering: `src/js/compiler.js`

目的:

- parser output を 3D board/rule IR へ lower する。
- collision layer を `(cell, layer)` から `(x,y,z,layer)` へ拡張する。
- direction expansion を 4 方向から 6 方向へ拡張する。
- property/aggregate/synonym を basic replacement rule で使える範囲に限って解決する。

最小変更:

- 3D mode の compiled state に `width`, `depth`, `height` を持たせる。
- `directions` の展開を mode-dependent にする。2D mode は 4 方向のまま、3D mode は 6 方向。
- `horizontal` は `left/right/front/back`、`vertical` は `up/down` とする。
- 既存 rule compiler が cell index を計算する箇所を、3D mode では `((z * depth + y) * width + x)` 相当の indexer へ差し替える。

避ける:

- PS Next の全 rule feature を 3D 対応にすること。
- Random rules, late/again/gosub/tag mapping などの複雑機能を MVP に含めること。

作業量:

- 3D state/rule IR の追加: 1 週間。
- basic property/aggregate/collision layer 対応: 1 週間。
- diagnostics と 2D regression 修正: 1 週間。

リスク:

- PS の rule semantics は parser より compiler/lowering に深く埋まっている可能性が高い。ここを直接 3D 化すると PR が大きくなる。最初は 3D mode 専用 lowering を横に置く方がよい。

### 3. Engine/runtime: `src/js/engine.js`

目的:

- 3D board 上で match/replace を実行する。
- movement intent と collision resolution を 6 方向で扱う。
- undo/restart/win check が 3D state を保存・復元できるようにする。

最小変更:

- state cell access を direct array access から dimension-aware helper に寄せる。
- 3D mode の neighbor offset を 6 個にする。
- `tryMove`, rule application loop, win condition check 相当の処理で 3D indexer を使う。
- 2D mode は既存 helper が同じ結果を返すことを regression test で確認する。

避ける:

- camera、renderer、editor convenience を engine state に入れること。
- 3D rendering の都合で rule state を変えること。

作業量:

- basic replacement execution: 1-2 週間。
- movement/collision/win/undo/restart 統合: 1-2 週間。
- PS Next 既存 feature との regression: 1-2 週間。

リスク:

- `again`, `late`, `rigid`, random, gosub などの実行順序が basic rules と絡む。MVP では「3D mode supports basic rules only」と明記し、未対応 feature は compile-time error にする方が安全。

### 4. Renderer: `src/js/graphics.js`, `src/js/inputoutput.js`, optional new `src/js/graphics3d.js`

目的:

- 3D board を可視化する。
- keyboard input を 6 方向へ割り当てる。
- 2D renderer を壊さずに 3D renderer を追加する。

最小変更:

- 3D mode では existing 2D canvas sprite renderer を直接拡張しない。まず `graphics3d.js` 相当を追加し、simple isometric / voxel projection で描く。
- 入力は `left/right/front/back/up/down` を semantic input として扱う。
- camera は presentation state として renderer 側に持つ。undo/restart の対象にしない。

避ける:

- SFX、tween、canvas sprite transform、theme 全体の統合。
- 3D sprite editor。
- shadow/lighting の canonical 化。

作業量:

- simple block renderer: 1 週間。
- camera/viewport/pixelate の最低限: 1 週間。
- mobile/editor/standalone 表示調整: 1-2 週間。

リスク:

- rendering が綺麗でも rule semantics の reference にはならない。upstream 提案ではまず「見える最小」に留める。

### 5. Editor/export: `src/js/editor.js`, `src/editor.html`, `src/js/buildStandalone.js`, `src/js/toolbar.js`

目的:

- 3D source を edit -> run -> export できるようにする。
- level editor なしでも、text source と preview で使える状態にする。

最小変更:

- editor は 3D mode を検出して 3D renderer を起動する。
- standalone export に 3D renderer module を含める。
- toolbar には 3D 専用機能を増やさない。Run/Rebuild/Export が動けばよい。

避ける:

- visual level editor。
- sprite editor。
- project workspace。
- solve/GIF の 3D 対応。

作業量:

- editor boot/export wiring: 3-5 日。
- standalone QA: 2-3 日。

リスク:

- PS Next の standalone builder は inlining 前提が強い可能性がある。新規 3D module を追加するなら build script の追従が必要。

### 6. Solver/tests/docs: `src/js/solver.js`, `src/tests`, `src/Documentation`

目的:

- 2D regression を守る。
- 3D MVP の conformance tests を追加する。
- docs に「core 3D」と「unsupported in 3D」を明記する。

最小変更:

- `3D Sokoban` の compile/run/win test。
- 2D canonical Sokoban の regression。
- 3D unsupported features の compile error tests。
- Documentation に `3D mode` ページを 1 枚追加。

避ける:

- 3D solver。
- full docs rewrite。

作業量:

- tests: 1 週間。
- docs: 2-3 日。

## このリポジトリ側から抽出できる部品

直接移植しやすい:

- `Direction3`, `Offset3`, `Size3`, `Coord3` の model。
- `DirectionSet3` の `directions/horizontal/vertical` 分類。
- `Level3` と `LevelBundle3` の width/depth/height validation。
- `MatchCell3`, `Pattern3`, `WriteOp3`, `Rule3` の basic replacement IR。
- `transition_rule_once`, `transition_rule_once_all`, `transition_rule_repeated` の考え方。

そのまま移植しにくい:

- Rust の strong typed IR 全体。
- local frame / dense pattern / variant family / scratch の高度な仕組み。
- current `.puzzle` scene system。
- current HTML editor の workspace/project/preview contract。

抽出方針:

- PS Next へは Rust 実装を移植するのではなく、同じ behavior を小さな JS module と tests に翻訳する。
- この repo の `puzzle3d_model` は reference behavior として使う。上流 PR の説明では「既存 Rust prototype で検証済みの semantics」として扱う。

## MVP の仕様線

MVP に含める:

- `three_dimensions` mode marker。
- Objects / Legend / CollisionLayers / Rules / WinConditions / ordinary `LEVELS`。
- 6 方向: `left`, `right`, `front`, `back`, `up`, `down`。
- `horizontal = left right front back`。
- `vertical = up down`。
- basic directional replacement rule。
- basic property `or` and aggregate `and`。ただし tag/mapping expansion は除外。
- 3D level slice。
- simple renderer。
- undo/restart/win。

MVP から外す:

- Prelude settings の大半。
- SFX/Music。
- advanced PS Next features: tags, mappings, canvas sprites, metadata twiddling, link, gosub, random, checkpoint, solver, GIF。
- dense 3D pattern。
- frame syntax。
- shadow/tween。

## 作業量見積もり

### Upstream-friendly MVP

合計: 6-10 週間。

- PS Next local checkout 調査と architecture note: 2-3 日。
- 3D syntax decision と docs draft: 2-3 日。
- parser/compiler 3D mode: 2-3 週間。
- engine basic replacement/movement/win/undo: 2-3 週間。
- simple renderer/input/export: 1-2 週間。
- tests/regression/docs/PR polish: 1-2 週間。

### Compatibility fork MVP

合計: 3-6 週間。

- PS-like frontend をこの repo の `puzzle3d_model` に lower: 1-2 週間。
- minimal JS renderer/export: 1-2 週間。
- example/tests/docs: 1 週間。
- packaging/license/readme: 1 週間。

### なぜ差が出るか

upstream は 2D compatibility を壊さないための regression と review surface が大きい。fork は 3D-first にできるが、PS Next への統合可能性は下がる。

## 最初の 3 PR に分けるなら

### PR 1: No-behavior architecture prep

内容:

- dimension helper / mode flag / direction table を追加。
- 2D behavior は変更しない。
- docs に 3D proposal draft へのリンクだけ追加。

狙い:

- upstream maintainer に「2D 互換を壊さない拡張面」を見せる。

### PR 2: 3D compile + engine MVP behind explicit mode

内容:

- explicit 3D sample だけが compile/run できる。
- basic replacement rule と win condition。
- renderer は debug/minimal でよい。

狙い:

- semantic value を先に証明する。

### PR 3: User-facing playable 3D

内容:

- simple 3D renderer。
- input/export/editor wiring。
- 3D Sokoban example と docs。

狙い:

- 実際に触れる artifact にする。

## 受け入れられやすい提案文の核

提案の中心は「PuzzleScript を一般ゲームエンジン化する」ではなく、次の形にする。

> This adds an explicit 3D mode for simple PuzzleScript-style grid replacement games. It keeps existing 2D behavior unchanged and starts with a deliberately small supported subset: objects, legend, collision layers, basic directional rules, win conditions, and 3D levels.

強調すること:

- 2D compatibility を壊さない。
- Prelude/SFX/advanced features は触らない。
- 3D mode は small subset から始める。
- 既存 PS mental model の延長で Sokoban in 3D が書ける。
- unsupported features は曖昧に動かさず compile error にする。

## 次にやる具体作業

1. PS Next を local checkout し、`parser.js`, `compiler.js`, `engine.js` の関数単位 map を作る。
2. `PS_EXTRACTION/CORE_SPEC_3D.md` を作り、MVP syntax を 1-2 ページに固定する。
3. `PS_EXTRACTION/PS_NEXT_FUNCTION_MAP.md` を作り、実際に触る関数、読むだけの関数、触らない機能を表にする。
4. 3D Sokoban の single-file example を PS Next style で書く。
5. その example から必要機能を逆算し、PR 1/2/3 の境界を再調整する。
