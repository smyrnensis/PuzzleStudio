# Current Spec

この文書は、現時点の実装が採用している `.puzzle` 仕様をまとめる。

## Architecture

```txt
.puzzle source
  -> puzzle-lang parser / compiler
  -> puzzle-core CompiledGame
  -> puzzle-play session / render helpers
  -> html-play
```

`puzzle-core` は `.puzzle` 文法、表示、ファイル IO、level 管理を知らない。

`puzzle-lang` が authoring syntax を読み、`CompiledGame` と level / legend / controls などの metadata を作る。

表示に使う名前付き値も通常の top-level const として `const title = <text>` のように書く。`title` / `subtitle` / `author` / `homepage` は予約されたfieldではなく、他のconstと同じ名前解決・不変性を持つ。scene expressionからはbare nameで読み、`heading title`や`caption author`のように任意のtext roleへ渡せる。

## Game Entry And Imports

ゲーム folder は workspace の単位で、実行時には entry `.puzzle` を明示する。file 自体は次元を持たず、各 `puzzle` model が block 内の `dimension` で自分の次元を宣言する。同じ file に異なる次元の model を置いても、file 名がその次元を上書きまたは制限することはない。旧 `puzzle3` 宣言は error。

```txt
games/fixban/
  game.puzzle
  levels.puzzle
  visuals.puzzle
```

adapter / editor / build tool に folder を渡した場合の entry 選択は host contract が所有する。language compiler は entry path と workspace document set を受け取り、folder scan や file IO を行わない。

各 `.puzzle` は独立した document module であり、完全な top-level 宣言を所有する。entry は model を直接宣言せず、import した model と scene を構成してもよい。

`import <alias> = "<relative-path>.puzzle"` は source root だけで有効。参照は直接 import に対する `<alias>:<name>` で、transitive re-export はない。path escape、missing document、duplicate alias、cycle、nested import は diagnostic になる。import は source text を連結しない。

Canonical example と editor が生成する source は、indent なし、tab 文字なしを標準形とする。
既存 file が whitespace indentation を含むことは許容する。これは authoring style の選択であり、
parser restriction ではない。

## Execution Model

`rules` は必須の puzzle gameplay entrypoint。旧 `transitions` / `main` block は読まない。

```txt
rules {
push
move
}
```

`routine` は名前付き statement list。定義しただけでは実行されない。旧 `rule` declaration は読まない。

```txt
routine movement {
push
move
}
```

routine の application はデフォルトで `once`。

`random` は rule application keyword。`random [ ... ] -> [ ... ]` は適用可能な match のうち1つだけを選ぶ。`random { ... }` と `routine name random { ... }` は発火可能な statement のうち1つだけを選ぶ。選択は hidden RNG ではなく、compiled game、rule、input、solver-visible state、候補数から決まる deterministic tie-breaker であり、同じ state と input では同じ next state になる。確率的に毎回違う結果を得る機能ではない。

`layers { each A:tag_set }` は selector alternatives を別々の通常 layer に展開する。これは collisionless layer ではなく、各 variant が表示順つきの通常 collision layer を得るための短縮文法。

`layers` 内の `for <binding> in <source...> { ... }` は layer row の parse 前に展開される。source は named value set、numeric range、または inline value list。`for k in kind { k = A:k B:k }` は、`A:kind` / `B:kind` が宣言済みなら `red = A:red B:red` のような名前付き layer 行へ展開できる。`for object in Box Wall { ... }` のような inline list は token 置換だけを行い、展開後の body 側構文が object selector や tag value として解釈する。

```txt
routine slide {
input [ Player ] -> [ > Player ]
move
}
```

`routine <name> repeat` は routine block 全体の application を明示的に `repeat` にする。block 内の statement sequence を、block 全体が変化しなくなるまで繰り返す。

rewrite 行も application を持つ。plain rewrite のデフォルトも `repeat`。行ごとに `once` / `once_all` / `once_per_level` / `repeat` を明示できる。標準 `move` routine を使わない direct movement rule では、block と rewrite 行の両方に `once` を明示する。

`once` は row-major order の最初の LHS match に1回適用する。その patch が solver-visible state を変えなくても後続 match へ移らず、match した rule は発火として扱う。

方向や selector の内部展開が複数の concrete rule を生成しても、1つの rewrite 行は1つの `once` application boundary のまま。`for d in directions { ... }` は方向ごとの別 statement を生成するため、各 statement がそれぞれの `once` boundary を持つ。

rewrite-level `repeat` は、solver-visible state を変える最初の match を row-major order で選びながら固定点まで繰り返す。progress 可能な match がなくなった pass では最初の match が発火しても反復を終了する。

`once_all` は、適用開始時点の全マッチを row-major order で集め、それぞれを最大1回ずつ適用する。各マッチは開始 state に対する write proposal を出し、同じ slot に複数 proposal が来た場合は row-major 後続マッチの proposal が勝つ。途中で作られた新しいマッチは同じ `once_all` では拾わない。

`once_per_level` は、その concrete rule が現在の level state 内でまだ発火していない場合だけ、最初の1マッチに適用する。restart / next level で初期 state に戻ると発火済み記録もリセットされる。

```txt
routine direct_slide once {
once input [ Player | ] -> [ | Player ]
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
- level ASCII layer composition is language lowering, not runtime or renderer behavior. Within one blank-line-delimited region, a single `+` line separates same-size ASCII layers. Reserved `.` cells are transparent; non-empty chars are placed into the same compiled state cell. When multiple layer maps place objects into the same cell and core layer, the later map is the upper layer and replaces the earlier object for that core layer before the compiled state is built. Blank lines still split auto-placed regions, so `+` only composes adjacent maps without blank lines.
- component behavior は lowering 後の component が入力の意味を所有する。`choice` は cursor と confirm、scrollable container は scroll を所有する。`level_menu` は runtime component ではない。

例:

```txt
rules {
input [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
```

これは anonymous rules を順番に実行する。

anonymous inline rewrite は application prefix を持てる。

```txt
rules {
input [ Player ] -> [ > Player ]
move
repeat input [ Fire | Wood ] -> [ Fire | Fire ]
}
```

`once` / `repeat` は statement block としても書ける。

```txt
rules {
repeat {
input [ Fire | Wood ] -> [ Fire | Fire ]
input [ Fire | Grass ] -> [ Fire | Fire ]
}
}
```

`repeat until <condition> { ... }` は condition が false の間だけ block を繰り返す pre-check loop。condition が最初から true なら body は0回。condition は `if` と同じ var / named condition / condition / pattern condition を使える。body が発火せず、condition も false のままなら transition error になる。

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

effect 文は `routine` にまとめられる。`[ before ] -> [ after ] <routine>` は rewrite statement 本体を実行し、その statement が LHS match によって trigger された場合だけ、その後に `<routine>` を 1 回呼ぶ。plain rewrite は通常どおり default `repeat` として安定するまで評価され、after routine は repeat 全体が一度でも trigger された後に実行される。省略時の routine block は `once` なので、after routine の中身は通常1回だけ実行される。RHS が state 差分を作らない場合でも、LHS が match すれば trigger として扱う。

`checkpoint` は現在の turn が commit された後の puzzle state を、その puzzle slot の restart 先として保存する。`clear_checkpoint` は保存された checkpoint を捨て、restart 先を level start state に戻す。level 移動や明示的な level load は checkpoint をリセットする。

`fix <tokens> { ... }` は囲んだ rewrite statement のデフォルト application / orientation を固定する。`fix <tokens> ... end` は互換 syntax として読む。

```txt
fix once {
[ > Player | Box ] -> [ > Player | > Box ]
[ > Player | Crate ] -> [ > Player | > Crate ]
}
```

展開後:

```txt
once [ > Player | Box ] -> [ > Player | > Box ]
once [ > Player | Crate ] -> [ > Player | > Crate ]
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

組み込みまたはauthor-definedの方向型setは orientation set prefix として使える。runtime で式評価するのではなく、lowering でset内の方向variantsへ展開する。たとえば3Dの `yz_plane [ ... ]` は `up` / `down` / `front` / `back` のrewriteとして読む。

bare `input [ ... ]` はmodel次元の絶対方向に対する input guard 付き orientation を表す。`input <direction-set> [ ... ]` は対象集合を指定する。lowering 上は、現在の input が対象 set の member だったときだけ、その member の oriented rewrite を評価する。

prefix なしの単独セル pattern は neutral として扱い、offset を方向回転しない。

prefix なしの空間 pattern、つまり複数セル、複数行、ellipsis、または相対方向属性を含む pattern は、2Dでは `up` / `down` / `left` / `right`、3Dでは `horizontal` (`left` / `right` / `front` / `back`) に lower する。3Dで全6方向へ展開する場合は `directions` prefix を明示する。

この規則は rewrite だけでなく、pattern condition と condition pattern にも適用される。

pattern cell の `null` は盤面外セル要求であり、object selector ではない。
`no X` は盤面内 cell に `X` がないことを要求し、`null` はその pattern cell 自体が
盤面外であることを要求する。`null` cell は他 token と混在できず、`no null` は
invalid。rewrite では `null` は match 側の検知専用であり、対応する RHS cell へ
object や mark を書けない。

```txt
[ A | ] -> [ | A ]
[ no Edge | null ] -> [ Edge | ]
some([ Player | Wall ])
some(down [ Rock | ])
some(horizontal [ Rock | ])
some(input horizontal [ Rock | ])
count([ Button | Box ])
count(down [ Rock | ])
count(directions [ Rock | ])
```

上のような prefix なし pattern は4方向 variant を作る。
orientation set prefix の pattern は、その set に含まれる方向 variants へ展開する。`input <direction-set> [ ... ]` は、現在の transition input がsetのmemberのときだけ、そのinputに対応するorientationのpatternとして評価する。

## Value Expansion

`for` は statement list を展開する。

```txt
rules {
for d in directions {
d [ A | ] -> [ | A ]
d [ B | ] -> [ | B ]
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
1...3
1...L
Box Wall Player
tag_1 tag_2 tag_3 tag_4
a b 1...3 z
```

`<start>...<end>` は inclusive numeric range で、`1...3` は `1` / `2` / `3` に展開する。endpoint は整数 literal または同じ puzzle 内で整数 literal に初期化された `var` / `const`。これは parse/lowering 時の authoring expansion であり、runtime loop ではない。mutable var を endpoint に使った場合も、turn 中の更新で展開数は変わらない。同じ range token は `tags` の value list でも使える。

inline value list は `for object in Box Wall Player` や `for tag in tag_1 tag_2 tag_3 tag_4` のように書ける。binding 名は型名ではなく lexical binding なので、object / tag / layer などの意味は展開後の body 側の構文が決める。

絶対方向の組み込みtag setは座標部分空間として定義する。2Dは `x_axis = left/right`、`y_axis = up/down`、`xy_plane = directions`。3Dのcanonical game座標は `right = +X`、`back = +Y`、`down = +Z` とする。したがって `x_axis = left/right`、`y_axis = front/back`、`z_axis = up/down`、`xy_plane = x_axis + y_axis`、`yz_plane = y_axis + z_axis`、`xz_plane = x_axis + z_axis`、`directions` は全6方向。renderer固有の座標系への変換は描画adapterが所有し、language、game state、editor documentの座標には持ち込まない。`horizontal` は2Dでは `x_axis`、3Dでは `xy_plane`、`vertical` は2Dでは `y_axis`、3Dでは `z_axis` の正式alias。2Dで `z_axis` / `yz_plane` / `xz_plane` は使用できない。object schema、`map`、visual table、`for`、orientation prefixは同じset ownerを使う。canonical direction値の部分集合だけからなるauthor-defined tag setも名前に依存せず方向型となる。

`layers` は state layer 定義から作られる tag set。展開値は layer group 名で、名前付き layer はその名前、匿名 layer は内部名を使う。標準 `move` rule はユーザーが同名 rule を定義していない場合に用意される。概念的には次の rule と同じ。

```txt
rule move repeat {
for d in directions {
for l in gameplay_layers {
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

Input guard、bare `input`、direction set を指定する `input directions` は transition context の input 名を参照する。

方向 set に対して同じ rewrite を書く場合は、orientation set prefix を使える。

```txt
directions [ Player | ] -> [ | Player ]
horizontal [ Player | ] -> [ | Player ]
input horizontal [ Player | ] -> [ | Player ]
```

authoring では、puzzle transition に渡る意味入力と、物理 key binding を分ける。

`up` / `down` / `left` / `right` は標準の意味入力で、direction mapping も既定で用意される。`restart` も標準の非方向 input で、既定では `r` key からこの input に対応する。model rule に `restart` input の明示 handler がなければ、`restart -> restart` が暗黙に追加される。`restart -> ...` のような名前 guard sugar は非方向 input でも意味を持つ。別名が必要な場合は `direction` で標準方向への alias を定義する。

```txt
direction east right
direction west left
direction north up
direction south down
```

キーボード入力は owner-scoped な `keys` block で semantic input 名または scene routine に対応させる。puzzle 内の `keys` は puzzle rules へ渡す input、scene 内の `keys` は scene-wide shortcut や title/menu confirm などを処理する routine/effect を定義する。

```txt
puzzle sokoban {
keys {
w ArrowUp -> up
s ArrowDown -> down
a ArrowLeft -> left
d ArrowRight -> right
r -> restart
}
rules {
restart -> restart
}
}
```

```txt
scene title {
keys {
Enter Space x -> confirm
}
button "Play" -> input confirm
routine confirm {
goto playing
}
}
```

`keys { <key...> -> <semantic-input-or-routine-or-effect> }` は、複数の logical key を同じ owner-scoped target に lower する。1文字の token は現在のキーボード配列が生成する論理文字を小文字化した値であり、物理位置を表さない。通常文字に加えて `ArrowUp` / `ArrowDown` / `ArrowLeft` / `ArrowRight` / `Enter` / `Space` / `Escape` / `Tab` / `Backspace` を named key token として書ける。browser adapter は `KeyboardEvent.code` から文字 token を合成しない。`keys { q Escape -> level_select }` は複数 key から scene-local `routine level_select` を呼ぶ shortcut、`keys { Escape -> goto title }` は key から直接 scene effect へ送る shortcut。`keys` では `=` を使わない。`r -> my_restart` のように model input で書くと、既定の `r -> restart` は shadow される。`button "Play" -> input confirm` は button click を semantic input 経路へ送る。model `rules` の `<input> -> <effect>` は input guard の sugar。scene key dispatch は `keys` と `routine` で書く。scene / presentation / lifecycle effect は `effect` wrapper を付けずに直接書く。scene が level lifecycle に介入する場合は `playing.restart` や `board.restart` のように target を明示する。

入力適用後の turn completion では、runtime が post-rules / pre-navigation の snapshot に対して `win_conditions` を評価する。`win_conditions` が true なら、その snapshot を level completion observation として確定してから、model lifecycle として `on_level_clear` を level navigation より前に実行する。solver の built-in completion と state predicate はこの同じ observation を照合し、navigation 後の session state は後続 flow と replay の continuation として別に保持する。通常の clear / advance / restart は model window component と puzzle lifecycle が所有し、scene condition transitions は overlay、menu、hub、特殊分岐などの例外的な flow 介入だけを担う。これは puzzle-core の rewrite ではなく、`GameSession` / standalone HTML runtime が扱う flow である。

`again` command も turn completion で解決される。`again` は入力 event の再送ではなく、同じ puzzle target の rule entrypoint を `InputId(0)` / no semantic input で再実行する follow-up turn request である。follow-up turn は現在の turn の rules、win 判定、lifecycle、navigation command が完了した後に実行される。現在の turn が wait で pause している間は `again` の解決にも進まない。follow-up turn 内で `again` が再び出ると次の no-input turnを実行する。runtime は 1 input から派生する automatic turn を最大 256 回に制限する。follow-up turn も通常の turn と同じ rule segment / wait / resume 契約を使う。各 segment の presentation event は commit 時に発生順で公開される。

Puzzle/model 内の `render` block では `tween = true` が move write に対する tween animation を有効化する。duration は `tween_duration = 160ms` で指定する。`tween_duration` は `tween = true` と同じ render block にある場合だけ有効で、単独では error。旧 block 形の `tween { duration = ... }` は読まない。`tween = false` は明示的な無効化。

Puzzle rule の `wait <duration>` と `wait animation` は、現在の rule segment を commit して turn を pause する。adapter が wait を完了すると runtime は保存した continuation から同じ turn の残りを再開する。pause 中は後続 rule / routine、win 判定、`on_level_clear`、navigation、`again` に進まず、別 input、undo、restart も受け付けない。undo は resume 後も入力開始前の state まで戻る 1 turn boundary のままになる。

同じ rule segment 内で同じ object occurrence に rotation rewrite と move write が発生した場合、runtime は visual tween event と position move event を独立した presentation event として公開する。renderer は描画時に同じ occurrence の両 channel を同じ progress で評価するため、位置移動と回転は同時に進む。`Player:left` から `Player:up` のような direction variant の rotation rewrite は対象になるが、任意 tag variant の置換は visual 差分だけを理由に tween を生成しない。途中の `wait` は segment boundary なので、境界の前後にある event batch は順に描画される。

`wait animation` の resume 条件には、その segment で発生した visual animation events の最大 duration を使う。対象 animation が空なら pause を作らず、そのまま continuation を実行する。`wait 300ms` は指定時間を使う。rule の `message` は awaited `standard.message` overlay component を surface に追加し、その instance が `dismiss` event を受け取るまで同じ continuation 契約で後続 rules を pause する。時間待機は発生させない。`sfx` は pause 条件を持たない。`wait tween` は alias として読めるが canonical は `wait animation`。

## Scenes

`scene` は puzzle transition の外側にある game-flow metadata。`screen <name>` は読まない。

scene は local state を持てる。`layout` block は scene-local state slot と表示 component をまとめて定義する。

scene は 2D / 3D model の所有者ではなく、presentation と flow の所有者である。root layout、component tree、scene input、scene transition は model の次元数に依存しない。model window component は次元にかかわらず `puzzle <slot>` と書く。

`layout` は component ではなく scene root layout block。`layout { ... }` 直下に component を改行で並べる形は、暗黙の `column` として扱う。作者は通常、細かい幅・高さ・gap を書かず、どの component があり、どの選択肢が `row` / `column` / matrix なのかを書く。root scene の論理サイズ、標準 gap、文字・button metrics は default / theme / renderer が持つ。

top-level `puzzle <name>` は、同名の `scene <name>` が明示されていない場合に限り、同名の playable scene を自動追加する。2D では `state { puzzle <name> }`、`layout { <name> }`、`rules { step <name> }` 相当、3D では同じ slot 名で `puzzle` model window を置く scene 相当になる。明示された `scene <name>` は override とみなし、自動 scene は追加しない。

renderer は component を sizing class で扱う。`heading` / `subheading` / `text` / `caption` は一つのtext componentのroleで、既定では`space fit`として親から与えられた幅の中で高さを測る。`puzzle` / `frame` は既定で`space fill 1`のratio contentで、割り当てられたslot内でaspect ratioを守ってcontainされる。`for` は authoring 時のcollection projection、`row` / `column` / `box` は runtime containerである。`level_menu` は `for`、`choice`、containerへloweringされる sugar で、rendererへは届かない。

`choice` は標準 UI cursor で選ばれる主選択肢、`button` は pointer や明示 key binding で押す補助操作である。`choice` だけが logical focus graph に入る。`button` は focus graph に入らない。すべてのtext roleはcellを占有するnon-focusable item。`layout` 直下は暗黙column、`row`はchild footprintを横連結、`column` / `box`は縦連結としてlogical gridに投影する。方向入力は同じ行または同じ列の次のfocusable `choice`にだけ移動する。

space allocation と配置は別契約である。`space fit` は内容量、`space fill [weight]` は主軸の残余空間、`aspect <w> <h>` は比率を表す。container の `align start|center|end|stretch` は cross axis、`distribute start|center|end|between` は main axis を所有する。`gap` と `scroll` はcontainerだけが所有し、文字サイズはtext roleとthemeが所有する。旧 `size` と二軸を混ぜた `align left top` は読まない。

Text component は内部的に一種類で、role は `heading` / `subheading` / `body` / `caption`。authoring keyword `text` は `body` roleへlowerする。標準 typography scale は順に `2rem/1.2`、`1.5rem/1.3`、`1rem/1.5`、`0.75rem/1.4`（font-size/line-height）で、themeが同じrole tokenをoverrideできる。`title`などのconst名はcontent値を参照するだけで、roleやcomponent kindではない。

Canonical scene layout keywords（`for` と `level_menu` は authoring 時に消える）:

```txt
heading
subheading
text
caption
button
choice
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
puzzle  // 3D puzzle model window
```

`panel` は component keyword ではない。styled panel が必要な場合も、文法上は `box` に theme / adapter 側の style を対応させる。

2D model window example:

```txt
scene playing {
state {
puzzle sokoban
}
layout {
sokoban
row {
button "Restart" -> sokoban.restart
button "Levels" -> goto level_select
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
puzzle board = push3d
}
layout {
puzzle board
row {
button "Restart" -> board.restart
button "Levels" -> goto level_select
}
}
}
```

The examples differ only at the model slot initializer and model window component. The implicit vertical stack, buttons, and scene effects are shared scene concepts.

開始 scene は top-level の明示順で決める。`scene` に加えて `puzzle` が生成する同名 scene もその宣言位置で数え、最初の scene を開く。title scene を既定表示にするには、その `scene` を model 宣言より前に置く。adapter は `title` という名前や puzzle component の有無から開始 scene を上書きしない。

2D puzzle の renderer 初期値は puzzle 内の `render` が所有する。現時点では `grid { occupied_cells }` / `grid { all_cells }` を受け付け、前者は object が存在する cell、後者は空 cell を含む全 cell の外周を表示する読み取り補助として扱う。これは floor、collision、rule、win condition、level data には影響しない。省略時は表示しない。

```txt
puzzle sokoban {
render {
grid {
occupied_cells
}
}
}
```

3D puzzle の renderer 初期値は puzzle 内の `render` が所有する。camera は scene layout や rule state ではないため、canonical syntax では puzzle top scope の個別設定ではなく `render` 内の `camera` group に書く。設定 group は `camera yaw=34 pitch=38 interactive_look` の inline 形と、`camera { yaw = 34 ... }` の block 形を同じ意味として扱う。bare option は有効化、値を持つ option は `key=value` で書く。

```txt
puzzle push3d {
render {
camera yaw=34 pitch=38 zoom=1 interactive_look interactive_zoom
lighting {
intensity = 1
ambient = 1
yaw = 53
pitch = 56
color = #ffffff
}
grid occupied_cells
viewport {
zoomscreen 7 7
focus Player
}
pixelate scale=4
shade
}
}
```

`orthographic = true` は正投影を選び、省略時または `false` は透視投影になる。`yaw` / `pitch` / `zoom` は初期 camera view、`interactive_look` は pointer drag による yaw/pitch 変更、`interactive_zoom` は wheel/pinch 系の zoom 変更を許す設定である。`zoom = 1` が `zoomscreen` / `smoothscreen` の通常倍率で、`zoom` や interactive zoom はその framing に対する上書き倍率として扱う。旧 `debug_camera` / `camera_yaw` / `camera_pitch` / `camera_zoom` や `interactive_look = true` のような boolean assignment は受け付けない。

`lighting` は 3D model の初期照明を指定する。`intensity` と `ambient` は backend 固有の lux などではなく、標準照明に対する比率であり、`1` が標準、`0` が消灯である。`ambient` は方向を持たず、光源の反対側を含む全体へ加わる明るさである。`yaw` / `pitch` は主光源が盤面へ入る方向を度数で指定し、`color` は ambient と方向光の共通色を指定する。省略時は上例の値を使う。

3D `zoomscreen` / `smoothscreen` は `render { viewport { ... } }` が所有する focus-follow framing 設定である。`zoomscreen <w> <d>` は focus object を中心に `w x d x full` の仮想 world-space box を置き、その box を現在の camera yaw/pitch で投影して画面に収まる最大倍率にする。`full` は occupied height ではなく `level.size.height` を使う。`zoomscreen <w> <d> <h>` は高さも focus 周りの `h` cell として扱う。`smoothscreen` は同じ desired framing を作るが、描画用 view target / scale だけが遅れて追従する。どちらも culling ではなく framing であり、外側 object を消さない。`focus <selector>` は追従対象で、省略時は `Player`。

Scene layout は `puzzle` を固定 4:3 display として扱う。`puzzle` component は可変 window ではなく、scene から割り当てられた display の内側に 3D visual を描く。scene は level の幅、focus object、`zoomscreen` の有無、投影後の見え方を参照して layout を決めてはいけない。`zoomscreen` の fitting は、親から渡された frame `W x H` と viewport 指定の cell frame `W cells x H cells` から決まる明確な計算であり、DOM や scene layout state を読まない関数として扱う。

3D model `rules` では `set yaw = <deg>` / `set pitch = <deg>` / `set zoom = <n>` を view-state emission として書ける。`reset_camera` は camera view を `render { camera { ... } }` の初期値に戻す。これらは `sfx` と同じく rule 発火に付随する presentation command であり、puzzle state、solver key、win condition には入らない。

`grid { occupied_cells }` は object が存在する cell の外周 edge を表示する preview/debug 用の読み取り補助である。これは floor や volume を追加するものではなく、puzzle state、collision、win condition、level data には影響しない。省略時は表示しない。

`render { shade }` は visual voxel の面ごとの明暗付けを有効にする renderer 設定である。これは puzzle state、visual voxel data、collision、win condition には影響しない。省略時も on。

`pixelate` / `pixelate scale=4` は Three.js の描画解像度を `scale` 分の1にし、nearest-neighbor で表示サイズへ拡大する。省略時の `scale` は `4`。`smoothing` は低解像度描画時の WebGL antialiasing を制御する。省略時は pixel 化しない。

3D object は、その `puzzle` model に属する `visuals` に同名 visual が定義されている場合だけ voxel visual を描く。visual 未指定の object に暗黙の cube や色は割り当てない。位置や占有を読みたい場合は `grid occupied_cells` などの debug 表示を使う。

visual は2D/3D共通の時間 × Z × Y × X model を持つ。ASCIIの列、行、layerはそれぞれcanonical game座標の +X（right）、+Y（back）、+Z（down）へ進む。slice 1は最初のASCII layerで、slice番号も同じ +Z 順に増える。2D visual は depth 1 の特殊例である。`>` だけの行が次の animation frame、`-` だけの行が同じ frame 内の次の +Z layer を表し、shape 内に空行は許さない。3D visual も2Dと同じ `visuals` entry、palette、`shapes` table、`shape =` propertyを使う。resource の dimension は `visuals` keyword ではなく、`of <model>` または所有する `puzzle` / `puzzle` model から決定する。2D owner では `-` を明示的に拒否する。色だけなら2Dでは単色 cell、3Dでは1x1x1 filled cubeになる。

`interactive_look` は semantic input ではない。親 scene は click/drag を 3D camera 用として特別扱いせず、raw input を通常どおり focused scene と layout/hit-test に従って component へ配信する。`puzzle` component は、自分の表示 box 内で始まった pointer drag を取得してよい。`interactive_look` を書いたときだけ、その gesture を camera yaw/pitch の view-state 更新として解釈する。これは model `rules` の `input` には渡らず、`if input == ...`、undo、restart、transition state、win condition には影響しない。

pointer drag の所有者は開始点で決まる。pointer down が `puzzle` の box 内なら、release/cancel まではその component が gesture を capture してよく、途中で pointer が box 外へ出ても同じ drag として継続する。例外は modal、disabled component、overlay、明示的な pointer capture、scene-level gesture など、より具体的な所有者がある場合だけである。

`scene puzzle [name]` は puzzle state を主モデルに持つ playable scene。`name` 省略時は `playing`。scene-local な state slot は puzzle instance を保持し、`layout` は画面配置を表す。scene-local な puzzle slot を明示しない場合は、`<name>` slot が暗黙に `puzzle <name>` として用意される。`board` は予約 slot 名ではない。`input <name...> { update <slot> }` は各 semantic input をその puzzle slot の transition に適用する scene transition へ lowering する。`if win_conditions { ... }` のような unqualified condition は primary puzzle slot の `<slot>.win_conditions` として解決できるが、通常の level progression には使わない。scene transition の `<slot>.<name>` は named condition を先に見て、存在しなければ `<slot>` の var `<name>` を truthy 判定する。

`scene level_menu [name]` の typed scene template は読まない。level list は通常 scene の `layout` 内に `level_menu` sugar として置く。`show_index = <true|false>`、`show_solved = <true|false>`、`layout = list`、`columns = <positive integer>`、`button ...` を読んだ後、同じ `for` projection、通常の `choice`、typed `goto` effect、`scroll=true` のcontainerへloweringする。level record は `id`、`name`、`puzzle`、`pack`、`ordinal`、`progress.cleared` を公開し、`show_solved = true` は `level.progress.cleared` を条件にした通常の label expression へloweringする。静的 metadata は language/model、progress は play session が同じ identity に投影し、HTML adapter は通常の path expression として評価する。`locked = ...` と `wrap = true` は、それぞれavailability field、共通choice navigation policyが未定義なのでエラーになる。

progress save version 2 は level name ではなく公開 `level.id` を `levels[].id` と `currentLevel` に保存する。runtime は name-only entry、欠落 field、型不一致を既定値へ変換せず、save contract の診断として返す。

`sounds { ... }` は top-level の音源定義。`sfx <name> { seed = <seed>; type = <type>; volume = <gain> }` と `music <name> { seed = <seed>; height = <0..1>; bars = <8|16|32|64>; bpm = <40..180>; volume = <gain> }` を持つ。`volume` は 0 以上の gain multiplier で、1 より大きい値は増幅として扱われる。scene は component definition の authoring 名であり、runtime の surface は一つの root と順序付きの content / overlay instance、各 instance の visibility、input focus を持つ。`goto` はrootを置換して履歴を破棄し、`enter` / `back` はroot navigation historyを操作する。root置換は以前のinstanceをすべてunmountし、`present`で作られた一意IDのstateも破棄する。`create` は安定IDを持つhidden overlay instanceとそのstateを作り、`show` / `hide` / `toggle` はmount済みinstanceのvisibilityだけを操作し、`focus` はvisible instanceへのinput routingだけを操作する。`delete` はnon-root instanceとstateを削除する。`move <component> first|last|before <anchor>|after <anchor>` はrootより上の表示順序を操作する。`present <definition>(<property> = <expr>, ...) [as content|overlay] [await <event>]` はrootの上へ一意IDと独立stateを持つvisible instanceを追加するprimitiveで、awaited instanceはdefinition-owned eventを受けるまでmodal input targetとなる。`message <expr>` は登録済み`standard.message` definitionに対する `present standard.message(text = <expr>) as overlay await dismiss` のsugarであり、時間waitは暗黙に追加しない。`wait [duration]` は独立したtimeline waitで、`wait`単体は`default_wait_time`を使う。game progressは `clear_game_progress`、`set current_level = <level>`、`clear current_level`、`set level.cleared = true|false`、`reset persistent_vars` で明示的に操作する。scene直下のlifecycle blockは `on_scene_start { ... }`、puzzle lifecycle blockは `on_level_start` / `on_level_clear` が所有する。複数effectはblockに一行ずつ書く。

`theme = "<preset>"` / `theme { ... }` は top-level の表示 theme metadata。theme は singleton config であり、preset は `theme { preset = "clean" }` のように quoted string で選ぶ。`clean` などの preset 名は作者定義 symbol ではなく builtin preset catalog の値である。theme block の canonical entry は `<setting> = <value>`。公開色は `accent_color`、`background_color`、`text_color` の 3 つだけである。UI の線、選択状態、panel、popup、盤面背景は HTML adapter の preset がこの 3 色の alpha だけで作り、別の実色を持たない。追加の非色設定は `ui_font`、`title_font`、`control_radius`、`panel_radius`。これらは HTML adapter が `--accent` / `--bg` / `--ink` などの CSS custom property へ lower し、preset CSS の値を上書きする。theme は `puzzle-core` の state、rule、transition には入らない。複数 theme 宣言は import 後の順序で preset 名または同じ項目を上書きする。theme 未指定時の default theme preset は `"clean"`。標準 preset は `"clean"`、`"terminal"`、`"paper"`、`"pixel"`、`"puzzlescript"`、`"candy"`、`"blueprint"`、`"noir"` で、HTML adapter は対応する CSS preset を同梱する。

`assets { ... }` は top-level の外部 file manifest。`css "game.css"`、`script "visuals.js"`、`file "visuals/player.png"` を持てる。path は game folder からの相対 path だけ。HTML adapter は宣言された CSS / script だけを読み込み、standalone HTML export は宣言された `file` だけを `PuzzleAssets` に埋め込む。puzzle folder 内の未宣言 file は asset として扱わない。`script` は rendered scene snapshot から追加表示を作るための補助 JS で、puzzle state、transition、undo stack、level index を直接変更してはならない。盤面に追従する script は `window.PuzzleStudio.registerAssetScript({ setup(api) { api.onRender(...) } })` を使う。

```txt
scene play_level {
state {
puzzle sokoban
}
layout {
message = "Push the box"
sokoban
box {
text message
button "Title" -> goto title
}
}
}
```

`puzzle sokoban` は scene-local puzzle state slot を model と同じ名前で定義する標準形。runtime snapshot は session / scene-local scalar state、authored component definition / properties、puzzle slot 一覧を公開せず、それらを評価済み component presentation と component instance 単位の viewport source registry に投影する。viewport leaf は `{ component, source }` の typed identity を持ち、renderer state は同じ identity の registry entry から取得する。viewport上の変化を表すanimation batchだけが同じtyped source identityを持つ。waitはsession timeline上の遅延、audioは解決済みasset IDに対するtyped commandであり、scene名・puzzle名・viewport identity・sound名・seed・recipeを持たない。adapterがこれらのtargetや音声意味論を再構成してはならない。editor/debug が raw state を必要とする場合は player snapshot に field を戻さず、editor feature が所有する別の typed debug projection を使う。複数 slot が必要な場合だけ `sokoban1 = puzzle sokoban` のように明示名を付ける。

platform keyboard event は adapter が `RuntimeKeyTrigger` へ変換し、`SessionAction::Key` として Rust session へ渡す。modal/awaited event、focused scene shortcut、selected choice の confirm / move、model input、未使用 key に対する system action の優先順位は session owner が一度だけ解決する。HTML / Bevy adapter は `z`、`y`、WASD、arrow、confirm などの game semantics を解釈しない。runtime theme も preset 名や CSS token map を公開せず、linear RGBA、typography、control metrics まで Rust presentation owner が解決した typed contract として渡す。

現在の標準 component:

```txt
sokoban
text message
button "Start" -> input start
box {
text message
}
row {
button "Title" -> goto title
}
column {
level_menu {
show_index = true
}
}
```

`heading` / `subheading` / `text` / `caption` は同じ text component のroleで、literal text、scene stateのscalar value、または`for` bindingのpathを表示する。`choice` と `button` は input、component effect、または scene effect を発行する layout component。`choice` は方向キー・ゲームパッドで選ばれる主選択肢、`button` は click/tap や明示 key binding 向けの補助操作である。旧 `button "Label" = name`、`choice "Label" action name`、裸名 RHS は読まない。`box` / `row` / `column` は入れ子の layout tree を作る。allocationは`space fit|fill [weight]`、比率は`aspect <w> <h>`、cross-axis配置は`align`、main-axis配置は`distribute`がそれぞれ所有する。旧`size <w> <h>`と二軸`align <x> [y]`は読まない。通常 `choice` 配列では、layout treeの論理構造に沿ってUI focusが移動する。`for` はprojection primitiveで、cursor移動やconfirm動作は所有しない。

layout の `for level in levels` が公開するrecord fieldは `id`、`name`、`puzzle`、`pack`、`ordinal`、`progress.cleared`。field path は rules と layout が共有する一般の record projection で解決され、record 自体を text や level name へ暗黙変換しない。`title`、`label`、`num`、`number`、`solved`、`cleared` の別名や暗黙値も持たない。current level の判定は puzzle target の公開conditionまたは明示したscene stateを使い、layout用recordをruntime globalとして扱わない。

```txt
scene level_select {
layout {
level_menu {
show_index = true
button "Title" -> goto title
}
}
}
```

`level_menu` 自体には runtime controller がない。authoring 時に `column scroll=true`、同じ `for level in levels`、通常の `choice`、`goto level.puzzle(level.name)` へ展開される。方向入力とconfirmは通常のchoice selection、scrollは通常のcontainer layoutが処理する。`level_menu` は inline source や `->` effect を取らず、表示するlevelの絞り込みは scene の `resources { levels ... }` が所有する。旧 `show index`、`columns <n>`、裸の `wrap`、`action <name>` は読まない。

level の開始、読み込み、restart は level scene / puzzle slot に対する effect として書ける。ただし通常の clear / advance / restart は level scene 内の model window component と puzzle lifecycle が所有する。scene からの target effect は、title/menu から入る、button で明示 restart する、hub から特定 level に飛ぶ、通常 clear とは別の例外 flow に入る、などの介入だけに使う。canonical な開始は `goto sokoban` または `goto sokoban(level_name)`。独自 scene なら `scene playing(level) { state { sokoban(level) } layout { sokoban } rules { step sokoban } }` として `goto playing(level)` で入る。旧 `start levels ... in <scene>` / `continue levels ... in <scene>` は読まない。`playing.restart` は playing scene の現在 level を初期状態に戻し、`playing.next_level` は playing scene を次 level で開始し、`playing.previous_level` は前 level で開始する。`playing.goto <level>` は指定 level で playing scene に移る。`board.restart` のように puzzle slot を target にした場合は、その puzzle state を初期状態に戻す。`board.next_level` はその puzzle を所有する level scene を進める。

puzzle rule でも `win`、`restart`、`next_level`、`again`、`message`、`sfx`、`goto` / `start` を effect として出せる。`win` はその turn の `win_conditions` を true として扱う clear outcome effect で、`set win_conditions = true` の sugar に近い。model rules では `restart -> restart` が semantic input `restart` を model restart effect に接続する rule になる。model rules 内に `restart` input handler がない場合は、この default handler が暗黙に追加される。scene key dispatch は `keys { q -> level_select }` と `routine level_select { goto level_select }` で書く。scene 側で restart / level navigation に介入したい場合は、`board.restart` や `playing.next_level` のように target effect を明示する。これは通常進行の書き方ではなく、ユーザー操作や特殊 flow のための escape hatch である。`[ Goal Box ] -> next_level` と `if win_conditions -> next_level` は board transition の結果として、所有 component/runtime に level advance effect を渡す。`again` は現在の turn を完了した後、runtime に no-input follow-up turn を要求する。`again` が再実行するのは直前の key / semantic input ではなく、同じ puzzle target の rule entrypoint である。したがって follow-up turn では input guard は成立しない。自動 turn は最大 256 回で止まり、`cancel` が出た場合はその自動 turn だけを取り消して停止する。各 automatic turn は通常の turn と同じ wait continuation を持つ。`[ Player Goal ] -> message "Found"` と `[ Player Box ] -> message hint` は awaited `standard.message` overlay component を surface に追加し、その instance の `dismiss` event まで同じ turn の後続 rules を pause する。`[ Player | Box | ] -> [ | Player | Box ] sfx push` は rule が match したときに named SFX を再生する effect を渡す。同じ turn 内で同じ named SFX が複数回出ても再生 event は 1 回にまとめる。`again` の follow-up は別 turn なので、各 automatic turn で同じ SFX を最大 1 回ずつ出せる。model 内の `sounds { move <selector> -> sfx <name> }` は、同じ puzzle scope の最終 catalog に対して selector を解決し、lowering 後の rewrite alternative が対象 object の `Move` writeを持つときだけ、その rule に `sfx` emission を付ける。remove+add は move sound の対象外。canonical syntax は `cantmove` sound trigger を持たない。PuzzleScript importer だけが PS `cantmove` を生成 `move` routine 内の明示的な `sfx` rule として変換する。PS `endlevel` も canonical sound trigger にはせず、生成 `on_level_clear` の先頭に `sfx endlevel` として明示的に埋め込む。

level list は `level_menu` または明示的な `for level in levels` projection で表す。`for` は単なる layout projection であり、cursor 移動や confirm 動作は所有しない。

3D scene の `keys { ... }` も 2D と同じ shared scene contract で扱う。`keys` は `<key...> -> <routine-or-effect>` の scene shortcut、または model/component が読む semantic input への owner-scoped mapping。model-specific input interpretation は `puzzle` / `puzzle` component または model runtime が所有する。

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
set count += 1
```

`set` prefix ありでもなしでも同じ global update operator を使える。`[ Pattern ] -> count += 1` は effect-only rewrite。右辺 pattern は左辺と同じものとして扱い、盤面は変更しない。演算は `i64` の checked arithmetic で、overflow と 0 除算は transition error になる。

`cancel` は effect-only または rewrite suffix として使える。

```txt
[ Player Trap ] -> cancel
[ Player Trap ] -> [ Player Trap Flash ] cancel
```

`cancel` が match した場合、その transition 全体は開始 state に戻って正常終了する。発火した rule は trace に残せるが、board / mark / var の変更は残らない。

`marks { ... }` で宣言した一時 fact は transition-local。値付き mark は `count = int` / `intent = directions` のように宣言し、`Box{count=3}` のように書く。`bool` mark だけは presence / absence として `Box{flag}` / `Box{no flag}` と書ける。`{mark}` は cell-anchored、`Box{mark}` は occurrence-anchored。どちらも rule chain 内では match / write できる。`wait` で同じ program が中断・再開される間は continuation が保持し、program / lifecycle block の完了時に破棄する。solver key / level state / undo / renderer には残らない。

`Box{mark}` と `Box {mark}` は別の anchor を指す。同じ mark 名の anchor 変換や同じ cell pattern 内での同居は valid だが warning になる。`>` / `<` / `^` / `v` sugar は builtin occurrence mark `__move` へ lower される。`parallel` / `perpendicular` は movement mark の相対方向 set sugar で、oriented lowering 時にそれぞれ `<` / `>`、`^` / `v` alternatives へ展開される。

## Condition

`condition` は author が名前を付ける盤面由来の値。core はゲーム語彙を持たず、構造的な condition primitive だけを持つ。

```txt
group {
cargo = Box Crate
}
condition cargo_count = count(cargo)
condition pressed_buttons = count([ Button Box ])
condition any_cargo = exists(cargo)
condition has_pressed_button = exists([ Button Box ])
condition no_cargo = none(cargo)
condition no_pressed_buttons = none([ Button Box ])
```

現在の condition primitive:

```txt
count(selector)
count(pattern)
exists(selector)
exists(pattern)
none(selector)
none(pattern)
some(selector)
some(pattern)
```

`exists` / `none` / `some` は boolean condition primitive として 1 または 0 を返す。意味上は `exists(matcher)` が `count(matcher) > 0`、`none(matcher)` が `count(matcher) == 0` と同値だが、runtime は `count` に lower せず、object count cache または `has_pattern_match` の short-circuit で評価する。`some` は `exists` の alias。

`if <condition> == <number>` と `if <condition>` は rule guard に lower される。`if <condition>` は condition value が 0 ではないことを意味する。

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
edge:rotate(directions)
}
```

これは `Boundary` selector が bind した `directions` 値を `rotate` で置換し、対応する `edge` shape value を参照する。

```txt
Boundary:rotate(directions) {
transparent #555
edge:directions
}
```

これは `directions` の元 value を `edge` lookup に使い、`rotate(directions)` を target `Boundary` object の value として使う。

## Layers And Objects

```txt
layers {
floor = Goal Button
actor = Player Box Wall
}

group {
solid = actor
}
```

`layers` は位置を持つ object の state layer と render priority を同じ順序付き宣言から作る canonical block。

runtime presentation は authoring visual を backend-neutral な `ResolvedVisualClip`、盤面上の `ResolvedRenderInstance`、同一 priority の `ResolvedCompositionGroup` へ materialize する。rendererへ渡す `ResolvedRenderFrame` ではpalette token、visual名、frame timing、animation channel、priority、merge規則はすでに解決済みで、pixel / voxel とlinear RGBA、cell、affine、opacity、object IDだけを持つ。Bevy 3D backendはこのRust型をJSON変換なしで直接消費し、voxel entity、shared mesh/material、camera、light、shadow、culling、batching、GPU bufferを所有する。pixel / external image batchは3D backendで別表現へ推測変換せず、対応する2D backendが存在するまで明示的に拒否する。

`visuals [name] [of namespace]` は object と animation の見た目を補完する resource block。state storage と layer order の所有者は `layers`。

visual block entry は `visual <name> { ... }`、または同じ意味の sugar `<name> { ... }`。名前は header で所有し、body に selector property は持たない。名前が宣言済み object / group / schema selector に解決できれば concrete object visual へ展開して結び付け、解決できなければ standalone visual asset として保持する。

単純な visual は `visuals` 内で block braces なしでも書ける。`Box` の次に `#aaa` だけを書くと cell 全体の単色塗りつぶしになる。これは `Background` の次に `#9CBD0F` だけを書くような PuzzleScript 由来の色だけ visual でも同じで、ASCII pattern 行は省略できる。続けて `00000` などの ASCII pattern 行を書くと、その行数・列数が visual pixel grid になる。`pixels_per_cell <w> <h>` を省略した場合は pattern の幅・高さが 1 cell の pixel grid になり、明示した場合は pattern が cell grid より大きくても描画は overflow できる。外部画像は `Box visuals/box.png` のように selector と画像パスを 1 行に書き、パスは game folder からの相対参照として HTML renderer に渡される。

再利用する見た目部品は `palette` と `shapes` sub-block に分ける。`palette` は色名、`shapes` はvisual dataだけを所有する。`transparent` は通常のpalette色で、empty cellとは異なる。translate/rotateはshapeではなくvisual参照側の順序付き空間操作である。worldが既定で、localだけを明示する。

runtime presentation は authoring visual を backend-neutral な `ResolvedVisualClip`、盤面上の `ResolvedRenderInstance`、同一 priority の `ResolvedCompositionGroup` へ materialize する。clip のpixel/voxelはpalette tokenを含まずlinear RGBAを持つ。render backendへ渡すのは、明示的なclock値とactive presentation eventを使ってRustがframe選択、move/tween/trigger sampling、transform適用、compositionを完了した `ResolvedRenderFrame` である。2D Canvasと3D Three.jsはこのcontractだけを消費し、visual名、palette、frame timing、priority、merge規則、animation channelを解釈しない。Three.jsはresolved voxelからのmesh生成、face culling、material、camera、shadow、GPU buffer更新を所有する。

runtime renderer state は authoring resource の搬送先ではない。3D の `puzzle3AuthoringResources`（object descriptor、visual、palette に相当する resource、input、order）は session snapshot の独立した編集/export channel として公開し、`RuntimePuzzle3Snapshot` と renderer input には含めない。`ResolvedRenderScene.cells` は `position`、`renderOrder`、`objectIds` だけを持つ resolved focus/index 情報であり、Three.js は raw cell/object descriptor を参照しない。camera focus の object ID 解決は Rust projection が所有し、renderer は渡された ID と resolved voxel を実行する。

editor の未完成な level grid と thumbnail は authoring data を直接編集するため、editor-owned DOM renderer を使う。solver の observation / solution / active task と playtest は有効な runtime state なので、Rust がその state を typed render scene へ projection してから runtime renderer が描画する。solver renderer が synthetic cell、visual名、palette、merge規則を解釈する契約は持たない。

`Average` compositionはdisplay framebufferではなくvisualのcanonical sample lattice上で行う。`pixels_per_cell`があればその幅と高さ、なければframe寸法を1 cellのlatticeとする。各instanceのstatic affineとsampled tween affineを合成座標へ適用してから、同じsample領域を持つlinear RGBAを平均する。Average groupの変換後sampleが同じlatticeへ正確に写らない場合はcontract errorであり、backend解像度へのrasterizeや近傍丸めで意味を補わない。ordered compositionは変換済みprimitiveの順序だけをRustが確定し、backendがmesh/sprite/materialへ変換する。

外部画像のfile IOとdecodeはhost asset layerが所有する。asset layerはsource IDに対応する寸法とdecoded RGBAをpresentationへ登録し、presentationはauthored fit、sampling、transform、compositionからrender primitiveを作る。Averageに必要なdecoded assetが未登録ならそのrender frameは未準備であり、別visualや未合成画像を描く経路へ切り替えない。ordered imageもbackendへ作者のfit記法を渡さず、Rustが解決したsource/destination geometryとsampling modeを渡す。pixel geometryにはlogical sampleを直接描くかdecoded rasterとしてsamplingするかも含める。これはvisual種別やsource名の再解釈ではなく、backendが実行するprimitive storageの区分である。

backendがpresentationへ渡す時刻は単調clockの値だけである。clipのloop/once frame、move position、visual tween、trigger visualのdurationと終了判定はpresentationが所有する。2D browser adapterはtyped resolverをWASM経由で使い、Bevy backendは同じresolverをRustから直接使う。色はtyped resolverを使うbackendでlinear RGBAとして合成し、display color spaceへのencodeは最終pixel出力だけが行う。

```txt
visuals fixban of sokoban {
palette {
piece:kind {
A = #4a4
B = #a4a
}
}
shapes {
edge {
111
000
000
}
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
pixels_per_cell 2 2
piece:kind transparent
shape mark:kind
}
}
```

2Dは`translate [world|local] <vec2>`と`rotate [world|local] <angle> [from <angle>]`、3Dは`translate [world|local] <vec3>`と`rotate [world|local] <angle> [from <angle>] [around <direction-or-vec3>]`を使う。3Dで`around`を省略した2D形は、XY平面の回転としてaxisを+Z（`down`）に既定する。`from`は左側のangle expressionから基準angleを引くsugarで、3Dの`Arrow:horizontal { rotate horizontal from front }`はfrontを0度としてright/front/left/backを-90/0/90/-180度へ展開する。2Dの`rotate directions from up`も同じく`rotate (directions - up)`と同じ。world操作はaffine変換へ左合成、local操作は右合成する。主angleを欠く旧`rotate from <angle>`、旧`rotate using`、`offset`、`transform` nodeは受理しない。

cell は visible objects の有限集合。実装は state-slot 方式。

同じ cell の同じ slot には最大 1 object しか存在できない。

`<layer_name> = <object-or-selector...>` は名前付き state layer と同じ位置の render priority を定義する。右辺が未知の名前なら object / schema として作り、既存の object / schema / group / layer tag ならその selector をその state layer へ割り当てる。layer 名はそのまま selector tag になり、`floor` は `Goal Button` のように使える。名前を省略した `Goal` も通常の匿名 layer 宣言である。

`!<visual>` は transient animation visual の render source。object を作らず state layer に入らないが、その行の render priority には入る。通常 object と同居でき、`Box !Box` では object `Box` と animation visual `Box` を同じ priority に置く。

object vocabulary は `layers { ... }` の右辺から作る。level 文字は `levels { legend { ... } }` で宣言する。独立した object 宣言ブロックは public syntax ではない。

rewrite cell の空欄は「未指定」。何も object がないことは意味しない。

RHS cell が `=` だけなら、対応する LHS cell へ parse/lowering 時に展開する。
`=` は RHS 専用で、LHS / condition pattern には置けず、同じ cell で他 token と混ぜられない。

```txt
input [ Player ] -> [ > Player ]
move
```

不存在条件は `no` で書く。

```txt
group {
blocked = Wall Box
}
input [ Player | no blocked ] -> [ | Player ]
```

右辺で object を追加するセルは、その object の state slot が空いていることを暗黙に要求する。

`layers` 直下の `priority = down right`（3D は3方向）は cell 座標の辞書式比較列で、最初に差が出た方向側を前に描く。省略時は2Dが `down right`、3Dが `down right front`。`merge { layer1 = ...; layer2 = ... }` は各行を別の state layer として保ち、描画時だけ一つの unordered priority へ合流させる。merge の重複 pixel / voxel は透明 sample を除いて RGBA channel ごとに単純平均する。

## Tags And Schemas

有限で順序を持つ tag set:

```txt
tags {
color = red blue
count = 1...10
}
```

tag value list の中の `<start>...<end>` は inclusive numeric range として展開する。`count = 1...10` は `count = 1 2 3 4 5 6 7 8 9 10` と同じ tag set を作る。

`*` は selector wildcard の予約 token なので tag value には使えない。`_` を含む identifier atom は通常の tag value として扱う。

この名前は `for` や schema tag slot に渡せる tag set である。bare `color = red blue`、単数 `tag ...`、古い `domain <name> ...` 形は public syntax ではない。

schema object は `layers` の右辺で宣言する:

```txt
layers {
actor = player:color
}
```

これは concrete object variants に展開される。

```txt
player:red
player:blue
```

pattern では selector を使える。

```txt
[ player:* | player:red | player:color | *:red ]
```

`player:*` は `player` の全 variants を明示的に選ぶ。`*:<tag>` は schema family をまたいで、その tag value または tag set に一致する variants を選ぶ。rewrite 右辺の `*:<tag>` は、左辺で一致した family wildcard occurrence の concrete object と同じ schema family / tag slot の target tag variant へ写像する。variant を持つ schema object では、裸の `player` は全 variants の省略形としては使わない。複数 tag slot の schema では `Box:red:*` や `Box:*:wood` のように、未制約 slot を `*` で明示する。

schema selector の slot が同じ puzzle 内の `var` / `const` 名であり、tag value や
tag set 名と衝突しない場合、その slot は dynamic selector になる。たとえば
`var count = 2`、`tags { num = 1 2 3 }`、`layers { actor = Box:num }` の
`Box:count` は、runtime の現在の `count` 値と同じ tag を持つ variant を選ぶ。
`count == 2` なら `Box:2`、`count == 4` なら `num` 外なので match しない。
これは object schema を作る syntax ではないため、`layers` の右辺で `Box:count`
を使って variants を宣言することはしない。

dynamic selector は lowering 時に tag value ごとの guarded static selector へ展開する。
runtime / core は dynamic schema lookup を持たず、通常の object id pattern と
`count == <value>` guard だけを見る。mutable `var` を dynamic selector に使う場合は、
値が tag slot 外へ出ると no-match になる warning を出す。`const` は warning しない。
同じ名前が tag value、tag set、`var` / `const` の複数に見える場合は ambiguous
selector として error。

rewrite 左辺の schema slot 名は tag value capture になる。`[ Obj:kind ] -> captured = kind`
は `Obj` の `kind` slot で match した concrete tag 値を `captured` に書く。
`kind` 参照は同じ rewrite 内で一意に束縛される場合だけ valid。`[ A:kind B:kind ] -> captured = kind`
は、2つの independent occurrence が同じ capture key を持つため ambiguous error になる。
独立した値を使う場合は `[ A:kind#1 B:kind#2 ] -> first = kind#1` のように
tag slot label を付ける。single-slot schema では `[ Obj:* ] -> captured = *` と
`[ Obj:*#1 ] -> captured = *#1` も同じ capture sugar として使える。RHS の `#1`
単体参照は読まず、`*#1` または `kind#1` のように capture key 全体を書く。
capture を puzzle `var` update value として使う場合は、tag value が `true` / `false` /
integer として読める必要がある。

object/group selector、schema tag selector、movement mark set は、rewrite-local な
集合 binding として同じ解決規約を持つ。左辺は concrete value の capture 宣言、右辺は
capture reference。解決優先順位は明示 `#label`、同一 occurrence、一意な compatible
capture。複数候補は ambiguous error、候補なしは unbound error。negated selector は
capture を作らない。`for <binding> in <set>` は rewrite capture ではなく lexical な
列挙であり、この規約の対象外。

組み込み方向setも出現ごとの独立 cartesian
展開にはしない。`[ directions A ] -> [ directions A directions B ]` の右辺は、左辺で
一致した同じ concrete direction を参照する。複数の独立 capture は
`directions#1` / `directions#2` のように label を付ける。

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

右辺で occurrence order とは別の対応を明示したい場合、selector に `#<label>` を付ける。`#` 以降は selector 名ではなく rewrite-local な occurrence label で、selector 解決は `#` より前だけに対して行う。mark も付ける場合は `<selector>#<label>{mark...}` の順で書く。

```txt
[ cargo#1 | cargo#2 ] -> [ cargo#2 | cargo#1 ]
[ Box#1{hot} | Box#2{cold} ] -> [ Box#2{cold} | Box#1{hot} ]
```

同じ label 付き selector occurrence を左辺で複数回定義することはできない。右辺の label 付き selector は左辺に同じ label 付き occurrence が必要。
右辺では同じ label を複数回参照でき、その場合は左辺で一致した同じ concrete object を各出現位置へ書く。

## Legend And Levels

表示文字と level 文字は `levels` 直下の `legend` で定義する。`puzzle` 直下の `legend` は読まない。

```txt
levels {
legend {
. = empty
* = Goal Box
+ = Goal Player
}
}
```

PuzzleScript 風の section header は canonical `.puzzle` 構文では読まない。
`=======` / `LEGENDS` / `=======` や `======` / `LEVELS` / `======`
のような見出しは、明示的な `legend { ... }`、`levels { ... }`、
`rules { ... }` に書き換える。PuzzleScript import の互換処理は
`puzzlescript` translator 側に閉じる。

別 document の model は root import の alias で参照する。

```txt
import board = "models/sokoban.puzzle"

scene playing {
layout {
puzzle main = board:sokoban
}
}
```

import 先も `.puzzle` document であり、`puzzle sokoban { ... }` や `scene title { ... }` のような完全な宣言を持つ。owner block の途中を別 file にする fragment はない。

複数 object の `legend` は overlay 表示。

`empty` は object ではなく、何もない cell を表す予約語。

2D / 3D に共通の `levels` では `.` が empty 文字として予約されており、`legend` に `. = empty` を書かなくても空 cell として読まれる。`. = empty` は同じ契約を明示する表記として受け入れる。`_ = empty` のように別文字を empty にする書き方や、`. = Floor` のように `.` を object に割り当てる書き方は rejected syntax。floor などの実体 object は `, = Floor` のように別の文字へ割り当てる。

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
exists(Goal)
none([ Goal no Box ])
}
```

`win_conditions` は loaded game metadata として扱われる。定義済みの named condition として puzzle rule の `if` から参照することもできる。

```txt
some <selector>
no <selector-or-pattern>
all <selector> on <selector>
some <selector> on <selector>
```

Canonical な勝利条件モデルでは `exists(matcher)` / `none(matcher)` / `count(matcher)` を使う。`some Goal` は `exists(Goal)`、`no <pattern>` は `none(<pattern>)` へ lower される。`all Goal on Box` の勝利判定も generic な same-cell coverage 条件へ lower されるが、language processing は lowering 前の明示構文から Goal を起点、Box を cover とする solver strategy を別契約として生成する。一般の `none` / `no` pattern 条件から空間 strategy は推測しない。

`all <selector> on <selector>` は same-cell coverage sugar であり、右辺に oriented pattern を取らない。方向つきの spatial relation は condition pattern が所有するため、`exists(<orientation> [ ... ])` / `none(<orientation> [ ... ])` または `some <orientation> [ ... ]` / `no <orientation> [ ... ]` で表す。3D の vertical support goal なら `exists(Goal)` と `none(down [ no Box | Goal ])` の組み合わせが canonical で、`all Goal on down [ Box | Goal ]` は受け入れない。
