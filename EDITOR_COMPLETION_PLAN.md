# Editor Completion Plan

この文書は、`.puzzle` editor に予測提示 / 補完を入れる計画をまとめる。

目的は、汎用 AI 的に推測することではなく、`.puzzle` の各文脈で「ここに書けるもの」を狭く提示することである。作者がいま書いている block / token / cursor 位置から候補範囲を決める。

## 1. Principle

補完は syntax owner に従う。

- `layers { ... }` は layer 名、object / schema / group selector を提示する
- `group { ... }` は selector alias と、右辺に使える object / schema / layer tag / group を提示する
- `scratch { ... }` は scratch 名と scratch type を提示する
- `keys { ... }` は key trigger と action 名を提示する
- rewrite cell は object / group / layer tag / scratch / `no` を提示する
- puzzle / scene / component block は、その block が所有する directive だけを提示する

やらないこと:

- `for level in levels` のような汎用 loop を level menu 専用補完にしない
- author が定義できる普通の名前を固定キーワード扱いしない
- parse 成功時だけ動く補完にしない
- UI 側だけに独立した `.puzzle` grammar table を増やさない

## 2. Existing Foundation

すでに highlighter 側で、補完に必要な土台ができつつある。

- ソースから object / display object を拾える
- layer tag / group / global state / scratch / variant を拾える
- `keys` block の key trigger と action 名を拾える
- section sugar と brace block の文脈をある程度判定できる
- Rust の `highlight_source_html` を WASM から呼べる

次は、同じ source scan を補完用の symbol collection と context detection に整理する。

## 3. Rust API

`puzzle-lang` に補完用 API を追加する。

```txt
suggest_source_completions(source, cursor_offset) -> CompletionList
```

WASM では JSON 文字列として返す。

```txt
crates/wasm/
  suggest_source_completions(source, cursor_offset) -> String
```

候補の最小 schema:

```txt
CompletionItem {
  label: String,
  kind: "keyword" | "object" | "group" | "layer" | "state" | "scratch" | "variant" | "input" | "effect",
  insert_text: String,
  detail: String,
}

CompletionList {
  replace_start: usize,
  replace_end: usize,
  items: Vec<CompletionItem>,
}
```

`replace_start` / `replace_end` を Rust 側で返すことで、JS 側は text replacement に集中できる。

## 4. Contexts To Support First

最初に攻める文脈は、候補範囲が明確で効果が大きいところに絞る。

### Puzzle Block

`puzzle <name> { ... }` 直下:

- `layers {`
- `group {`
- `scratch {`
- `legend {`
- `win_conditions {`
- `lose_conditions {`
- `transitions {`
- `levels {`
- `on_level_start {`
- `on_level_clear {`
- `on_display {`
- `global <name> = false`
- `persistent <name> = 0`

### Layers

`layers { ... }`

左辺:

- 新しい layer tag 名なので、基本は候補を強く出さない
- 既存 layer tag の再利用候補は出してよい

右辺:

- object / schema
- display object
- group
- `each <schema>:<axis>`

### Group

`group { ... }`

左辺:

- 新しい group 名なので、既存名との重複警告に近い補助

右辺:

- object / schema
- layer tag
- group
- variant selector examples such as `Box:red`

### Scratch

`scratch { ... }`

候補:

- `<name>`
- `<name>:int`
- `<name>:<value_set>`
- `<name>:<object_axis>`

rewrite cell 内では:

- `{scratch_name}`
- `{scratch_name:<variant>}`
- `{no scratch_name}`

### Keys

`keys { ... }`

左辺:

- `Escape`
- `Enter`
- `Space`
- `ArrowUp`
- `ArrowDown`
- `ArrowLeft`
- `ArrowRight`
- single-letter keys

右辺:

- scene action names from `transitions`
- scene effects such as `back`, `goto <scene>`, `<scene>.next_level`
- existing action names found in the same scene

### Rewrite / Transition Statements

Inside `main`, `transitions`, `rule`, lifecycle blocks:

- application keywords: `once`, `once_all`, `once_per_level`, `repeat`
- control words: `for`, `if`, `else`
- inputs: `input`, `up`, `down`, `left`, `right`, aliases
- pattern helpers: `no`
- object / group / layer tag / scratch
- effects: `set`, `cancel`, `next_level`, `play_sfx`, `play_music`, `pause_music`, `resume_music`, `stop_music`

## 5. UI Integration

Editor UI は自前 textarea / overlay なので、最初は小さい popup を実装する。

Trigger:

- `Ctrl+Space` / `Cmd+Space`
- identifier 入力中の軽い debounce
- `{`, `:`, space after `=`, newline after block opener

Controls:

- ArrowUp / ArrowDown: 選択
- Enter / Tab: 挿入
- Escape: 閉じる
- click: 挿入

表示:

- label
- kind icon or short tag
- detail

最初は候補説明を短くし、documentation panel は作らない。

## 6. Implementation Phases

### Phase 1: Rust Core

- highlighter の source symbol scan を補完でも使える形に分離する
- cursor offset から current token と surrounding block context を取る
- `CompletionList` / `CompletionItem` を追加する
- unit test を context ごとに追加する

### Phase 2: WASM Boundary

- `crates/wasm` に `suggest_source_completions` を公開する
- JSON serialization を最小依存で実装する
- generated `wasm/puzzle_wasm.js` と standalone editor を更新する

### Phase 3: Editor Popup

- `editor.js` に completion state を追加する
- source editor の cursor offset を Rust API に渡す
- popup positioning を textarea overlay と合わせる
- keyboard / mouse 操作を入れる

### Phase 4: Context Coverage

- puzzle block / layers / group / scratch / keys を先に完了
- rewrite cell と transition effect を次に追加
- scene / menu / component block は最後に広げる

### Phase 5: Diagnostics Link

補完と error / warning を近づける。

- unknown selector の位置で既存 selector を候補に出す
- duplicate layer / group / scratch は候補より diagnostic に寄せる
- parse error 中でも、直前文脈から候補を出す

## 7. Test Strategy

Rust tests:

- `layers` 右辺で object / group が出る
- `group` 右辺で layer tag / object が出る
- `scratch` block で value set type が出る
- rewrite cell `{` 後に scratch が出る
- `keys` 左辺で `Escape Enter Space` が出る
- `keys` 右辺で action/effect が出る
- parse 失敗中でも候補が返る

Browser checks:

- popup appears near caret
- Enter / Tab inserts text
- Escape closes popup
- source highlight and completion do not fight each other
- generated standalone editor works through WASM on a static HTTP server

## 8. Near-Term First Slice

最初の実装 slice はこれにする。

```txt
source + cursor
  -> current token range
  -> block context
  -> symbol table
  -> JSON completion list
  -> Ctrl+Space popup
```

対象文脈:

- `layers` right-hand side
- `group` right-hand side
- `scratch` block rows
- `keys` block left/right side

これで、補完の価値と UI 操作の手触りを確認できる。rewrite cell 補完は便利だが文脈判定が少し難しいため、第二段に回す。
