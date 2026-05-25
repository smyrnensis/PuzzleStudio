# CLI Implementation Plan

この文書は、PuzzleStudio CLI の目的、責務境界、初期コマンド、実装順をまとめる。

Status: 一部実装済み。`check`、`play`、`preview`、`editor`、`export-html`、
`export-editor`、`import-puzzlescript` は `puzzlestudio` binary から呼べる。
`simulate`、`test`、`format`、`new` はまだ計画段階。

## 1. Purpose

CLI の目的は、ローカルファイルを中心にした制作ループを、AI エージェント、CI、シェル、自動化から扱いやすくすることである。

PuzzleStudio の中心価値は、意図、`.puzzle` source、決定論的実行、検証結果の間の短いフィードバックループにある。CLI はそのループを GUI なしで回すための入口になる。

```txt
human / AI edits .puzzle
  -> puzzlestudio check
  -> puzzlestudio preview / export-html
  -> puzzlestudio simulate / test
  -> human / AI fixes source
```

CLI が特に支える対象:

- Codex / Claude Code などのローカルエージェント
- GitHub Actions などの CI
- shell script による一括変換や検証
- GUI を開かずに行う authoring / export / regression check

## 2. Ownership Boundary

CLI は新しい挙動の所有者ではない。既存の各層を呼び出す薄い adapter として実装する。

- `.puzzle` parsing / validation / lowering は `puzzle-lang` が所有する
- deterministic transition は `puzzle-core` が所有する
- session mechanics、level start、clear、undo/restart/advance は `puzzle-play` が所有する
- HTML preview / standalone export は既存の `html-play` / editor service と共有する
- filesystem traversal、stdout/stderr、exit code、JSON output は CLI adapter が所有する

同じ入力に対して、CLI、HTML editor server mode、Tauri shell、WASM editor が異なる compile / preview / save semantics を持たないようにする。

## 3. Proposed Package

Rust workspace に新しい binary crate を追加する。

```txt
crates/cli/
  Cargo.toml
  src/main.rs
```

Binary name:

```txt
puzzlestudio
```

既存の `ascii-play`、`html-play`、`html-editor` binary は当面残す。CLI が安定した後、共通コマンドからそれらの体験を呼べるようにしてもよい。

現在は `puzzlestudio play` / `preview` / `editor` がそれぞれ既存 adapter の
実装を薄く呼び出す。既存 binary は開発・後方互換用に残す。

## 4. Initial Command Surface

### `puzzlestudio check`

`.puzzle` を読み込み、parse / validation / lowering を実行する。

```bash
puzzlestudio check games/spec_2d.puzzle
puzzlestudio check games/spec_2d/
puzzlestudio check games/spec_2d.puzzle --json
```

Expected behavior:

- 成功時は exit code `0`
- error がある場合は exit code `1`
- warning-only の扱いは最初に決める。初期案は exit code `0`
- `--json` は AI / CI が読める structured diagnostics を返す
- diagnostics は file、line、column、message、severity を持つ

### `puzzlestudio export-html`

standalone HTML を生成する。

```bash
puzzlestudio export-html games/spec_2d.puzzle -o dist/game.html
```

Expected behavior:

- 既存の `html-play` export logic を共有する
- asset path resolution は game folder 基準にする
- output path が未指定なら安全な default を使うか、明示指定を要求するかを実装前に決める

### `puzzlestudio preview`

ローカル preview server を起動する。

```bash
puzzlestudio preview games/spec_2d.puzzle
```

Expected behavior:

- 既存の editor / html preview server behavior を共有する
- 起動時に URL を stdout に出す
- file watching は初期実装では必須にしない

### `puzzlestudio play`

terminal player を起動する。

```bash
puzzlestudio play games/spec_2d.puzzle
```

Expected behavior:

- 既存の `ascii-play` terminal runtime を共有する
- 2D / prototype 3D document の single model を実行する
- terminal 固有の key handling と表示だけを adapter が所有する

### `puzzlestudio editor`

local editor server を起動する。

```bash
puzzlestudio editor games/spec_2d.puzzle
```

Expected behavior:

- 既存の `html-editor` service を共有する
- 起動時に editor URL を stdout に出す
- save / preview / workspace root semantics は editor service が所有する

### `puzzlestudio simulate`

入力列を流し、最終状態や結果を返す。

```bash
puzzlestudio simulate games/spec_2d.puzzle --level first --inputs right,right,up
puzzlestudio simulate games/spec_2d.puzzle --level 3 --inputs @inputs.txt --json
```

Expected behavior:

- `puzzle-play` の session lifecycle を通す
- `on_level_start`、clear 判定、level navigation command の扱いが runtime と一致する
- AI 向けには `--json` で level、turn count、cleared、commands、state summary を返す

### `puzzlestudio test`

将来の authoring examples / regression tests を実行する。

```bash
puzzlestudio test games/spec_2d/
```

Expected behavior:

- 初期段階では設計だけ置く
- `.puzzle` 内の examples を採用するか、隣接ファイルにするかは未決定
- `simulate` が安定してから実装する

### `puzzlestudio format`

`.puzzle` source を整形する。

```bash
puzzlestudio format games/spec_2d.puzzle
puzzlestudio format games/spec_2d.puzzle --check
```

Expected behavior:

- 初期実装では後回し
- formatter は syntax preservation とコメント保持の方針が必要
- AI が編集した source を安定化する用途として重要

### `puzzlestudio import-puzzlescript`

PuzzleScript source を canonical `.puzzle` に変換する。

```bash
puzzlestudio import-puzzlescript source.txt -o game.puzzle
```

Expected behavior:

- 既存の `translate_puzzlescript_to_canonical` を呼ぶ
- 現在の import coverage が限定的であることを diagnostics に含める

### `puzzlestudio export-editor`

standalone editor HTML を生成する。

```bash
puzzlestudio export-editor games/spec_2d.puzzle -o editor.html
```

Expected behavior:

- 既存の `html-editor` export logic を共有する
- editor WASM bundle の更新自体は `tools/build_wasm_editor.sh` が所有する

### `puzzlestudio new`

新規 project scaffold を作る。

```bash
puzzlestudio new my-game
```

Expected behavior:

- 初期段階では後回し
- template は canonical syntax を使い、legacy syntax を含めない

## 5. Output Contract

AI / CI との相性を考えると、human-readable output と machine-readable output を両方持つ必要がある。

Human output:

```txt
error: expected `rules` block
  --> games/foo/game.puzzle:42:1
```

JSON output:

```json
{
  "ok": false,
  "diagnostics": [
    {
      "severity": "error",
      "file": "games/foo/game.puzzle",
      "line": 42,
      "column": 1,
      "message": "expected `rules` block"
    }
  ]
}
```

Rules:

- `--json` must not mix progress text into stdout
- human-readable diagnostics may go to stderr
- JSON result should go to stdout
- command failure should use non-zero exit codes
- paths should be stable and preferably relative to cwd when invoked with relative input

## 6. Implementation Order

1. Add `crates/cli` with argument parsing and `puzzlestudio check`
2. Reuse existing game entry resolution from `puzzle-lang`
3. Add human diagnostics and stable exit codes
4. Add `--json` diagnostics
5. Add `export-html` by sharing existing HTML export code
6. Add `preview`
7. Add `play`
8. Add `editor`
9. Add `export-editor`
10. Add `simulate --json`
11. Add `import-puzzlescript`
12. Add `test`
13. Add `format`
14. Add `new`

The first useful milestone is:

```bash
puzzlestudio check games/spec_2d.puzzle
puzzlestudio check games/spec_2d.puzzle --json
puzzlestudio export-html games/spec_2d.puzzle -o /tmp/game.html
```

## 7. Design Checks Before Implementation

Before implementing each command, check the owner of the behavior:

- Is this command only exposing an existing capability?
- If it needs new behavior, which crate should own that behavior?
- Would HTML editor, Tauri, WASM editor, and CLI agree on the result?
- Does the command need a JSON contract for AI / CI?
- Does it have deterministic exit codes?

Do not make the CLI a parallel implementation of parser, runtime, preview, or project loading semantics.

## 8. Open Questions

- Resolved: the workspace package and installed binary are both named `puzzlestudio`.
- Should `preview` open a browser automatically, or only print the URL?
- Should `check` treat warnings as failure behind `--deny-warnings`?
- Should formatter work from original tokens to preserve comments, or initially print canonical source only?
- Where should gameplay regression examples live: inside `.puzzle`, adjacent `.puzzle.test`, or a project-level manifest?
- Should `simulate` expose full board snapshots by default, only summaries, or both behind flags?
