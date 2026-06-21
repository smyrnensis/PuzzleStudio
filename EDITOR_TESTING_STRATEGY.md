# Editor Testing Strategy

この文書は、HTML editor 周辺をリファクタリングする前に、どの種類の
テストをどこへ置くかを整理するための開発者向けメモである。

目的は、ブラウザで何でも確認することではない。editor service、browser
UI、preview runtime の境界を分け、壊れやすい部分だけを実ブラウザで押さえ
ることである。

## 基本方針

テストは、挙動の所有者に置く。

- parser、validation、lowering の意味論は `puzzle-lang` に置く。
- deterministic state transition は core/runtime owner に置く。
- undo、restart、level advance、screen flow は play/session owner に置く。
- workspace、save、highlight、preview compilation は `html-editor` の
  Rust service test に置く。
- DOM、iframe、keyboard、pointer、focus、resize、WASM loading、
  `postMessage` は browser test に置く。

editor のリファクタリングで最初に守るべき contract は、「editor が runtime
fixture や renderer 内部 schema を再解釈せず、公開された preview/control
surface だけを使うこと」である。内部 JSON の形ではなく、editor 操作が公開
contract を通って runtime へ届くことを確認する。

## 判断表

| 確認したいこと | 置き場所 | 理由 |
| --- | --- | --- |
| workspace root の外へ保存できない | `cargo test -p html-editor` | service contract であり、DOM は不要 |
| generated/build/dependency files が editable documents に入らない | `cargo test -p html-editor` | workspace loading の contract |
| preview HTML が Pages 用 WASM loader を参照する | `cargo test -p html-editor` | HTML generation の contract |
| source edit 後に `/api/preview` が request source を使う | `cargo test -p html-editor` | browser event ではなく preview service の contract |
| Run Preview button が実際に iframe preview を起動する | browser test | DOM click、iframe、WASM loader が絡む |
| level editor の playtest が key input で盤面表示を変える | browser test | focus、keyboard、runtime bridge が絡む |
| 3D level preview が update message を runtime frame へ送る | browser test | `postMessage` と iframe/window boundary が絡む |
| 3D preview の payload shape が public contract を満たす | service/unit test + browser smoke | shape は unit、実配送は browser |
| layout resize 後に panes や editor wrap が破綻しない | browser visual/screenshot test | ResizeObserver と CSS layout が絡む |

迷った場合は、先に非ブラウザ test にできないかを見る。browser test は実行
コストと失敗要因が大きいため、少数の user-facing flow に絞る。

## Refactor 前の最小セット

大きな editor リファクタリングの前には、まず次の順で足場を作る。

1. `html-editor` service test を足す。

   既存の `crates/html_editor/src/lib.rs` の `#[cfg(test)]` module に、変更対象
   の contract を追加する。たとえば workspace path、preview request、save、
   highlight、asset loading、3D preview contract など。

2. editor 初期表示の browser smoke を足す。

   HTTP で editor を起動し、`/` または `/editor.html` を開いて、source pane、
   preview frame、主要 tool pane button が見えることを確認する。

3. 変更対象の browser flow を 1 本だけ足す。

   たとえば level editor refactor なら、Level pane を開く、playtest を開始
   する、key input を送る、source が勝手に commit されない、という流れだけ
   を確認する。3D preview refactor なら、preview update が public
   `postMessage` contract 経由で届くことを確認する。

4. refactor 後に必要なら browser flow を 1 本増やす。

   browser test は網羅率より、境界の代表例を守るために使う。細かい分岐は
   service/unit test へ戻す。

## Service Test の書き方

既存の `html-editor` test は一時 workspace を作り、fixture `.puzzle` を書き、
`EditorService` を直接開く形が中心である。この形を優先する。

典型的な対象:

- `EditorService::open`
- `EditorService::open_game_entry`
- `EditorService::source_json`
- `EditorService::compile_preview`
- `EditorService::highlight_json`
- `EditorService::save_source_file`
- request path normalization and rejection
- generated file exclusion

service test 実行:

```bash
cargo test -p html-editor --lib
```

service test で見るべきなのは、公開 API の入出力である。static JS の文字列
検査は、DOM test を持たない現状では有用だが、複雑な runtime behavior の
代替にしすぎない。

## Browser Test の書き方

browser test は、必ず HTTP で served editor を開く。`file://` は editor の
supported release surface ではない。

起動例:

```bash
cargo run -p html-editor -- games/spec_2d.puzzle --serve --port 8787
```

このリポジトリでは、追加 npm 依存を持たない最小の Chrome DevTools
Protocol smoke を `tools/editor_browser_smoke.mjs` に置く。これは見た目の
良し悪しではなく、HTTP editor、DOM click、iframe runtime、keyboard event、
3D `postMessage` contract が実ブラウザでつながっていることだけを見る。

実行:

```bash
cargo test -p html-editor --test browser_smoke
```

直接実行する場合:

```bash
cargo build -p html-editor
node tools/editor_browser_smoke.mjs --editor-bin target/debug/html-editor
```

Chrome/Chromium が標準位置にない場合は `PUZZLESTUDIO_CHROME` か `--chrome`
で実行ファイルを指定する。

Playwright などを追加する場合の最小 shape:

```ts
import { test, expect } from "@playwright/test";

test("editor loads and can run preview", async ({ page }) => {
  await page.goto("http://127.0.0.1:8787/");

  await expect(page.locator("#previewFrame")).toBeVisible();
  await page.getByRole("button", { name: "Run preview" }).click();

  const frame = page.frameLocator("#previewFrame");
  await expect(frame.locator("body")).toBeVisible();
});
```

selector は、ユーザーが操作する安定した surface を優先する。

- role/name が安定している button は `getByRole` を使う。
- editor 内で既に contract 的に使われている要素は `#previewFrame`、
  `#levelPlaytestButton`、`#levelBoard`、`#level3dPlaytestButton` などの ID
  を使ってよい。
- private helper 関数名や renderer 内部 object の shape に依存しない。

## Browser Test の対象を絞る

browser test に向いている flow:

- editor first load
- Run Preview
- source edit -> preview refresh
- level editor open -> playtest start/stop
- keyboard input during playtest
- 3D level preview update through `postMessage`
- pane resize or visibility toggle with a screenshot/visual assertion

browser test にしない方がよいもの:

- parser error details
- lowering result details
- workspace path rejection
- generated file exclusion
- save request validation
- HTML string assembly details
- every small UI branch

これらは Rust unit test や crate owner の focused test に置く。

## Refactor 時の使い方

リファクタリング前に、変更対象を次の一文へ落とす。

```txt
この変更で守りたい public behavior は何か。
```

その答えが service 入出力なら `cargo test -p html-editor` に足す。答えが
browser event、iframe、WASM、layout、canvas、`postMessage` なら browser test
に足す。

リファクタリング中は、次の順で回す。

```bash
cargo test -p html-editor
cargo run -p html-editor -- games/spec_2d.puzzle --serve
```

3D editor/preview を触るときは `games/spec_3d.puzzle` でも served editor を
開く。

```bash
cargo run -p html-editor -- games/spec_3d.puzzle --serve
```

最後に、変更が generated Pages output へ反映される必要があるときだけ、
owner-specific instructions に従って WASM build と `tools/generate_web_editor.sh`
を実行する。通常の refactor verification では `docs/` の generated files を
直接編集しない。
