# Publication Plan

この文書は、PuzzleStudio をどう公開・配布するかの方針をまとめる。

目的は、制作環境と公開体験を混同しないことである。PuzzleStudio の中心価値は、`.puzzle` を編集し、実行し、検査し、修正する短いフィードバックループにある。そのため、本格的な制作環境はローカルアプリを本命とし、Web 公開は軽量な共有・試遊・将来のブラウザ版として位置づける。

## 1. Primary Target: Local App

本命はローカルアプリ。

有力候補は Tauri。既存の HTML/CSS/JS editor UI を活かしつつ、Rust の `puzzle-lang`、`puzzle-core`、`html-play`、solver をアプリ内 backend として呼び出す。

```txt
Tauri window
  -> editor UI
  -> Rust commands
  -> parse / compile / preview / solve / export / save
```

ローカルアプリを本命にする理由:

- `.puzzle`、`game.css`、`visuals.js`、画像、音声をプロジェクトフォルダとして自然に扱える
- 編集後の Run Preview を Rust compiler で即時実行できる
- 保存が実ファイル保存になる
- solver や trace など重い処理をローカル CPU で実行できる
- 現在の Rust 実装資産を捨てずに伸ばせる

まずは現在の `html-editor --serve` を制作環境として磨き、その後 Tauri で包む。

配布は、editor と runtime が十分に安定し、作成・保存・preview・export・solver の主要ループが破綻しなくなってから検討する。未成熟な段階では、アプリ配布よりもローカル開発環境としての品質を優先する。

## 2. Web Target: WASM

Web 版の本命は WASM。

GitHub Pages のような静的ホスティングでは Rust binary や local HTTP server を動かせない。そのため、ブラウザ版で編集後 preview まで完結させるには、compiler/runtime をブラウザ内で動かす必要がある。

```txt
.puzzle source
  -> Rust WASM parser/compiler/runtime
  -> browser JS editor
  -> preview iframe
```

WASM 化の対象候補:

- `puzzle-lang`: `.puzzle` parse / validation / compile
- `puzzle-core`: deterministic transition runtime
- `html-play` export logic: preview / standalone HTML generation
- solver: 必要に応じてブラウザ内探索

Web 版で避けるもの:

- ローカルファイルシステムへの暗黙の直接保存
- TCP server 前提の `/api/*`
- native-only な file IO や process 実行

Web 版は、URL だけで試せることを重視する。完全な制作環境ではなく、共有、試遊、サンプル編集、軽量な authoring 体験を主な役割にする。

現在の入口:

```txt
crates/wasm/
  compile_preview(source, puzzle_path, game_css, game_visuals_js) -> HTML
  generate_visuals_js(source, base_visuals_js) -> JavaScript
  highlight_source_html(source) -> HTML
```

最初の統合目標は、editor の `/api/preview` 依存を Web 版では `compile_preview` 呼び出しへ置き換えること。

WASM editor assets は次で再生成する:

```bash
tools/build_wasm_editor.sh
```

この script は `crates/wasm` を `wasm32-unknown-unknown` release build し、`wasm-bindgen --target web` の出力を `crates/html_editor/static/wasm/` に置く。

GitHub Pages 用の standalone editor bundle は次で生成する:

```bash
tools/generate_web_editor.sh games/microban/game.puzzle -o docs/index.html
```

これは `docs/index.html` と `docs/wasm/` を揃える。`index.html` は standalone editor、`wasm/` は静的 Web 版の Run Preview fallback 用。

## 3. Near-Term Web Bridge: Standalone HTML

当面の Web 公開は、生成済み standalone HTML を使える。

```bash
tools/generate_editor.sh games/microban/game.puzzle -o docs/index.html
```

この形なら GitHub Pages へ置ける。初期 preview、埋め込み source、編集 UI、download/export は提供できる。

ただし、静的 HTML だけでは Rust compiler を再実行できないため、編集後の Run Preview は本格対応ではない。これは最終形ではなく、Web 公開の暫定手段として扱う。

## 4. Role Split

```txt
Local app:
  本格制作環境。project folder、save、preview、solver、export を扱う。

Web WASM app:
  ブラウザだけで動く軽量 editor。共有、試遊、サンプル編集を扱う。

Standalone game HTML:
  個別ゲームの配布・投稿・共有用。

GitHub Pages:
  サンプル、作品一覧、Web WASM 版、または standalone editor/game の公開先。
```

## 5. Architecture Implication

公開方針は、既存のレイヤー分離を強める方向で進める。

- `puzzle-core` は deterministic runtime として UI / file IO から独立させる
- `puzzle-lang` は `.puzzle` authoring syntax と validation / lowering を所有する
- `puzzle-play` は session mechanics を所有する
- `html-editor` / future Tauri UI は制作体験を所有する
- Web 版は backend API 依存を減らし、WASM boundary を明確にする

Rust を捨てることは現時点の主方針ではない。ブラウザ公開のために必要なのは、Rust 実装を捨てることではなく、local server 依存と browser-executable core の境界を整理することである。
