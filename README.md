# PuzzleStudio

ターンベース・グリッドベース・ルール駆動型パズルゲームのための実験環境。

現時点では、Rust の `puzzle-core` が高速で決定論的な状態遷移を担当し、`puzzle-lang` が `.puzzle` を低レベル IR にコンパイルし、`ascii-play` がテストプレイ用 UI、`html-play` が単体 `.html` エクスポートを提供する。

## Run

HTML:

```bash
cargo run -p html-play -- games/spec_2d/game.puzzle
```

デフォルトでは `games/spec_2d/game.html` を生成する。出力先を変える場合:

```bash
cargo run -p html-play -- games/spec_2d/game.puzzle -o /tmp/spec_2d.html
```

旧ローカルサーバで確認する場合:

```bash
cargo run -p html-play -- games/spec_2d/game.puzzle --serve
```

起動後に表示される `http://127.0.0.1:<port>` をブラウザで開く。

Editor:

```bash
tools/generate_editor.sh games/spec_2d/game.puzzle
```

デフォルトではプロジェクト直下の `editor.html` を生成する。出力先を変える場合:

```bash
tools/generate_editor.sh games/spec_2d/game.puzzle -o /tmp/editor.html
```

ローカルサーバで Run Preview しながら編集する場合:

```bash
cargo run -p html-editor -- games/spec_2d/game.puzzle --serve
```

editor は `html-play` とは別 binary。生成 HTML には初期 preview と source を埋め込み、右ペインの
Level で level を追加して `.puzzle` として export できる。Solver は同じ右ペインの別画面で解探索と
解の再生/書き出しを行う。再コンパイルは `--serve` 時の Run Preview で行う。

`tools/generate_editor.sh` は先に editor 用 WASM を更新し、その後で
`cargo run --release -p html-editor -- ...` を使う。`target/release/html-editor` を直接実行すると、
`static/editor.js` や `html-play/static/standalone.js`、WASM export を変更した直後に古い binary /
古い WASM から stale な `editor.html` を生成してしまうことがあるため、standalone editor を生成するときは
この script を使う。

GitHub Pages など静的ホスティング向けに、standalone editor と WASM preview fallback をまとめて生成する場合:

```bash
tools/generate_web_editor.sh games/spec_2d/game.puzzle -o docs/index.html
```

ASCII:

```bash
cargo run -p ascii-play -- games/spec_2d/game.puzzle
```

操作:

- `w/a/s/d` または矢印キー: 移動
- `r`: 現在ステージをリスタート
- `n`: クリア後に次ステージへ
- `q`: 終了

## Test

```bash
cargo test
```

## Project Layout

```txt
crates/core/
  汎用 transition core。表示・入力・ファイル読み込みに依存しない。

crates/lang/
  `.puzzle` parser/compiler。authoring syntax を `puzzle-core` の IR と level metadata に落とす。

crates/play/
  ロード済みゲームの session 管理と state 描画 helper。parser は持たない。

crates/ascii_play/
  terminal adapter。ファイル選択、キー入力、terminal 表示を担当する。

crates/html_play/
  browser adapter。単体 `.html` を生成し、必要に応じてHTTP経由のローカルHTMLテスト環境も提供する。

games/
  仕様カテゴリごとの小さな検証 entry。大量の遊び用・実験用サンプルは
  `archive/games/legacy_samples/` に退避し、通常のテスト入口にはしない。
  top-level `title` などの game prelude metadata を持つ `.puzzle` が entry。
  play / build / editor に folder を渡すと、その folder 内の prelude-bearing
  `.puzzle` を entry として読む。`game.puzzle` は慣例名として優先されるが必須ではない。
  prelude のない `levels.puzzle` や `sprites.puzzle` は import fragment。
  外部 CSS / JS は entry `.puzzle` の `assets { ... }` で明示参照したものだけを読み込む。
  例: `assets { ... }` に `css "game.css"` / `script "visuals.js"` を書く。
  HTML renderer の sprite class は object 名の大文字・小文字を保持し、
  CSS class に使えない区切り文字だけ `-` に置き換える。例: `Player`
  は `.sprite.Player`、`Portal:one` は `.sprite.Portal-one`。
  画像・音声などの asset は `sprites/`, `sounds/` などに置き、game folder からの相対パスで参照する。

User-facing docs:

AUTHORING_SYNTAX.md
  `.puzzle` を書くための canonical 文法リファレンス。

README.md
  実行方法、プロジェクト構成、主要 entrypoint。

Developer-facing docs:

DESIGN_PRINCIPLES.md
  最上位の設計原則。文法の一貫性、所有者境界、hardcode 回避の判断基準を含む。

CURRENT_SPEC.md
  現時点の実装仕様。parser / lowering / runtime / adapter の実際の契約。

IMPLEMENTATION_PLAN.md
  runtime / authoring / solver に向けた実装計画。

PUBLICATION_PLAN.md
  ローカルアプリ、Web/WASM、standalone HTML の公開・配布方針。

EDITOR_COMPLETION_PLAN.md
  `.puzzle` editor の予測提示 / 補完の実装計画。

SOLVER_DESIGN.md
  solver の役割と設計方針。
```

## Current Architecture

```txt
.puzzle authoring file
  -> puzzle-lang parser/compiler
  -> typed orientation AST
  -> puzzle-core CompiledGame IR
  -> puzzle-play session/render helpers
  -> transition_state / transition_trace
  -> HTML export / ASCII play / future solver
```

`core` はゲーム固有ルールを持たない。2D 仕様検証用のルール、ステージ、表示文字、入力割り当ては `games/spec_2d/game.puzzle` にある。
