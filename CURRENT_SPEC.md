# Current Spec

この文書は、現時点の実装が採用している `.puzzle` 仕様をまとめる。

## Architecture

```txt
.puzzle source
  -> puzzle-lang parser / compiler
  -> puzzle-core CompiledGame
  -> puzzle-play session / render helpers
  -> ascii-play / html-play
```

`puzzle-core` は `.puzzle` 文法、表示、ファイル IO、level 管理を知らない。

`puzzle-lang` が authoring syntax を読み、`CompiledGame` と level / legend / controls などの metadata を作る。

Top-level metadata は `title <text>` と省略可能な `subtitle <text>` / `author <text>` / `homepage <text>` で書く。`name <text>` は top-level metadata として読まない。scene の `title` / `subtitle` component は、引数を省略すると top-level metadata を表示する。旧 surface の `game.title` / `game.subtitle` / `game.author` / `game.homepage` は互換として残すが、canonical example では使わない。

## Game Entry And Imports

ゲーム folder は package の単位で、実行 entry は top-level `title` などの game prelude metadata を持つ `.puzzle` とする。

```txt
games/fixban/
  game.puzzle
  levels.puzzle
  sprites.puzzle
```

adapter / editor / build tool に folder を渡した場合は、その folder 内の prelude-bearing `.puzzle` に解決する。複数ある場合は `game.puzzle`、`<folder>.puzzle`、`main.puzzle`、その他の順で優先する。prelude-bearing `.puzzle` がない folder は error。`levels.puzzle` や `sprites.puzzle` のような prelude を持たない分割 file は import fragment であり、それ自体は実行対象ではない。

`.puzzle` file を直接渡す場合、その file 自体が prelude を持てば実行対象になる。prelude を持たない fragment を渡した場合は、同じ folder から親 folder へ向かって最初の game entry を探す。

`import "<path>"` は source composition である。同じ folder にある `.puzzle` は自動では読まれず、entry から明示的に import する。

## Execution Model

`rules` は必須の puzzle gameplay entrypoint。旧 `transitions` / `main` block は読まない。表示用の派生処理は `routine @name` で宣言し、`@name` statement を置いた位置で実行する。

```txt
rules {
push
@refresh_board
move
}

routine @refresh_board once {
repeat [ @Light ] -> []
[ Player no @Light ] -> [ Player @Light ]
}
```

`routine` は名前付き statement list。定義しただけでは実行されない。旧 `rule` declaration は読まない。

```txt
routine movement {
push
move
}
```

routine の application はデフォルトで `repeat`。

`routine @name` は visual routine を定義する。display routine は main object と、その call site の transition input を読めるが、write できるのは `@Name` display object だけ。effect は使えない。solver は display routine を実行せず、display object を state key から外す。

短い表示派生は routine 宣言なしで `display [ ... ] -> ...` と書ける。複数行なら `rules` や `on_level_start` の中に statement block として `display { ... }` を置ける。どちらも置いた位置で実行される。

main rules と gameplay condition は display object を読めない。display object に依存する見た目は `@routine` / `display <routine>` に置き、gameplay / solver の因果には入れない。bare main routine call は同じ role の routine だけを呼べるため、`rules { refresh_board }` のような暗黙 display call は使えない。

`on_level_start` / `on_level_clear` でも `@routine` / `display <routine>` を書ける。ただし lifecycle block は通常入力ではないため、そこで呼ばれた display routine は `input` orientation / `if input == ...` を使えない。

`on_display { ... }` は renderer / editor が表示 snapshot を作る直前に走る display-only hook。中には `@routine`、`display <routine>`、`display <rewrite>`、`display { ... }` などの display statement だけを書ける。`on_display` は gameplay state、solver、win condition、undo key を変えるための hook ではなく、editor でセルを直接編集した状態にも同じ visual derivation をかけるための projection である。

main object と display object は layer namespace / layer order を共有する。`@Name` は display object を表し、`layers` block の右辺で main object と同じ順序の中に宣言できる。`display_objects` block は読まない。

`layers { each A:tag_set }` は selector alternatives を別々の通常 layer に展開する。これは collisionless layer ではなく、各 variant が表示順つきの通常 collision layer を得るための短縮文法。

`layers` 内の `for <binding> in <value_set> { ... }` は layer row の parse 前に展開される。`for k in kind { k = A:k B:k }` は、`A:kind` / `B:kind` が宣言済みなら `red = A:red B:red` のような名前付き layer 行へ展開できる。

```txt
routine slide {
input directions [ Player | ] -> [ | Player ]
}
```

`routine <name> repeat` は routine block 全体の application。block 内の statement sequence を、block 全体が変化しなくなるまで繰り返す。

rewrite 行も application を持つ。plain rewrite のデフォルトも `repeat`。行ごとに `once` / `once_all` / `once_per_level` / `repeat` を明示できる。1セルだけ動かすような rule は、block と rewrite 行の両方に `once` を明示する。

`once_all` は、適用開始時点の全マッチを row-major order で集め、それぞれを最大1回ずつ適用する。各マッチは開始 state に対する write proposal を出し、同じ slot に複数 proposal が来た場合は row-major 後続マッチの proposal が勝つ。途中で作られた新しいマッチは同じ `once_all` では拾わない。

`once_per_level` は、その concrete rule が現在の level state 内でまだ発火していない場合だけ、最初の1マッチに適用する。restart / next level で初期 state に戻ると発火済み記録もリセットされる。

```txt
routine move once {
once input directions [ Player | ] -> [ | Player ]
}
```

## Statements

現在の statement:

```txt
fix block
routine call
inline rewrite
effect statement
repeat block
repeat until block
for block
if block
```

Control word の境界:

- `if` は condition guard。routine statement list では block を guard し、scene `rules` では condition transition を表す。
- routine statement list の bare puzzle var condition は `var != 0` として読む。`else` block は negated guard として lowering される。
- lifecycle hook は `on_level_start { ... }` / `on_level_clear { ... }`。puzzle lifecycle point なので scene transition arrow にはしない。
- `on_level_start` is runtime lifecycle, not parser materialization: raw `Level.initial_state` remains the parsed map, and `puzzle-play` / standalone HTML apply the hook on level entry, restart, and level navigation. Rule emissions such as `message` and `sfx` are collected at that runtime point.
- level body can add level-local lifecycle behavior. In `level { ... }`, `on_level_start { ... }` / `on_level_clear { ... }` attach statement lists to that level only. As sugar, `message` / `sfx` / `wait` before the first ASCII map row become level-local `on_level_start`, and the same commands after the map become level-local `on_level_clear`.
- component behavior は component が入力の意味を所有する。`level_menu` は cursor 移動と enter を所有するため、author は `cursor.*` や `emit` を書かない。

例:

```txt
rules {
once input directions [ Player | Box | ] -> [ | Player | Box ]
once input directions [ Player | ] -> [ | Player ]
}
```

これは anonymous rules を順番に実行する。

anonymous inline rewrite は application prefix を持てる。

```txt
rules {
once input directions [ Player | ] -> [ | Player ]
repeat input directions [ Fire | Wood ] -> [ Fire | Fire ]
}
```

`once` / `repeat` は statement block としても書ける。

```txt
rules {
repeat {
input directions [ Fire | Wood ] -> [ Fire | Fire ]
input directions [ Fire | Grass ] -> [ Fire | Fire ]
}
}
```

`repeat until <condition> { ... }` は condition が false の間だけ block を繰り返す pre-check loop。condition が最初から true なら body は0回。condition は `if` と同じ var / named condition / query / pattern condition を使える。body が発火せず、condition も false のままなら transition error になる。

```txt
rules {
repeat until no down [ Rock | ] {
once_all down [ Rock | ] -> [ | Rock ]
}
}
```

`repeat` / until-stable は、変化がなくなったら stable として終了する。同一 state が再出現した場合は cycle として検出し、repeat 境界の開始状態へは巻き戻さず、再訪した現在 state のまま repeat を終了する。cycle せず発散する repeat は 200 回の内部上限で打ち切り、その時点の state で後続 statement へ進む。`cancel` はこの cycle / 上限処理より優先する。

block application は source block の境界にかかる。rewrite 行の application は source statement の境界にかかる。group や schema selector が concrete rewrite variants に展開される場合、それらは同じ repeat 境界内の alternatives として扱う。

Rule effect は statement 位置に直接書ける。pattern match は持たず、scene effect ではなく puzzle rule effect だけを受ける。旧 `do <effect>` は canonical ではなく、parse error になる。

```txt
rules {
sfx tick
message "Ready"
set moved = false
}
```

`effect <name> { ... }` は puzzle rule effect の名前付き macro。body は `cancel`、`win`、`restart`、`next_level`、`again`、`sfx <name>`、`message <text>`、var update、または別の名前付き effect を 1 行ずつ持つ。呼び出しは effect 名を statement として直接書くか、`if <condition> -> <name>`、rewrite suffix の `<name>`。

`fix <tokens> { ... }` は囲んだ rewrite statement のデフォルト application / orientation を固定する。`fix <tokens> ... end` は互換 syntax として読む。

```txt
fix once {
input directions [ Player | Box | ] -> [ | Player | Box ]
input directions [ Player | ] -> [ | Player ]
}
```

展開後:

```txt
once input directions [ Player | Box | ] -> [ | Player | Box ]
once input directions [ Player | ] -> [ | Player ]
```

application と orientation は同時に固定できる。

```txt
fix once left {
[ Player | ] -> [ | Player ]
}
```

展開後:

```txt
once left [ Player | ] -> [ | Player ]
```

orientation だけも固定できる。

```txt
fix right {
[ A | ] -> [ | A ]
}
```

概念的には:

```txt
right [ A | ] -> [ | A ]
```

明示 prefix は `fix` より優先する。`fix once` の中でも `repeat [ ... ] -> [ ... ]` は repeat のまま。`fix` は top-level directive を生成する authoring macro ではない。

`if` は `input == <input-or-binding>` と puzzle state 変数の比較を lower できる。

```txt
var button_is_pushed = false

rules {
[ Switch ] -> [ SwitchOn ] set button_is_pushed = true
[ Button Box ] -> set button_is_pushed = true
[ Button Box ] -> count += 1
if button_is_pushed == true {
once [ A ] -> [ APrime ]
once [ B ] -> [ BPrime ]
} else {
once [ A ] -> [ A ]
}
}
```

## Rewrite Orientation

方向を明示する rewrite は orientation prefix を持つ。

```txt
directions [ Player | ] -> [ | Player ]
horizontal [ Player | ] -> [ | Player ]
input horizontal [ Player | ] -> [ | Player ]
right [ Player | ] -> [ | Player ]
d     [ Player | ] -> [ | Player ]
```

内部では次の式として読む。

```rust
OrientationExpr::Input
OrientationExpr::InputSet(...)
OrientationExpr::Fixed(...)
OrientationExpr::Binding(...)
OrientationExpr::Neutral
```

`directions` / `horizontal` / `vertical` は orientation set prefix として使える。runtime で式評価するのではなく、lowering でそれぞれの方向 variants に展開する。`horizontal [ ... ]` は `left [ ... ]` と `right [ ... ]`、`directions [ ... ]` は `up` / `down` / `left` / `right` の rewrite として読む。

`input horizontal [ ... ]` / `input directions [ ... ]` は input guard 付きの orientation set。概念的には `for d in horizontal { if input == d { d [ ... ] -> ... } }` と同じ。

prefix なしの単独セル pattern は neutral として扱い、offset を方向回転しない。

prefix なしの空間 pattern、つまり複数セル、複数行、ellipsis、または相対方向属性を含む pattern は、PuzzleScript 互換の cardinal direction pattern として `up` / `down` / `left` / `right` に lower する。

この規則は rewrite だけでなく、pattern condition と query pattern にも適用される。

```txt
[ A | ] -> [ | A ]
some([ Player | Wall ])
some(down [ Rock | ])
some(horizontal [ Rock | ])
some(input horizontal [ Rock | ])
count([ Button | Box ])
count(down [ Rock | ])
count(directions [ Rock | ])
```

上のような prefix なし pattern は4方向 variant を作る。
orientation set prefix の pattern は、その set に含まれる方向 variants へ展開する。`horizontal [ ... ]` は left/right、`vertical [ ... ]` は up/down だけを見る。`input horizontal [ ... ]` は、現在の transition input が horizontal の member のときだけ、その input に対応する orientation の pattern として評価する。

## Value Expansion

`for` は statement list を展開する。

```txt
rules {
for d in directions {
if input == d {
d [ A | ] -> [ | A ]
d [ B | ] -> [ | B ]
}
}
}
```

概念的には、各方向について block 内の statement が順番ごと複製される。

```txt
left  [ A | ] -> [ | A ]
left  [ B | ] -> [ | B ]
right [ A | ] -> [ | A ]
right [ B | ] -> [ | B ]
...
```

現在の主な軸:

```txt
directions
horizontal
vertical
layers
```

`directions` は組み込み tag set であり、常に `up` / `down` / `left` / `right` の4値を表す。`horizontal` は `left` / `right`、`vertical` は `up` / `down`。object schema、`map`、visual table、`for` で同じ集合として使える。

`layers` は layer 定義から作られる tag set。展開値は layer group 名で、名前付き layer はその名前、匿名 layer は内部名を使う。標準 `move` rule はユーザーが同名 rule を定義していない場合に用意され、概念的には次の rule と同じ。

```txt
rule move repeat {
for d in directions {
for l in layers {
d [ d l | no l ] -> [ | l ]
}
}
}
```

## Input

`input` は canonical state ではない。物理 key を読み替えた semantic input として transition context に渡される。

```txt
transition(state, input) -> state
```

`if input == right` と `if input in directions` は transition context の input 名を参照する。

方向 set に対して同じ rewrite を書く場合は、orientation set prefix を使える。

```txt
directions [ Player | ] -> [ | Player ]
horizontal [ Player | ] -> [ | Player ]
input horizontal [ Player | ] -> [ | Player ]
```

authoring では、puzzle transition に渡る意味入力と、物理 key binding を分ける。

`up` / `down` / `left` / `right` は標準の意味入力で、direction mapping も既定で用意される。`restart` も標準の非方向 input で、既定では `r` key からこの input に対応する。model rule に `restart` input の明示 handler がなければ、`restart -> restart` が暗黙に追加される。`if input == restart` のような名前 guard は非方向 input でも意味を持つ。別名が必要な場合は `direction` で標準方向への alias を定義する。

```txt
direction east right
direction west left
direction north up
direction south down
```

物理キーは owner-scoped な `inputs` block で semantic input 名に対応させる。model 内の `inputs` は puzzle/model rules へ渡す input、scene 内の `inputs` は scene-wide shortcut や title/menu confirm など scene rules が読む input を定義する。

```txt
model puzzle sokoban {
inputs {
up <- w ArrowUp
down <- s ArrowDown
left <- a ArrowLeft
right <- d ArrowRight
restart <- r
}
rules {
restart -> restart
}
}
```

```txt
scene title {
inputs {
confirm <- Enter Space x
}
button "Play" -> input confirm
rules {
input confirm -> start levels in playing
}
}
```

`<input> <- <key...>` は、複数の physical key を同じ semantic input に lower する。通常文字に加えて `ArrowUp` / `ArrowDown` / `ArrowLeft` / `ArrowRight` / `Enter` / `Space` / `Escape` / `Tab` / `Backspace` を named key token として書ける。`my_restart <- r` のように model input で書くと、既定の `restart <- r` は shadow される。`button "Play" -> input confirm` は button click を同じ semantic input 経路へ送る。model `rules` の `<input> -> <effect>` と scene `rules` の `input <input> -> <command>` は `if input == <input> { ... }` の sugar。scene / presentation / lifecycle command は `effect` wrapper を付けずに直接書く。scene が level lifecycle に介入する場合は `playing.restart` や `board.restart` のように target を明示する。

入力適用後の turn completion では、runtime が post-rules / pre-navigation の snapshot に対して `win_conditions` を評価する。`win_conditions` が true なら model lifecycle として `on_level_clear` を level navigation より前に実行する。通常の clear / advance / restart は model window component と puzzle lifecycle が所有し、scene condition transitions は overlay、menu、hub、特殊分岐などの例外的な flow 介入だけを担う。これは puzzle-core の rewrite ではなく、`GameSession` / standalone HTML runtime が扱う flow である。

`again` command も turn completion で解決される。`again` は入力 event の再送ではなく、同じ puzzle target の rule entrypoint を `InputId(0)` / no semantic input で再実行する follow-up turn request である。follow-up turn は現在の turn が commit され、message / sfx / wait / navigation command の収集が終わった後に予約される。follow-up turn 内で `again` が再び出ると次の no-input turn が予約される。runtime は 1 input から派生する automatic turn を最大 256 回に制限する。

## Scenes

`scene` は puzzle transition の外側にある game-flow metadata。`screen <name>` は読まない。

scene は local state を持てる。`view` block は scene-local state slot と表示 component をまとめて定義する。

scene は 2D / 3D model の所有者ではなく、presentation と flow の所有者である。root layout、component tree、scene input、scene transition は model の次元数に依存しない。同じ scene 構文の中で、model window component だけが `puzzle <slot>` または `puzzle3 <slot>` として model-specific になる。

`view` は component ではなく scene root layout block。`view size 720 540 { ... }` は、2D board でも 3D board でも同じ意味で scene の標準表示領域を指定する。`row` / `column` / `box` は generic layout component で、`size <w> <h>`、`gap <n>`、`align <x> [y]` の header attribute を `view` と同じ形で読める。

Canonical generic scene component keywords:

```txt
title
subtitle
text
button
row
column
box
for
level_menu
menu
```

Model window component keywords:

```txt
puzzle   // 2D puzzle model window
puzzle3  // 3D puzzle model window
```

`panel` は component keyword ではない。styled panel が必要な場合も、文法上は `box` に theme / adapter 側の style を対応させる。

2D model window example:

```txt
scene playing {
state {
puzzle sokoban
}
view size 720 540 {
column gap 12 align center top {
sokoban
row gap 8 {
button "Restart" -> sokoban.restart
button "Levels" -> goto level_select
}
}
}
rules {
step sokoban
}
}
```

3D model window example using the same scene/layout shape:

```txt
scene playing3d {
state {
board = puzzle3 push3d
}
view size 720 540 {
column gap 12 align center top {
puzzle3 board
row gap 8 {
button "Restart" -> board.restart
button "Levels" -> goto level_select
}
}
}
}
```

The examples differ only at the model slot initializer and model window component. The scene root size, layout nesting, buttons, and scene commands are shared scene concepts.

3D model の renderer 初期値は model 内の `render` が所有する。camera は scene layout や rule state ではないため、canonical syntax では model top scope の個別設定ではなく `render { camera { ... } }` に包む。

```txt
model puzzle3 push3d {
render {
camera {
yaw 34
pitch 38
zoom 1.1
interactive_look true
interactive_zoom true
}
grid {
occupied_cells true
}
shade true
}
}
```

`yaw` / `pitch` / `zoom` は初期 camera view、`interactive_look` は pointer drag による yaw/pitch 変更、`interactive_zoom` は wheel/pinch 系の zoom 変更を許す設定である。旧 `debug_camera` / `camera_yaw` / `camera_pitch` / `camera_zoom` は compatibility syntax で、新しい例では使わない。

`grid { occupied_cells true }` は object が存在する cell の外周 edge を表示する preview/debug 用の読み取り補助である。これは floor や volume を追加するものではなく、puzzle state、collision、win condition、level data には影響しない。省略時は off。

`render { shade false }` は sprite voxel の面ごとの明暗付けを無効にする renderer 設定である。色の表示だけを揃えたい preview 用であり、puzzle state、sprite voxel data、collision、win condition には影響しない。省略時は既存どおり on。

`interactive_look` は semantic input ではない。親 scene は click/drag を 3D camera 用として特別扱いせず、raw input を通常どおり focused scene と layout/hit-test に従って component へ配信する。`puzzle3` component は、自分の表示 box 内で始まった pointer drag を取得してよい。`interactive_look true` のときだけ、その gesture を camera yaw/pitch の view-state 更新として解釈する。これは model `rules` の `input` には渡らず、`if input == ...`、undo、restart、transition state、win condition には影響しない。

pointer drag の所有者は開始点で決まる。pointer down が `puzzle3` の box 内なら、release/cancel まではその component が gesture を capture してよく、途中で pointer が box 外へ出ても同じ drag として継続する。例外は modal、disabled component、overlay、明示的な pointer capture、scene-level gesture など、より具体的な所有者がある場合だけである。

`scene puzzle [name]` は puzzle state を主モデルに持つ playable scene。`name` 省略時は `playing`。中の `layers` は board/object layer、`view` は画面配置を表す。scene-local な puzzle slot を明示しない場合は、`<name>` slot が暗黙に `puzzle <name>` として用意される。`board` は予約 slot 名ではない。`input <name...> { update <slot> }` は各 semantic input をその puzzle slot の transition に適用する scene transition へ lowering する。`if win_conditions { ... }` のような unqualified condition は primary puzzle slot の `<slot>.win_conditions` として解決できるが、通常の level progression には使わない。scene transition の `<slot>.<name>` は named condition を先に見て、存在しなければ `<slot>` の var `<name>` を truthy 判定する。

`scene level_menu [name]` は level 選択専用 scene。`name` 省略時は `level_select`。直下に `show_index = <true|false>`、`show_solved = <true|false>`、`layout = list`、`columns = <n>`、`wrap = <true|false>`、`locked = disabled|hidden`、`button ...` などの `level_menu` option を書ける。matrix では `left` / `right` が隣の item、`up` / `down` が列数ぶん前後の item に移動する。

`scene title_menu [name]` はタイトル用 menu scene。`name` 省略時は `title`。直下に `title` / `subtitle` / `text` / `button` / layout component を書ける。

`sounds { ... }` は top-level の音源定義。`sfx <name> seed=<seed> type=<type>` と `music <name> seed=<seed> tone=<0..1> bpm=<60..160> volume=<0..1>` を持つ。`sfx type=puzzlescript` は PuzzleScript numeric sound seed 互換 generator を選ぶ import 用 type。scene/component RHS の canonical form は `input <name>`、`component_effect <name>`、または direct scene command。scene command は `sfx <name>`、`play_music <name>`、`pause_music [name]`、`resume_music [name]`、`stop_music [name]`、`goto <scene>`、`enter <scene>`、`start levels [scope] in <scene>`、`continue levels [scope] in <scene>`、`back`、`<target>.restart` などを書ける。`start levels in playing` は target scene が受け入れる level 集合のうち、先に書かれた level で level scene を開始する。`continue levels in playing` は保存復元や level menu で選ばれた level が target scene に入れるならそこから再開し、なければ `start levels in playing` と同じ先頭 level を使う。`start levels microban in playing` / `continue levels microban in playing` はその scope/prefix に属する level に絞る。通常の clear / advance / restart は model window component と puzzle lifecycle の責務なので、scene command は明示的な介入に限る。`play_sfx <name>` は読まない。`message <expr>` は popup message を出す presentation effect で、quoted text、scene `var`、top-level `var`、effect binding を参照できる。`wait [duration]` は `wait 0.1s` / `wait 1s` / `wait 100ms` のように書く scene presentation wait で、`wait` 単体は既定で `0.2s`。top-level `default_wait_time = 500ms` のように bare `wait` の既定値を変更できる。scene 直下の lifecycle block は `on_scene_start { ... }` のみ。`on_level_start { ... }` は puzzle lifecycle block であり、scene には置けない。複数 command は block に 1 行ずつ書き、`then` は使わない。音声、message、wait は presentation adapter の責務で、core rule state には入らない。

`theme <theme>` / `theme <theme> { ... }` は top-level の表示 theme metadata。theme の見た目の identity は HTML adapter の CSS preset が持ち、`.puzzle` の theme 宣言は preset 名の選択と、作者に公開する少数の調整項目だけを持つ。公開項目は `accent_color`、`background_color`、`text_color`、`muted_text_color`、`line_color`、`board_color`、`ui_font`、`title_font`、`control_radius`、`panel_radius`。これらは HTML adapter が `--accent` / `--bg` / `--board-bg` などの CSS custom property へ lower し、preset CSS の値を上書きする。theme は `puzzle-core` の state、rule、transition には入らない。複数 theme 宣言は import 後の順序で preset 名または同じ項目を上書きする。theme 未指定時の default theme name は `clean`。標準 preset は `themes/clean.puzzle`、`themes/terminal.puzzle`、`themes/paper.puzzle`、`themes/pixel.puzzle`、`themes/candy.puzzle`、`themes/blueprint.puzzle`、`themes/noir.puzzle` に置く。これらの `.puzzle` import は実質的に `theme <theme>` を分割 file にしたもので、HTML adapter は対応する CSS preset を同梱する。editor upload でもこの `themes/*.puzzle` import は built-in として解決される。

`assets { ... }` は top-level の外部 file manifest。`css "game.css"` と `script "visuals.js"` を持てる。path は game folder からの相対 path だけ。HTML adapter は宣言された CSS / script だけを読み込む。`script` は rendered scene snapshot から追加表示を作るための補助 JS で、puzzle state、transition、undo stack、level index を直接変更してはならない。盤面に追従する script は `window.PuzzleStudio.registerAssetScript({ setup(api) { api.onRender(...) } })` を使う。

```txt
scene play_level {
state {
puzzle sokoban
}
view {
message = "Push the box"
sokoban
box {
text message
button "Back" -> back
}
}
}
```

`puzzle sokoban` は scene-local puzzle state slot を model と同じ名前で定義する標準形。runtime snapshot は scalar state を `sceneState`、puzzle slot 名を `scenePuzzles` として出す。複数 instance が必要な場合だけ `sokoban1 = puzzle sokoban` のように明示名を付ける。

現在の標準 component:

```txt
sokoban
text message
button "Start" -> input start
box {
text message
}
row {
button "Back" -> back
}
column {
level_menu {
show_index = true
}
}
```

`text` は literal text、scene state の scalar value、または `for` binding の path を表示する。`button` は input、component effect、または scene command を発行する view component。旧 `button "Label" = name` や裸名 RHS は読まない。`-> input <name>`、`-> component_effect <name>`、または direct scene command を使う。`box` / `row` / `column` は入れ子の view tree を作る layout component。`box` は純粋な配置用の矩形で、背景・枠線・装飾をデフォルトでは持たない。`panel` は layout primitive ではなく、canonical syntax では使わない。`view` / `box` / `row` / `column` は共通の layout header attribute として `size <w> <h>`、`gap <n>`、`align <x> [y]` を読める。scene root の標準サイズ指定は `view size 720 540 { ... }`。`for` は scene state collection の各 item から view node を生成する projection primitive で、level list には使わない。

scene condition は current level context を読める。`level.name == <name>` / `level.name != <name>`、`level.label == <label>` / `level.label != <label>`、`level.last`、`level.has_next` をサポートする。level 固有の message / sounds / exception flow は effect 側ではなく condition 側で scoped にする。通常の level progression は scene condition の標準責務にしない。authoring での level 指定は `level.name` を標準にし、index / number 条件は標準 surface にしない。

```txt
scene level_select {
view {
message = "Select a level"
text message
level_menu {
show_index = true
show_solved = true
}
button "Back" -> back
}
}
```

`level_menu` は level 選択専用 component。component が cursor と enter を所有する。通常は key binding を書かなくてよい。既定では `w/a/s/d` と arrow keys が移動、Enter/Space が `enter`、Escape/q が `back` になる。

```txt
scene level_select {
view {
level_menu {
show_index = true
}
}
}
```

enter 時は選択 level を開始する。これは `level_menu` template の主動作なので、`action goto_level` や `choose_level` transition は書かない。旧 `show index`、`columns <n>`、裸の `wrap`、`action <name>` は読まない。`show` / `hide` / `toggle` は scene visibility effect として残す。

level の開始、読み込み、restart は level scene / puzzle slot に対する command として書ける。ただし通常の clear / advance / restart は level scene 内の model window component と puzzle lifecycle が所有する。scene からの target command は、title/menu から開始する、button で明示 restart する、hub から特定 level に飛ぶ、通常 clear とは別の例外 flow に入る、などの介入だけに使う。`start levels in playing` は playing scene の受け入れる level 集合のうち、先に書かれた level で開始する。`continue levels in playing` は保存復元や level menu で選ばれた level が playing scene に入れるならそこから再開し、なければ先頭 level で開始する。`start levels microban in playing` / `continue levels microban in playing` は scope/prefix を絞った level に絞る。`playing.restart` は playing scene の現在 level を初期状態に戻し、`playing.next_level` は playing scene を次 level で開始し、`playing.previous_level` は前 level で開始する。`playing.goto <level>` は指定 level で playing scene に移る。`board.restart` のように puzzle slot を target にした場合は、その puzzle state を初期状態に戻す。`board.next_level` はその puzzle を所有する level scene を進める。

puzzle rule でも `win`、`restart`、`next_level`、`again`、`message`、`sfx` を effect として出せる。`win` はその turn の `win_conditions` を true として扱う clear outcome command で、`set win_conditions = true` の sugar に近い。model `rules` の `<input> -> <effect>` と scene `rules` の `input <input> -> <command>` はどちらも `if input == <input> { ... }` の sugar。model rules では `restart -> restart` が semantic input `restart` を model restart effect に接続する rule になり、scene rules では `input level_select -> goto level_select` が scene input を flow command に接続する transition になる。model rules 内に `restart` input handler がない場合は、この default handler が暗黙に追加される。scene 側で restart / level navigation に介入したい場合は、`board.restart` や `playing.next_level` のような target command を明示する。これは通常進行の書き方ではなく、ユーザー操作や特殊 flow のための escape hatch である。`[ Goal Box ] -> next_level` と `if win_conditions -> next_level` は board transition の結果として、所有 component/runtime に level advance command を渡す。`again` は現在の turn を commit した後、runtime に no-input follow-up turn を要求する。`again` が再実行するのは直前の key / semantic input ではなく、同じ puzzle target の rule entrypoint である。したがって follow-up turn では `if input == <name>` は成立しない。自動 turn は最大 256 回で止まり、`cancel` が出た場合はその自動 turn だけを取り消して停止する。standalone HTML では follow-up turn を `defaultAgainMs`、現在は 120ms 間隔で実行し、各 turn の `sfx` / `message` emissions を別 snapshot として公開する。`[ Player Goal ] -> message "Found"` と `[ Player Box ] -> message hint` は popup message command を渡す。`[ Player | Box | ] -> [ | Player | Box ] sfx push` は rule が match したときに named SFX を再生する command を渡す。

level list は `level_menu` で表す。Generic `for` は scene state collection 用で、`for level in levels` は canonical syntax ではない。

3D prototype の旧 `keys { ... }` scene block は互換処理または移行対象であり、canonical scene syntax ではない。新しい例と移行先は owner-scoped `inputs { <input> <- <key...> }` に寄せる。2D と 3D の scene input は同じ shared scene contract で扱い、model-specific input interpretation は `puzzle` / `puzzle3` component または model runtime が所有する。

## Variables

`var` は置かれた owner に応じて変数を定義する。`const` は同じ owner に読み取り専用の初期値を定義する。top-level では session 値、`scene` 内では scene instance 値、`puzzle` 内では puzzle state に紐づく整数スロットになる。puzzle var / const の boolean は `true = 1`, `false = 0` として保持する。

```txt
var total_moves = 0
const target_moves = 12

scene playing {
var message = "Ready"
const title = "Level Select"
}

puzzle sokoban {
var button_is_pushed = false
const max_moves = 120
persistent var cleared = false
}
```

`persistent var` は scope を作らず、同じ owner の通常初期化をまたいで値を保持する modifier。`const` は rule guard や message / scene expression から読めるが、puzzle rewrite effect の `set` / `+=` などでは更新できず、scene param でも上書きされない。旧 `global <name> = ...` と `persistent <name> = ...` は読まない。

`if <var> == <true | false | number>` は rule guard に lower される。bare var は `var != 0` として読む。

rewrite 末尾の effect は、match が成立したときの patch effect として適用される。

```txt
count = 0
count += 1
count -= 1
count *= 2
count /= 2
count %= 10
set count = 0
```

`[ Pattern ] -> count += 1` は effect-only rewrite。右辺 pattern は左辺と同じものとして扱い、盤面は変更しない。演算は `i64` の checked arithmetic で、overflow と 0 除算は transition error になる。

`cancel` は effect-only または rewrite suffix として使える。

```txt
[ Player Trap ] -> cancel
[ Player Trap ] -> [ Player Trap Flash ] cancel
```

`cancel` が match した場合、その transition 全体は開始 state に戻って正常終了する。発火した rule は trace に残せるが、board / scratch / var の変更は残らない。

`scratch { ... }` で宣言した一時 fact は transition-local。`{mark}` は cell-anchored、`Box{mark}` は occurrence-anchored。どちらも rule chain 内では match / write できるが、transition / lifecycle block 終了時に自動消去され、solver key / level state / renderer には残らない。

`Box{mark}` と `Box {mark}` は別の anchor を指す。同じ scratch 名の anchor 変換や同じ cell pattern 内での同居は valid だが warning になる。`>` / `<` / `^` / `v` sugar は builtin occurrence scratch `__move` へ lower される。`parallel` / `perpendicular` は movement scratch の相対方向 set sugar で、oriented lowering 時にそれぞれ `<` / `>`、`^` / `v` alternatives へ展開される。

## Query

`query` は author が名前を付ける盤面由来の値。core はゲーム語彙を持たず、構造的な query primitive だけを持つ。

```txt
group {
cargo = Box Crate
}
query cargo_count = count(cargo)
query pressed_buttons = count([ Button Box ])
query any_cargo = exists(cargo)
query has_pressed_button = exists([ Button Box ])
```

現在の query primitive:

```txt
count(selector)
count(pattern)
exists(selector)
exists(pattern)
some(selector)
some(pattern)
```

`if <query> == <number>` と `if <query>` は rule guard に lower される。`if <query>` は query value が 0 ではないことを意味する。

## Map

`map` は有限 tag set 上の写像。

```txt
tags {
color = red blue
}

map revert color {
red -> blue
blue -> red
}
```

map call は bind 済み value に対する value expression として使う。右辺 schema selector、`for` 展開中の token、visual table lookup、visual selector は同じ評価規則を使う。

```txt
once [ box:color ] -> [ box:revert(color) ]
```

これは concrete selector assignments に展開される。

```txt
Boundary:directions {
transparent #555
ascii edge:rotate(directions)
}
```

これは `Boundary` selector が bind した `directions` 値を `rotate` で置換し、対応する `edge` shape value を参照する。

```txt
Boundary:rotate(directions) {
transparent #555
ascii edge:directions
}
```

これは `directions` の元 value を `edge` lookup に使い、`rotate(directions)` を target `Boundary` object の value として使う。

## Layers And Objects

```txt
layers {
floor = Goal Button
actor = Player Box Wall
overlay = @Cursor @Hint
}

group {
solid = actor
}
```

`layers` は位置を持つ main object、display object、layer assignment をまとめる canonical block。

`sprites [name] [of namespace]` は object の見た目を補完する resource block であり、位置を持つ object と layer order の所有者は `layers`。

単純な sprite は `sprites` 内で block braces なしでも書ける。`Box` の次に `#aaa` だけを書くと cell 全体の単色塗りつぶしになる。これは `Background` の次に `#9CBD0F` だけを書くような PuzzleScript 由来の色だけ sprite でも同じで、ASCII pattern 行は省略できる。続けて `00000` などの ASCII pattern 行を書くと、その行数・列数が sprite pixel grid になる。外部画像は `Box sprites/box.png` のように selector と画像パスを 1 行に書き、パスは game folder からの相対参照として HTML renderer に渡される。

再利用する見た目部品は `colors`、`palettes`、`shapes` sub-block に分ける。`colors` は色名、`palettes` は色列、`shapes` は ASCII shape を所有する。sprite entry は `palette <ref>` と `shape <ref>` でそれらを参照する。`<name>:<tag_set>` は selector binding と同じ tag set を使って variant を解決する。

```txt
sprites fixban of sokoban {
colors {
clear = transparent
piece:kind {
A = #4a4
B = #a4a
}
}
palettes {
piece:kind {
A = piece:A clear
B = piece:B clear
}
}
shapes {
mark:kind {
A {
01
10
}
B {
11
00
}
}
}
Box:kind {
palette piece:kind
shape mark:kind
}
}
```

cell は visible objects の有限集合。実装は layer-slot 方式。

同じ cell の同じ layer には最大 1 object しか存在できない。

`<layer_name> = <object-or-selector...>` は名前付き layer を定義する。右辺が未知の名前なら object / schema として作り、既存の object / schema / group / layer tag ならその selector をその layer へ割り当てる。layer 名はそのまま selector tag になり、`floor` は `Goal Button` のように使える。匿名 layer も内部 group 名を持ち、`layers` 展開で利用できる。

`objects { ... }`、`display_objects { ... }`、`layer { ... }`、`layer <name> { ... }` は読まない。object / display object は `layers { ... }` で宣言し、level 文字と表示文字は `levels { legend { ... } }` で宣言する。

rewrite cell の空欄は「未指定」。何も object がないことは意味しない。

```txt
input directions [ Player | ] -> [ | Player ]
```

不存在条件は `no` で書く。

```txt
group {
blocked = Wall Box
}
input directions [ Player | no blocked ] -> [ | Player ]
```

右辺で object を追加するセルは、その object の layer が空いていることを暗黙に要求する。

## Tags And Schemas

有限で順序を持つ tag set:

```txt
tags {
color = red blue
}
```

この名前は `for` や schema tag slot に渡せる tag set である。bare `color = red blue`、単数 `tag ...`、古い `domain <name> ...` 形は public syntax ではない。

schema object:

```txt
object player:color 1
```

これは concrete object variants に展開される。

```txt
player:red
player:blue
```

pattern では selector を使える。

```txt
[ player:* | player:red | player:color ]
```

`player:*` は `player` の全 variants を明示的に選ぶ。variant を持つ schema object では、裸の `player` は全 variants の省略形としては使わない。複数 tag slot の schema では `Box:red:*` や `Box:*:wood` のように、未制約 slot を `*` で明示する。

同じ selector が rewrite 左辺と右辺に出る場合、右辺は左辺で一致した concrete object を保持する。

identity は tag value そのものには載らない。`for c in color` の `c` は各 value として置換される。一方、schema / group selector の identity は、左辺 match が選んだ concrete object assignment と occurrence order によって保持される。

同じ group / schema selector が左辺に複数回出る場合、各 occurrence は独立に cartesian 展開される。右辺の同名 selector は出現順で対応する左辺 occurrence を保持する。

```txt
group {
cargo = Box Crate
}
[ cargo | cargo | ] -> [ | cargo | cargo ]
```

これは `Box Box` / `Box Crate` / `Crate Box` / `Crate Crate` の variants に展開される。

## Legend And Levels

表示文字と level 文字は `levels` 直下の `legend` で定義する。`model puzzle` 直下の `legend` は読まない。

```txt
levels {
legend {
. = empty
* = Goal Box
+ = Goal Player
}
}
```

Section header は既存 block header の sugar として読まれる。

```txt
=======
LEGENDS
=======
. = empty
P = Player
* = Goal Box
```

これは `levels` 直下では表面構文として `legend { ... }` と同じ意味になる。見出し名は lowercase snake_case に正規化され、既存 block 名に対応する場合だけ section として扱われる。たとえば `RULES` は `rules`、`ON DISPLAY` は `on_display`、`LAYERS` は `layers`、`LEGENDS` は `legend` になる。`TRANSITIONS` は canonical section ではない。

`parse_game_file` で読む file は `import "<path>"` で別 file をその場展開できる。相対 path は import を書いた file の directory から解決される。

```txt
puzzle sokoban {
layers { ... }
rules { ... }
import "levels.puzzle"
}

// levels.puzzle
levels {
legend { ... }
level warmup
...
}
```

分割 file も `.puzzle` に統一し、`import` は wrapper を作らず内容をそのまま展開する。import 先は `puzzle sokoban { ... }`、`menu level_select { ... }`、`theme clean` / `theme clean { ... }` のように必要な owner 構文を自分で持つ。

複数 object の `legend` は overlay 表示。

`empty` は object ではなく、何もない cell を表す予約語。

level:

```txt
levels {
level warmup
#####
#P.G#
#####
}
```

`levels { ... }` の中では `level <name>` が名前付き level header になる。`level <name>` の body は次の空白行または `levels` の終端まで続く。複数の unbraced level は空白行で区切る。`level <name>` なしで map chunk を置くと unnamed level になる。

```txt
levels {
#####
#P.G#
#####

#####
#PBG#
#####
}
```

level 内で空白行を region 区切りとして使う multi-region level は block で書く。名前付きは `level <name> { ... }`、名前なしは `{ ... }`。

```txt
levels {
level split_room {
P.
..

.G
..
}

{
P.
..

.G
..
}
}
```

`level` body の中では、その level の parse にだけ使う局所 `legend` を追加できる。

```txt
levels {
level puzzle_with_one_off_tile {
legend {
x = Goal Box
}

Px.
}
}
```

level-local `legend` は `levels` 直下の共有 legend に合成してその level の文字解決だけに使われる。他の level や描画用 legend には書き戻さない。右辺は一つの concrete object set に解決できる必要があり、`empty` は `levels` 直下の `legend` で定義する。

## Win Conditions

```txt
win_conditions {
some Goal
all Goal on Box
}
```

`win_conditions` は loaded game metadata として扱われる。定義済みの named condition として puzzle rule の `if` から参照することもできる。

```txt
some <selector>
all <selector> on <selector>
some <selector> on <selector>
```
