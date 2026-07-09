# Authoring Syntax

この文書は、`puzzle-lang` が読む `.puzzle` ファイルの文法リファレンスである。

`.puzzle` は人間と AI が編集する authoring syntax であり、`puzzle-lang` が `puzzle-core` の `CompiledGame` に lowering する。

Block の標準表記は `{ ... }`。互換のため `... end` 形式も読めるが、新しく書く `.puzzle` では `{}` に揃える。

## Control Flow Words

canonical な制御語は `if` に寄せる。

- `if`: condition guard。routine statement list 内では block を guard し、scene `rules` 内では condition transition を表す。
- lifecycle hook: `on_level_start { ... }` / `on_level_clear { ... }`。event handler だと読めるように `on_` prefix を使うが、scene transition arrow にはしない。
- component behavior: component が入力の意味を所有する。`level_menu` は cursor 移動と enter を所有するため、author は `cursor.*` や `emit` を書かない。

`when` と二語の `on <event>` はこの制御構文には使わない。

## File Shape

```txt
title "Sokoban"
subtitle "A compact pushing puzzle"
author "Puzzle Author"
homepage "https://example.com"

puzzle sokoban {
layers {
floor = Goal Button
actor = Player Box Wall
@overlay = @Cursor @Hint
}

groups {
solid = actor
}

keys {
w ArrowUp -> up
s ArrowDown -> down
a ArrowLeft -> left
d ArrowRight -> right
r -> restart
}

win_conditions {
some Goal
all Goal on Box
}

var button_is_pushed = false

levels {
legend {
. = empty
* = Goal Box
+ = Goal Player
}

level warmup
#########
#P.B.G..#
#..B.G..#
#.......#
#########
}

rules {
input directions [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
if win_conditions -> next_level
}
}

scene playing {
state {
puzzle sokoban
}
layout {
sokoban
}
rules {
step sokoban
}
}

scene level_select {
layout {
level_menu {
show_index = true
}
button "Title" -> goto title
}
}
```

## Definition Syntax

### `title` / `subtitle` / `author` / `homepage`

```txt
title "Sokoban"
subtitle "A compact pushing puzzle"
author "Puzzle Author"
homepage "https://example.com"
```

表示用・配布用のゲーム metadata。`title` はゲーム名、`subtitle` は短い説明、`author` は作者名、`homepage` は作者または作品の URL。`subtitle` / `author` / `homepage` は省略可能。scene の `title` / `subtitle` component は、引数を省略すると top-level metadata を表示する。scene expression からは `title` / `subtitle` / `author` / `homepage` を top scope の bare name として読む。`name <text>` は top-level metadata としては読まない。

### Tags / Object Schema

有限で順序を持つ tag set を `tags` block で定義できる。tag set は schema の axis として使う object-name atom の集合であり、値は object 名と同じ名前空間の atom として扱う。単数 `tag` 行 sugar や bare `color = red blue` は canonical syntax ではない。

```txt
tags {
color = red blue
facing = left right
count = 1...10
}
```

tag value list の中の `<start>...<end>` は inclusive numeric range として展開される。`count = 1...10` は `count = 1 2 3 4 5 6 7 8 9 10` と同じ tag set を作る。

`directions` は組み込み tag set で、常に `up down left right` を表す。`horizontal` は `left right`、`vertical` は `up down` を表す。これらは object schema、`map`、visual `shape` / `palette`、`for` の展開元で同じように使える。再定義はできない。

`layers` は object 定義から作られる組み込み tag set。名前付き layer はその名前、匿名 layer は内部名で展開される。各 layer 名は同じ layer に属する object group としても登録される。

tag set を使った object schema は、`layers` の右辺で concrete object に展開される。たとえば `Box:color` は `Box:red` / `Box:blue` のような object identity を作る。

tag set の値が object family 名を表す場合、tag set 名に suffix を付けた selector は各 object-name atom に同じ suffix を機械的に付けて解決する。

```txt
tags {
kind = a b
pair = A B
}
layers {
actor = A:kind B:kind C:kind
}

win_conditions = count(pair:a) == 2
```

この `pair:a` は `A:a B:a` と同じ selector 集合として扱われる。`pair:a` の展開先に存在しない selector が含まれる場合は error になる。

```txt
layers {
actor = player:color box:color
marker = marker:directions
}
```

これは概念的には次の object 群を作る。

```txt
player:red
player:blue
box:red
box:blue
marker:up
marker:down
marker:left
marker:right
```

schema selector の slot には、同じ puzzle 内の `var` / `const` も書ける。これは
object variants を作る syntax ではなく、既にある schema variants のうち、現在の
var 値と同じ tag value を持つ object を選ぶ runtime selector である。

```txt
var count = 2

tags {
num = 1 2 3
}

layers {
actor = Box:num
}

rules {
once [ Box:count ] -> [ Box:count ]
}
```

この例で `Box:count` は `count == 2` のとき `Box:2` にだけ一致する。
`count` が `num` 外の値なら、その selector は match しない。mutable `var` を
dynamic selector に使う場合は warning が出る。`const` は値が変わらないので
warning しない。

rewrite 左辺の schema slot 名は、その rewrite の effect で match した tag 値として
参照できる。slot 名参照は同じ rewrite 内で一意に束縛される場合だけ有効で、
複数の selector が同じ slot 名を束縛する場合は `kind#1` のように label を付ける。
single-slot schema では `*` / `*#1` も tag 値 capture として使える。

```txt
var captured = 0
var first = 0
var second = 0

rules {
once [ Obj:kind Detector ] -> captured = kind
once [ A:kind#1 | B:kind#2 ] -> first = kind#1
once [ Obj:* ] -> captured = *
once [ Obj:*#1 ] -> captured = *#1
}
```

`[ A:kind | B:kind ] -> captured = kind` は `kind` がどちらの selector を指すか
曖昧なので error。capture 値を `var` に書く場合、tag 値は `true` / `false` /
integer として読める必要がある。

同じ名前が tag value、tag set、`var` / `const` の複数に見える場合は ambiguous
selector として error になる。黙って tag set や var のどちらかを優先しない。

表示・level 文字は schema とは別に `legend` で定義する。

### `directions` / `direction`

`directions` は標準の `up` / `down` / `left` / `right` を表す組み込み tag set。

`up` / `down` / `left` / `right` は標準の意味入力でもあり、direction mapping も既定で用意される。

これらは physical key ではなく、key を読み替えた semantic input である。キーやタッチなどの物理入力との対応は puzzle の `keys` table で定義し、puzzle rule からは `transition(state, input)` の transition context として参照する。

別名を使いたい場合だけ、`direction` で標準方向への alias を定義する。

```txt
direction east right
direction west left
direction north up
direction south down
```

```txt
direction <alias> <up | down | left | right>
```

`direction east right` は、direction / input 文脈で `east` を `right` として lower する sugar。object 名、group 名、level 名、scene effect 名を置換する general alias ではない。

`direction` alias は、orientation prefix、input guard、`keys` から semantic input に渡す名前で解決される。

`direction <input_name> <dx> <dy>` の数値ベクトル形式は public syntax ではない。

`input` は canonical state ではない。物理 key を読み替えた semantic input として `transition(state, input)` の transition context に渡され、model rule の input sugar から名前を参照できる。

すべての input が方向を持つわけではない。`right` / `left` などは方向付き input なので `input directions [ A | ]` の orientation として使えるが、`enter` や `restart` のような非方向 input は名前としては存在しても orientation にはならない。非方向 input で `input directions [ ... ]` 型の pattern を評価すると match しない。

`restart` は標準の非方向 input として定義済みで、物理 key `r` は既定でこの input に対応する。puzzle rule に `restart` input の明示 handler がなければ、`restart -> restart` が暗黙に追加される。

追加 input は `input <name>` で定義する。方向付き input にしたい場合だけ direction を付ける。

```txt
input rotate
input east direction right
```

scene から特定 component / puzzle slot を直接操作したい場合は target-qualified scene effect として書く。

```txt
button "Restart" -> playing.restart
button "Restart Board" -> board.restart
```

物理入力の対応は puzzle の `keys` block に書く。

```txt
keys {
w ArrowUp -> up
s ArrowDown -> down
a ArrowLeft -> left
d ArrowRight -> right
r -> restart
}
```

`keys` は `<key...> -> <input>` の形で、raw key を puzzle semantic input に割り当てる。`r -> my_restart` のように書くと、既定の `r -> restart` は shadow され、`r` は `my_restart` として解釈される。

`rules` 内では `<input> -> <effect>` を input guard の sugar として書ける。

```txt
rules {
my_restart -> {
message "Press R again to restart"
restart
}
}
```

### `map`

有限 tag set 上の写像を定義できる。

```txt
map <name> <tag_set> {
<from> -> <to>
}
```

```txt
map revert color {
red -> blue
blue -> red
}
```

schema selector の右辺、`for` 展開中の token、visual table lookup、visual selector では、bind 済みの value に map を適用できる。

```txt
once [ box:color ] -> [ box:revert(color) ]
```

```txt
for d in directions {
@Edge:rotate(d) {
edge:d
}
}
```

```txt
Boundary:rotate(directions) {
transparent #555
edge:directions
}
```

これは概念的には次の concrete rewrite variants に展開される。

```txt
once [ box:red ] -> [ box:blue ]
once [ box:blue ] -> [ box:red ]
```

### `layers`

```txt
layers {
floor = Goal Button
actor = Player Box Wall
@overlay = @Cursor @Hint
}

groups {
solid = actor
@hints = @overlay
}
```

`layers` は位置を持つ main object、display object、layer assignment をまとめる canonical authoring block。object / schema を生成できる owner は `layers` だけで、独立した object 宣言 block はない。

`sprites` は object の見た目を補完する block であり、位置を持つ object と layer order の所有者は `layers`。

`<name> = <object-or-selector...>` は「同じ cell に同居できない object 群」を表す。たとえば `actor = Player Box Wall` と書くと、`Player` / `Box` / `Wall` は同じ cell に同時に 1 つしか入れない。

layer 名はそのまま tag selector として使える。たとえば `floor = Goal Button` と書いた後は、rule や legend や condition で `floor` が `Goal Button` の selector として解決される。

右辺は、未知の名前なら新しい object / schema として作られ、既存の object / schema / group / layer tag ならその selector をその layer に割り当てる。

schema family の base 名と同じ単体 object は定義できる。たとえば `Room Room:state` と書いた場合、`Room` は単体 object、`Room:open` / `Room:close` は family variant を指す。`Room` は family 全体の省略形にはならない。family 全体を選ぶときは `Room:*`、特定 variant を選ぶときは `Room:open` のように明示する。`Room` という単体 object が定義されていない場合、裸の `Room` selector は error。

puzzle 直下の declaration/use block は同じ puzzle scope の catalog に対して解決される。したがって `sounds`、`rules`、`win_conditions`、`legend` などの object selector は、同じ puzzle 内の `layers` が作る最終 catalog を見る。block のテキスト順は、statement list や layout child のように順序そのものを表す構文でだけ意味を持つ。

`@Name` は display object を表す。display object は main object と同じ layer order 上に並ぶが、main object と同じ storage layer には入れない。`display @Name` も互換・明示形として読める。`@layer_name = ...` と `@group_name = ...` は display-only の alias であり、右辺に main object を含められない。`@` なしの layer / group は display object を含められない。

```txt
color = red blue

layers {
floor = Goal Button
actor = Player Box Wall
paint = Blob:color
@overlay = @Cursor:color
}

groups {
solid = actor
@cursor_marks = @overlay
}
```

level 文字と表示文字は `levels { legend { ... } }` に書く。`layers` は object identity と storage layer を作るだけで、level 文字は作らない。

semantic selector は layer declaration とは別責務なので、canonical では group row を `groups { ... }` の中に集める。

同じセルの同じ layer には最大 1 object しか入れない。

### `groups`

```txt
groups {
pushable_objects = Box Crate
}
```

group は selector の別名。rewrite では object selector と同じ場所で使える。各 row は `<name> = <selector...>` の形で書く。

group は concrete object selector の集合であり、schema family term を後から suffix 展開する機能は持たない。`A:a B:a` のように concrete object selector として解決できるものだけを入れる。object-name atom set に suffix を付けたい場合は `tags` を使う。

同じ selector の複数 occurrence を右辺で明示的に入れ替えたい場合は、mark より前に `#` で occurrence label を付ける。`#` は object / group / schema 名の一部ではなく、その rewrite 内だけの identity label。

```txt
[ pushable_objects#1 | pushable_objects#2 ] -> [ pushable_objects#2 | pushable_objects#1 ]
[ Box#1{hot} | Box#2{cold} ] -> [ Box#2{cold} | Box#1{hot} ]
```

右辺では同じ label を複数回参照できる。

```txt
[ pushable_objects#1 | pushable_objects#2 ] -> [ pushable_objects#1 | pushable_objects#1 ]
```

### `marks`

```txt
marks {
visited
frontier
intent = directions
count = int
armed = bool
}
```

`marks` は rule chain の中だけで使う transition-local な一時 fact を宣言する block。mark は `State` に保存されず、level / undo / solver key / renderer には残らない。通常 transition、`level_start`、display lifecycle の各実行が終わるとすべて消える。

mark は宣言時に cell 用 / object 用を分けない。書いた位置が anchor を決める。

```txt
{visited}          // cell に付く mark
Box{visited}       // Box occurrence に付く mark
Box {visited}      // Box object + cell mark
Box{intent=right}  // Box occurrence に値付き mark
no {visited}       // cell mark の不存在
Box{no visited}    // Box occurrence mark の不存在
```

型を書かない mark は `flag`。`Box{visited}` は visited flag を足す / 持つこと、`Box{no visited}` は visited flag を消す / 持たないことを表す。`= bool` も presence / absence syntax で使う。

cell mark と occurrence mark は同じ名前を使えるが、互いに match しない。`Box{mark}` と `Box {mark}` は別の意味を持つため、anchor が変わる rewrite や同じ cell pattern 内での同居は valid だが warning になる。

`>` / `<` / `^` / `v` と direction token の prefix sugar は、builtin movement mark へ lower される。内部名は `__move`。

```txt
> Box
Box{__move=right}
```

上の2つは概念的に同じ occurrence mark を表す。通常は sugar か標準 `move` rule 経由で使い、author-defined mark 名として `__move` を再宣言しない。

movement mark では相対方向 set も使える。

```txt
Box{parallel}       // Box{<} または > Box
Box{perpendicular}  // Box{^} または Box{v}
parallel Box        // Box{parallel} と同じ prefix sugar
```

`parallel` / `perpendicular` は rule orientation に対する相対 set。`directions` のような絶対方向 set ではなく、oriented rewrite / pattern condition / condition pattern の lowering 時に concrete な `<` / `>` または `^` / `v` alternatives へ展開される。

### `var` / `const`

```txt
var button_is_pushed = false
const target_moves = 12
var score = 0
persistent var cleared = false
```

```txt
var <name> = <true | false | number>
const <name> = <true | false | number>
persistent var <name> = <true | false | number>
```

名前付き puzzle state 変数または定数の初期値。boolean は内部では `true = 1`, `false = 0` として保持される。`const` は rule guard から読めるが、`set` / `+=` などの rewrite effect では更新できない。`persistent var` は restart / level load などの通常初期化をまたいで session 内の値を再注入する。

top-level の `var` / `const` は scene / puzzle に属さない session 値、`scene` 内の `var` / `const` は scene instance 値として扱う。scene の `const` は `goto ... with name = value` のような scene param では上書きされない。旧 `global <name> = ...` と `persistent <name> = ...` は読まない。

### `condition`

```txt
groups {
cargo = Box Crate
}
condition cargo_count = count(cargo)
condition pressed_buttons = count([ Button Box ])
condition any_cargo = exists(cargo)
condition has_pressed_button = exists([ Button Box ])
condition no_open_doors = none(OpenDoor)
condition no_pressed_buttons = none([ Button Box ])
```

```txt
condition <name> = <condition_expr>
```

`condition` は盤面から読む named value。`cargo_count` や `pressed_buttons` のような意味名は author が決め、core は構造的な操作だけを提供する。

現在の core condition:

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

`exists` / `none` は boolean condition として 1 または 0 を返す。意味としては `exists(matcher)` が「1つ以上ある」、`none(matcher)` が「1つもない」だが、実装は `count(matcher)` を計算して比較するのではなく、match が見つかった時点で止まる condition primitive として扱う。`some(...)` は `exists(...)` の PuzzleScript 互換 alias。

`if` では condition 名、または condition expression を直接参照できる。

```txt
if pressed_buttons == 2
if has_pressed_button
if count(cargo) == 2
if exists(cargo)
if none([ Goal no Box ])
```

### `win_conditions`

```txt
win_conditions {
exists(Goal)
none([ Goal no Box ])
}
```

`win_conditions` は loaded game metadata として扱われる。定義済みの named condition として puzzle rule の `if` から参照することもできる。

```txt
exists(<selector-or-pattern>)
none(<selector-or-pattern>)
some <selector>
no <selector-or-pattern>
all <selector> on <selector>
some <selector> on <selector>
```

`exists(Goal)` と `none([ Goal no Box ])` は「Goal が存在し、Box が乗っていない Goal がなければクリア」という意味になる。PuzzleScript 互換の読みやすい sugar として `some Goal` / `all Goal on Box` も受け付ける。
`all <selector> on <selector>` の右辺は selector だけを受け取る。方向つきの空間関係は `all Goal on down [ Box | Goal ]` のように混ぜず、`exists` / `none` または `some` / `no` の pattern 条件で書く。たとえば 3D で「Box が上にない Goal がない」は `none(down [ no Box | Goal ])` と書く。

### Levels

複数 level を定義できる。

```txt
levels {
level warmup
#########
#P.B.G..#
#########

level hallway
#########
#P.B.G..#
#########
}
```

`levels { ... }` の中では、基本的に `level <name>` の次から level body が始まり、空白行で次の level と区切る。`level <name>` なしで map chunk を置いた場合は unnamed level になる。

```txt
levels {
#########
#P.B.G..#
#########

#########
#P..BG..#
#########
}
```

空白行を level 内の multi-region 区切りとして使いたい場合は、その level だけ block にする。名前付きなら `level <name> { ... }`、名前なしなら `{ ... }` を使う。

```txt
levels {
level two_rooms {
P.
..

.B
..
}

{
P.
..

.B
..
}
}
```

通常のクリア後進行は puzzle rule effect の `next_level` や model lifecycle が所有する。
play UI の固定キーとして `n` が次 level に進めるわけではない。

level body には、その level の puzzle parse だけに有効な局所 `legend` を置ける。

```txt
levels {
level warehouse {
legend {
x = Goal Box
}

P.x
}
}
```

inline でも書ける。

```txt
levels {
level warehouse
legend x = Goal Box
P.x
}
```

level-local `legend` は、その level を読むときだけ `levels` 直下の共有 legend に重ねる。別の level には漏れず、描画用 legend も変更しない。右辺は一つの concrete object set に解決できる必要がある。`empty` は局所定義ではなく `levels` 直下の `legend` で定義する。

同じ region の ASCII map は、空白行を入れずに単独行 `+` でつなぐと複数 layer として重ねられる。`empty` char は透明で、それ以外の char は同じ cell に追加配置される。同じ core layer の object が同じ cell に重なった場合は、後に書いた上側 layer が優先される。空白行は従来通り region separator なので、`+` の前後には空白行を入れない。

```txt
levels {
level intro {
###
#.#
###
+
...
.P.
...
}
}
```

level body では、map row の前に置いた `message` / `sfx` / `wait` はその level の `on_level_start` sugar、map row の後に置いたものは `on_level_clear` sugar として扱う。

```txt
levels {
level intro {
message "I need no one"
P..
...
message "Room clear"
}
}
```

明示的に書きたい場合は `level { ... }` の中に level-local lifecycle block を置く。

```txt
levels {
level intro {
on_level_start {
message "I need no one"
}

P..
...

on_level_clear {
message "Room clear"
}
}
}
```

## Execution Syntax

### `fix`

`fix` は囲んだ rewrite statement のデフォルト application / orientation を固定する。

```txt
fix once {
[ > Player | Box ] -> [ > Player | > Box ]
[ > Player | Crate ] -> [ > Player | > Crate ]
}
```

これは各 rewrite が明示的に `once` を持つのと同じ application で実行される。

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

```txt
fix right {
[ A | ] -> [ | A ]
}
```

これは neutral rewrite を `right` orientation として扱う。

```txt
right [ A | ] -> [ | A ]
```

明示 prefix は `fix` より優先する。`fix once` の中でも `repeat [ ... ] -> [ ... ]` は repeat のまま。`fix` は top-level directive を生成する authoring macro ではない。

### Blocks

`map` / `group` / `keys` / `routine` / `rules` / `level` / `scene` / `for` / `if` / `once` / `repeat` / `fix` は `{ ... }` で block を書く。`inputs` / `rule` / puzzle-level `transitions` / `main` などの旧 header は読まない。

```txt
rules {
input right [ Player | ] -> [ | Player ]
}
```

Section header は、既存 block header の sugar として読める。これは正規文法を置き換えるものではなく、次の section header、同じ階層の block directive、または親 block の終端までを通常の block に展開する。

```txt
=======
LEGENDS
=======
. = empty
P = Player
* = Goal Box
```

上は次と同じ意味になる。

```txt
legend {
. = empty
P = Player
* = Goal Box
}
```

見出し名は英数字、空白、`_`、`-` からなる名前を lowercase snake_case に正規化し、既存 block 名に対応する場合だけ section として扱う。たとえば `RULES` は `rules`、`ON DISPLAY` は `on_display`、`LAYERS` は `layers`、`LEGENDS` は `legend` になる。`TRANSITIONS` は canonical section ではない。未知の見出しは section sugar ではなく通常行として扱われる。

### Imports

ゲーム folder の実行 entry は、top-level `puzzle` または `puzzle3` model を宣言する `.puzzle` / `.puzzle3`。`title` などの top-level metadata は表示情報であり、entry 資格ではない。folder を play / build / editor に渡すと、その folder 内の model-declaring source を entry として読む。複数ある場合は `game.puzzle`、`game.puzzle3`、`<folder>.puzzle`、`<folder>.puzzle3`、`main.puzzle`、`main.puzzle3`、その他の順で優先する。

`levels.puzzle`、`sprites.puzzle`、`menus/title.puzzle` のような model を宣言しない分割 file は import fragment。直接開いた場合、preview / build / play は同じ folder から親 folder へ向かって最初の game entry を探す。entry から明示的に import する。

`parse_game_file` で読む file は、同じ場所へ別 file の内容を展開できる。

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

`import "<path>"` は書いた場所へ file 内容をそのまま展開する。相対 path は import を書いた file の directory から解決され、import された file の中の相対 import もその file の directory から解決される。

`import` は source composition であり、どの game を起動するかは決めない。同じ folder にある `.puzzle` は自動では読まれない。

分割 file も `.puzzle` に統一する。import は wrapper を作らないので、import 先は `puzzle ... {}`、`scene ... {}`、`menu ... {}`、`theme ... {}` のように必要な owner block を自分で持つ。

```txt
// game.puzzle
import "sokoban.puzzle"
import "level_select.puzzle"

// sokoban.puzzle
puzzle sokoban {
...
}

// level_select.puzzle
menu level_select {
...
}
```

`else` も `} else {` または `else {` で書ける。ただし、gameplay としての `else` は `if` セクションの制約に従う。

### `routine`

```txt
routine movement {
input directions [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
```

`routine` は名前付き statement list。定義しただけでは実行されない。puzzle `rules` や他の routine から名前を書いて呼び出す。旧 `rule` declaration は読まない。

`routine @name` は display-only assertion 付き routine を定義する。中に normal rule や normal rule with display effect が混ざるとエラーになる。`routine display <name>` は同じ意味の明示形。旧 `rule @name` / `rule display <name>` は読まない。

routine block の application はデフォルトで `once`。

```txt
routine slide {
input directions [ Player ] -> [ > Player ]
move
}
```

`routine <name> repeat` は routine block 全体の application を明示的に `repeat` にする。block 内の statement sequence を、block 全体が変化しなくなるまで繰り返す。`routine <name> random` は、発火可能な statement のうち1つだけを deterministic に選んで実行する。

rewrite 行も application を持つ。plain rewrite のデフォルトも `repeat` で、必要なら行ごとに `once` / `once_all` / `once_per_level` / `random` / `repeat` を明示できる。

```txt
routine spread repeat {
once input directions [ Fire | Wood ] -> [ Fire | Fire ]
once input directions [ Fire | Grass ] -> [ Fire | Fire ]
}
```

この例は「各行は1回適用、block 全体は変化しなくなるまで反復」。`repeat` を書かない routine では、block 全体は1回だけ実行される。一方で plain rewrite にすると、その rewrite 行自体は変化しなくなるまで適用される。

rewrite-level `repeat` は、同じ concrete rewrite rule の match origin がなくなるまで繰り返す。実行順は row-major order。実装は単純な単一 component / fixed offset rule では dirty-origin delta を使い、それ以外の矩形 pattern、可変 gap、離散 pattern では全 origin を再検査する。

rewrite-level `once_all` は、適用開始時点の全マッチを row-major order で集め、それぞれを最大1回ずつ適用する。各マッチは開始 state に対する write proposal を出し、同じ slot に複数 proposal が来た場合は row-major 後続マッチの proposal が勝つ。途中で作られた新しいマッチは同じ `once_all` では拾わない。

rewrite-level `once_per_level` は、その concrete rule が現在の level state 内でまだ発火していない場合だけ、最初の1マッチに適用する。restart / next level で初期 state に戻ると発火済み記録もリセットされる。

rewrite-level `random` は、適用可能な match のうち1つだけを選ぶ。選択は hidden RNG ではなく solver-visible state から決まるため、同じ state と input では同じ next state になる。`random { ... }` は発火可能な statement のうち1つだけを選ぶ。

`repeat` は変化しなくなるまで実行する。途中で同一 state が再出現した場合は cycle として検出し、巻き戻さず、その再訪 state で repeat を終了する。state が発散して cycle にならない場合も、repeat は内部上限で打ち切り、その時点の state で次の statement へ進む。`cancel` は例外で、repeat の cycle / 上限より優先して turn 全体を取り消す。

標準 `move` routine を使わず、直接位置を書き換える advanced routine では、block と rewrite 行の両方に `once` を明示する。

```txt
routine direct_slide once {
once input directions [ Player | ] -> [ | Player ]
}
```

### `rules`

```txt
rules {
movement
}
```

`rules` は1ターン内で実行する puzzle statement list。現在は必須。旧 `transitions` / `main` block は読まない。

`rules` には named routine call と inline rewrite を直接書ける。

```txt
rules {
input directions [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
```

これは named routine を呼ぶ形と同じ種類の実行列として lowering される。

Rule effect はその statement 位置に直接書くと、pattern match なしに 1 回発火する。`rules` に置けば各 transition で、`on_level_start` に置けば level 初期化時に実行される。旧 `do <effect>` は canonical ではなく、parse error になる。

```txt
rules {
sfx tick
message "Ready"
set moved = false
set moves += 1
}
```

effect 文や follow-up rule は `routine` にまとめられる。rewrite suffix の `<routine>` は、rewrite statement 本体が LHS match によって trigger された場合だけ、その後に 1 回呼ばれる。plain rewrite は default `repeat` として安定するまで評価され、suffix routine は repeat 全体が一度でも trigger された後に実行される。省略時の routine block は `once` なので、suffix routine の中身は通常1回だけ実行される。RHS が state 差分を作らない場合でも、LHS が match すれば trigger として扱う。

```txt
routine clear_feedback once {
sfx clear
message "Clear"
next_level
}

rules {
[ Goal Box ] -> [ Goal Box ] clear_feedback
}
```

### `on_level_start`

```txt
on_level_start {
materialize_level
}
```

`on_level_start` は raw level を runtime が読み込んだ直後に一度だけ実行する statement list。parse 時に `Level.initial_state` へ焼き込まれず、restart / level select / next level のたびに runtime lifecycle として実行される。state 変換だけでなく、`message` / `sfx` などの rule emission もこの時点で presentation event として回収される。

`on_level_start` は通常入力ではないので、`input` orientation や `if input == ...` は使えない。壁の境界、影、初期ライトなど、level map から派生する静的または初期化用オブジェクトの materialize に使う。

Top-level puzzle の `on_level_start` は全 level に共通して走る。特定 level だけの処理は `level <name> { on_level_start { ... } }` に置く。

```txt
routine materialize_level once {
repeat [ Wall no Light ] -> [ Wall Light ]
}

on_level_start {
materialize_level
}
```

旧 directive の `run_rules_on_level_start` とは併用できない。

### `on_level_clear`

```txt
on_level_clear {
mark_clear_state
}
```

`on_level_clear` は `win_conditions` が成立した turn の model lifecycle として、level navigation より前に実行する statement list。scene transition の付属物ではない。クリア時の盤面変換、スコア用 var 更新、クリア演出用 marker の追加に使う。

`on_level_clear` も通常入力ではないので、`input` orientation や `if input == ...` は使えない。通常の clear / next level 処理は puzzle rule effect や model window component の level lifecycle が所有する。scene は overlay、menu、特殊な分岐など、標準 lifecycle から外れる介入だけを target effect として明示する。

inline rewrite にも application prefix を付けられる。名前付き `routine` を作らず、その rewrite だけの適用方式を指定する。

```txt
rules {
input directions [ Player ] -> [ > Player ]
move
repeat input directions [ Fire | Wood ] -> [ Fire | Fire ]
}
```

`once` / `repeat` は複数 statement をまとめる block としても書ける。

```txt
rules {
repeat {
input directions [ Fire | Wood ] -> [ Fire | Fire ]
input directions [ Fire | Grass ] -> [ Fire | Fire ]
}
}
```

block application は source block 単位、rewrite 行の application は source statement 単位。group や schema selector が concrete variants に展開されても、それらは別々の repeated rule ではなく同じ repeat 境界内の alternatives として実行される。

### `if`

`if` は nested statement list に guard を付ける。

```txt
rules {
if button_is_pushed == true {
once [ A ] -> [ APrime ]
once [ B ] -> [ BPrime ]
}
}
```

現在は input guard、puzzle state 変数、condition guard に対応する。bare var は `!= 0` として読む。

```txt
if button_is_pushed
if button_is_pushed == true
if score == 3
if cargo_count == 2
if has_pressed_button
if count(cargo) == 2
if exists(cargo)
```

`else` は gameplay rule に lowering される。

```txt
if button_is_pushed {
once [ Door ] -> [ OpenDoor ]
} else {
once [ Door ] -> [ ClosedDoor ]
}
```

### Input-Gated Direction Rewrite

入力方向に合わせて object を動かす基本形は、まず movement mark を付け、最後に標準搭載の `move` routine を呼ぶ。

`input` は key から読み替えられた semantic input の名前。`input directions [ ... ]` は、現在の input が `directions` の member だったときだけ、その member を rewrite orientation として使う。`> Player` は「その rewrite orientation へ移動したい」という builtin movement mark。実際に隣の cell へ移す処理と collision は標準 `move` routine が担当する。通常の移動例では、直接 `[ | Player ]` へ書き換えたり、`for d in directions { if input == d { ... } }` へ展開して書かない。

```txt
rules {
input directions [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
```

これは概念的には、入力された方向だけに対応する movement intent を Player に付け、前方の Box にも同じ intent を伝播し、最後に `move` で可能な object だけを動かす。

`directions` は常に `up/down/left/right` の4方向 tag set。`input` は canonical state ではなく、`transition(state, input)` の transition context である。

入力とは独立に、方向 set へ同じ rewrite を展開する場合は次の形を使える。

```txt
rules {
directions [ Fire | Wood ] -> [ Fire | Fire ]
horizontal [ Player | ] -> [ | Player ]
input horizontal [ Cursor | ] -> [ | Cursor ]
}
```

`directions [ ... ]` は `up` / `down` / `left` / `right`、`horizontal [ ... ]` は `left` / `right`、`vertical [ ... ]` は `up` / `down` に展開される。これは input guard を付けない。

`input horizontal [ ... ]` は input guard 付きの短縮形。現在の input が `left` または `right` のときだけ、その方向の rewrite として評価する。横移動だけを受け付けたい場合も、通常は intent を付けて `move` を呼ぶ。

同じ形で `input directions [ ... ]` と `input vertical [ ... ]` も使える。これは `input` が方向そのものだという意味ではなく、input 名が指定 set の member だったときに、その member を orientation として使うという意味。

名前だけを見たい場合は `restart -> restart` のような input sugar を使える。これは orientation を要求しないので、非方向 input でも意味を持つ。

入力に連動しない複数方向ルールや、方向ごとに複数の文をまとめて生成したい advanced case では、明示的な `for` block を使える。

### Direction Expansion Trigger

`for <binding> in <source...>` は、有限で順序を持つ source の各 item へ statement list を展開する。source には `directions` / `horizontal` / `vertical` / `layers`、author-defined tag set、numeric range、または inline value list を使える。

よく使う軸:

```txt
for d in directions
for h in horizontal
for v in vertical
for l in layers
for i in 1...3
for i in 1...L
for object in Box Wall Player
for tag in tag_1 tag_2 tag_3 tag_4
for x in a b 1...3 z
```

`directions` / `horizontal` / `vertical` はそれ自体を offset の単位としては使わない。展開後の `up` / `right` などが orientation prefix や movement mark として解釈される。

`<start>...<end>` は inclusive numeric range。`1...3` は `1` / `2` / `3` へ展開される。endpoint は整数 literal または同じ puzzle 内で整数 literal に初期化された `var` / `const` を使える。これは authoring-time expansion なので、turn 中に var を更新しても展開数は変わらない。同じ range token は `tags` の value list でも使える。

inline value list の各 token は、展開後に body 側の構文が object selector、tag value、layer name などとして解釈する。`for object in Box Wall { no object }` は `no Box` / `no Wall` と同じ。`for tag in red blue { Box:tag }` は `Box:red` / `Box:blue` と同じ。

```txt
rules {
for d in directions {
d [ A | ] -> [ | A ]
d [ B | ] -> [ | B ]
}
}
```

`layers` は layer group 名へ展開されるため、`no l` は展開後のその layer group 全体の不存在条件になる。他の layer の object は禁止しない。

標準 rule `move` はユーザーが同名 rule を定義していない場合に用意される。対象は display object を除いた gameplay object の layer。概念的には次の rule と同じ。

```txt
rule move repeat {
for d in directions {
for l in gameplay_layers {
d [ d l | no l ] -> [ | l ]
}
}
}
```

`gameplay_layers` は説明用の名前で、author-facing な tag set ではない。`d l` は「gameplay layer `l` の object が方向 `d` の builtin movement mark を持つ」ことを表す。右辺の `l` は左辺で一致した concrete object を保持し、movement mark は transition 終了時に消える。

これは概念的には、各方向について block 内の statement を順番ごと複製する。

```txt
left [ A | ] -> [ | A ]
left [ B | ] -> [ | B ]

right [ A | ] -> [ | A ]
right [ B | ] -> [ | B ]
```

### Orientation Prefix

方向を明示する rewrite は orientation prefix を持つ。

```txt
right [ Player | ] -> [ | Player ]
left  [ Player | ] -> [ | Player ]
```

`right` / `left` / `up` / `down` などの固定方向名は、その方向に固定された rewrite。

`for` block 内では固定方向名の代わりに binding を書ける。

```txt
for d in directions {
d [ Player | ] -> [ | Player ]
}
```

prefix なしの単独セル rewrite は neutral として扱われ、方向回転しない。

```txt
once [ A ] -> [ APrime ]
[ Switch ] -> [ SwitchOn ] set button_is_pushed = true
[ Button Box ] -> set button_is_pushed = true
[ Button Box ] -> count += 1
[ Button Box ] -> set count += 1
```

prefix なしでも、複数セル、複数行、ellipsis、または `>` / `<` / `^` / `v` のような相対方向属性を含む pattern は PuzzleScript 互換の cardinal pattern として扱う。rewrite、pattern condition、condition pattern のすべてで同じ規則を使う。

```txt
[ A | ] -> [ | A ]
some([ Player | Wall ])
count([ Button | Box ])
```

これらは `up` / `down` / `left` / `right` の4方向 variant へ lower される。

rewrite 末尾には effect を書ける。

```txt
score = 0
score += 1
score -= 1
score *= 2
score /= 2
score %= 10
```

右辺 pattern を省略して `-> count += 1` のように書くと、盤面は変更せず、左辺 pattern が match したときだけ effect を発火する。演算は `i64` の checked arithmetic で、overflow と 0 除算は transition error になる。

`cancel` は rewrite effect として書ける。左辺 pattern が match すると、その transition 全体を入力前の state に戻して正常終了する。途中の board write、mark write、var write は残らない。

```txt
once [ Player Trap ] -> cancel
once [ Player Trap ] -> [ Player Trap Flash ] cancel
```

`win` は puzzle rule effect として書ける。これはその turn の `win_conditions` を true として扱う clear outcome で、`set win_conditions = true` の sugar に近い。実際の board object や named condition 定義は書き換えず、runtime が `on_level_clear` と cleared 記録を実行するための effect として扱う。

`next_level` も puzzle rule effect として書ける。これは board state そのものではなく runtime への level advance command なので、rule は一度だけ発火する扱いになる。

```txt
[ Player Exit ] -> win
[ Goal Box ] -> next_level
if win_conditions -> next_level
```

`again` は PuzzleScript 互換の puzzle rule effect。現在の turn を commit した後、runtime に no-input follow-up turn を要求する。

`again` が「again」するのは物理 key や直前の semantic input ではない。直前に押された `left` / `x` / `Enter` を再送しない。`again` は、同じ puzzle target の通常 rule entrypoint、たとえば scene の `rules { step sokoban }` で指定された `sokoban` を、input なしで 1 turn だけもう一度実行する。したがって `if input == left` のような input guard は `again` turn では false になり、input に依存しない rule や、前 turn が盤面に残した object / mark ではない状態だけが進む。

1つの follow-up turn がまた `again` を出すと、さらに次の no-input follow-up turn が予約される。自動 turn は最大 256 回で止まり、`cancel` が出た場合はその自動 turn だけを取り消して停止する。standalone HTML export では follow-up turn は既定で 120ms ごとに 1 turn ずつ実行され、top-level の `again_interval = 100ms` / `again_interval = 0.1s` で変更できる。PuzzleScript import 互換として `again_interval 0.1` も秒指定として読める。各 follow-up turn は別 snapshot として公開されるため、その turn で発火した `sfx` / `message` も turn ごとに処理される。

```txt
[ Dog | Baby ] -> [ | dog_angry ] again
```

Move tween は puzzle/model 内の `render` block に書く。`tween` を書くこと自体が有効化で、`enabled = true` は使わない。

```txt
puzzle sokoban {
render {
tween = true
tween_duration = 160ms
}
}
```

`wait animation` は rules 内の明示的な animation boundary。そこまでの segment で発生した tween などの visual animation が終わってから、同じ turn の残りの rules を continuation として実行する。animation が発生していなければ no-op。`wait tween` は互換 alias だが、canonical には `wait animation` を使う。

```txt
rules {
input directions [ Player ] -> [ > Player ]
move
wait animation
@refresh_board
}
```

`checkpoint` は現在の turn が commit された後の puzzle state を、その puzzle slot の restart 先として保存する。`restart` は checkpoint があればそこへ戻り、なければ従来どおり level start state へ戻る。`clear_checkpoint` は保存された checkpoint を捨て、restart 先を level start state に戻す。level 移動や明示的な level load は checkpoint をリセットする。

```txt
[ Player Checkpoint ] -> checkpoint
[ Player ResetCheckpoint ] -> clear_checkpoint
```

mark は transition 終了時に自動消去される。明示的な var clear effect は持たない。

```txt
marks {
checked
}

once [ Box ] -> [ Box{checked} ]
once [ Box{checked} ] -> [ Box ]
```

## Pattern Cells

### Multiple Blocks

1つの rewrite side には複数の bracket block を書ける。

```txt
once right [ Player ] [ Bird ] -> [ Player ] [ ]
```

各 block は独立した origin でマッチする。上の例では、`Player` がいる場所と `Bird` がいる場所を別々に探し、右辺の2つ目の空 block によって一致した `Bird` を消す。

左辺と右辺は同じ block 数、同じ cell / `...` 配置でなければならない。

右辺 cell に `=` だけを書くと、対応する左辺 cell をそのまま書いたものとして扱う。
これは「この cell は変えない」ことを短く書くための sugar で、左辺や condition
pattern には使えず、同じ cell 内で他の token と混ぜられない。

```txt
once [ A | B ] -> [ = | C ]
```

### Rectangular Blocks

block 内の `;` は行区切りとして扱う。

```txt
once right [ Player | Box ; Goal | no Wall ] -> [ Player | Box ; Goal | Wall ]
```

これは2x2領域を1つの component としてマッチし、右下 cell に `Wall` を追加する。

向き付き rewrite では、長方形内の `x, y` offset も orientation に従って回転する。

長方形 block 内でも `...` を使える。ただし矩形として解釈できるように、各行の `...` は同じ列位置に置く必要がある。同じ列の `...` は同じ gap 幅を共有する。

```txt
once right [ A | ... | B ; C | ... | D ] -> [ A | ... | X ; C | ... | Y ]
```

### Ellipsis

block 内の `...` は、その向きに沿った可変長 gap を表す。

```txt
once right [ Laser | ... | Target ] -> [ Laser | ... | Ash ]
```

`...` は短い gap から順に試す。右辺にも同じ位置に `...` を書くことで、gap の先にある同じ cell へ write できる。

### Object Cell

```txt
[ Player | Box ]
```

セルにその object が存在することを要求する。

schema object は selector として使える。

```txt
[ player:* | player:red | player:color | marker:left | *:left ]
```

意味:

```txt
player:*      = player の全 variants
player:red    = color が red の player
player:color  = color tag set 上の任意の player
marker:left   = facing tag set の値 left を持つ marker
*:left        = family をまたいで left tag value を持つ全 variants
```

variant を持つ schema object では、裸の `player` は全 variants の省略形としては使わない。全 variants を指定するときは `player:*` と書く。複数 tag slot の schema では `Box:red:*` や `Box:*:wood` のように、未制約 slot を `*` で明示する。

rewrite 右辺で `*:B` のような family wildcard selector を使うと、左辺で一致した `*:A` などの concrete object と同じ schema family の `B` variant へ置き換える。

同じ selector が rewrite 左辺と右辺に出る場合、右辺は左辺で一致した concrete object を保持する。

```txt
input directions [ player:color | box:color | ] -> [ | player:color | box:color ]
```

`player:color` と `box:color` は独立に展開される。同名 tag selector 同士は自動では連動しない。
同じ group / schema selector が左辺に複数回出る場合も、各 occurrence は独立に cartesian 展開される。右辺の同名 selector は出現順で対応する左辺 occurrence を保持する。

```txt
groups {
cargo = Box Crate
}
once [ cargo | cargo | ] -> [ | cargo | cargo ]
```

これは `Box Box` / `Box Crate` / `Crate Box` / `Crate Crate` の variants に展開され、`Box Crate .` に match した場合は `. Box Crate` になる。

### Blank Cells And `no`

```txt
[ Player | ]
[ Player | no Wall ]
[ Player | no pushable_objects ]
```

空欄セルは「何も指定しない」。何もオブジェクトがない、という意味ではない。

不存在を要求するときは `no` を使う。`no` の右側には object / schema selector / group を書ける。

盤面外セルを要求するときは `null` を使う。`null` は object ではなく、対応する
pattern cell が stage の外にあることを検知するための atom。`no Wall` は盤面内
cell に `Wall` がないこと、`null` は cell 自体が盤面外であることを意味する。
`null` は同じ cell 内で他の token と混ぜられず、`no null` も書けない。

```txt
once right [ no Edge | null ] -> [ Edge | ]
```

この例は、右隣が盤面外である右端の cell に `Edge` を置く。

右辺で object を追加するセルは、その object の layer が空いていることを暗黙に要求する。ただし、通常のプレイヤー移動は direct rewrite ではなく、movement mark と標準 `move` routine で書く。

```txt
input directions [ Player ] -> [ > Player ]
move
```

## Visual Syntax

### Display Objects

```txt
layers {
actor = Player Box
@overlay = @Shadow @Glow
@edge = @Edge:directions
}
```

`@Name` は表示・読みやすさ・派生描画用の display object を定義または参照する。display object は solver から除外される。

main object と display object は同じ layer order に並ぶ。描画順は layer の宣言順で決まる。ただし、同じ storage layer に main object と display object を混ぜることはできない。

```txt
layers {
floor = Goal
@floor_overlay = @Shadow @Glow
actor = Player Box Wall
@edge = @Edge:directions
}
```

`layers` の `each <selector...>` 行は、selector の concrete alternatives をそれぞれ別の通常 layer に展開する。これは collision しない特殊 layer ではない。作られた各 layer は通常どおり collision layer であり、宣言順の表示順も持つ。

`layers` 内でも `for <binding> in <tag_set> { ... }` を使える。これは layer row を parse する前に token 展開される sugar なので、selector 側だけでなく layer 名側にも効く。

```txt
tags {
kind = red blue
}
layers {
for k in kind {
k = A:k B:k
}
}
```

これは `red = A:red B:red` / `blue = A:blue B:blue` と同じ。`A:k` のように concrete value へ展開する場合、schema は同じ `layers` block 内の展開後の右辺から生成される。

display object は通常プレイでは state に存在できるが、solver の state key と solver transition からは除外される。ASCII 表示でも既定では無視される。

display object は `layers { @overlay = @Name }` のように layer assignment と同時に宣言する。

### `display`

```txt
routine @refresh_board once {
repeat [ @Light ] -> []
[ Player no @Light ] -> [ Player @Light ]
}

rules {
move
@refresh_board
display [ Box no @Light ] -> [ Box @Light ]
display {
[ Goal no @Light ] -> [ Goal @Light ]
}
}
```

`@routine` は display-only assertion 付き routine をその場で実行する statement。`display @routine` / `display <routine>` は互換・明示形として読める。`display [ ... ] -> ...` は宣言なしの一行 display rewrite。`display { ... }` は宣言なしの複数行 display block。いずれも `rules`、`on_level_start`、`on_level_clear` などの statement block の中に置き、置いた位置で実行される。

display statement は main object を pattern / condition で読める。ただし write できる object は `@Name` display object だけ。通常入力の transition 中に実行される display statement は、同じ transition context の `input` orientation や `if input == ...` を使える。`cancel` や var update などの effect は使えない。

rule の role は routine 単位ではなく rewrite 単位で決まる。match に display object があり normal state を変えない rule、または match に display object がなく display object だけを書く rule は display rule。match に display object がある rule が normal state を変えようとするとエラー。match に display object がなく、normal state と display object を同時に変える rule は normal rule with display effect になる。

puzzle `rules`、`on_level_start`、`on_level_clear`、goal / condition などの gameplay 側は display object を読めない。display object に依存する派生表示は display rule 側に閉じる。normal routine から display routine を bare call してもよい。

solver は display statement 由来の rule を実行せず、display object を state key からも外す。

`on_level_start` / `on_level_clear` の中でも display statement は使える。ただし lifecycle block は通常入力ではないため、その中の display statement は `input` orientation や `if input == ...` を使えない。

### `on_display`

```txt
routine @paint once {
repeat [ @Glow ] -> []
[ Goal no @Glow ] -> [ Goal @Glow ]
}

on_display {
@paint
}
```

`on_display` は表示 snapshot を作る直前に走る display-only hook。renderer、editor、preview は raw gameplay state を表示する前にこの hook を適用できる。`rules` の途中に置く `display paint` は call-site のアニメーション境界として残し、`on_display` は editor の直接編集、restart、undo、level load など turn を通らない状態にも同じ visual derivation をかけるために使う。

`on_display` の中には display statement だけを書ける。`on_display` は通常入力ではないため、`input` orientation や `if input == ...` は使えない。

### `sprites`

```txt
sprites {
Box
#aaa

Background
#9CBD0F

Crate
#aaa
00000
00000
00000
00000
00000

Gem sprites/gem.png

Player {
pixels_per_cell 5 5
offset 2 -1
#e94f64 #2f80ed #22a06b
........
..00....
..01....
........
}

shapes {
edge:directions {
rotate from up
11111
00000
00000
00000
00000
}

player_shape {
........
..00....
..01....
........
}
}

Boundary:directions {
rotate from up
transparent #555
11111
00000
00000
00000
00000
}

Player {
pixels_per_cell 5 5
offset 2 -1
#e94f64 #2f80ed
shape player_shape
}
}
```

sprite entry は、selector block の中に色行、ASCII pattern の順で書ける。canonical では `pixels_per_cell` / `offset` の配置メタデータを上に置き、`rotate from <value>` を使う場合はその次、色行、ASCII pattern または `shape <name>` の順で書く。brace なし sprite entry でも同じ順序で `rotate from <value>` を書ける。色行は `colors` keyword を付けてもよいが、省略するのが canonical。色は CSS color として渡されるため、`transparent`、基本 CSS color keywords、`orange`、`grey` / `gray` variants、`brown`、`pink`、`#rrggbbaa` の alpha 付き hex も使える。`.` は透明、`0`..`9`、`a`..`z`、`A`..`Z` は色行の順序に対応する。

単純な sprite は block braces なしでも書ける。selector の次の行が色 1 つだけで pattern がなければ cell 全体の単色塗りつぶしになる。これは `Background` / `#9CBD0F` のような PuzzleScript 由来の色だけ sprite でも同じで、`00000` のようなダミー ASCII pattern は不要。pattern を続けると、その行数・列数が sprite pixel grid になる。`pixels_per_cell <w> <h>` を省略した場合は ASCII pattern の行数・列数が 1 cell の pixel grid になる。明示した場合は、pattern がその grid より大きくても描画は overflow できる。

外部画像は `Box sprites/box.png` のように selector と画像パスを 1 行に書ける。パスは game folder からの相対パスとして HTML renderer に渡される。

shape lookup は value expression を読める。たとえば `edge:rotate(directions)` は、selector で bind された `directions` 値を `rotate` map で置換してから shape table を引く。再利用したい pattern は `shape` と object block 内の色行 + `shape <ref>` で分けて書く。

`offset <x> <y>` は描画位置だけをずらす。基準は sprite pixel grid の左上で、正の x は右、正の y は下。object の実セル、collision、rule matching は変えない。

`sprites` は HTML renderer 用の sprite alias と ASCII pattern を定義する。`shapes` 内の `<name>:<tag_set>` は value ごとの ASCII pattern table。table 内または sprite entry 内で `rotate from <value>` の後に pattern rows を書くと、その pattern を `<value>` として登録し、標準方向 tag set では `up -> right -> down -> left -> up` の内蔵 cycle で他の value の pattern を生成する。

HTML renderer が生成する sprite 名と CSS class は、object 名の大文字・小文字を保持する。CSS class として危ない区切り文字だけ `-` に置き換える。例: `Player` は `.sprite.Player`、`Box:A` は `.sprite.Box-A`。

標準方向以外の tag set や別順の cycle を使う場合は table 内で `rotate using <map_name> from <value>` の後に pattern rows を書く。既存 table entry と分けたい場合は、`<value> { ... }` の後に `rotate from <value>` だけを書く形も読む。旧互換として `rotate from <value> { ... }`、`shape <name>:<tag_set> rotate from <value>`、`shape <name>:<tag_set> rotate <map_name> from <value>` も読むが、canonical では table header や source value に余分な block を足さない。rotation は parse/lowering 時点で通常の shape entries に展開されるため、runtime state や renderer は tag set 固有の挙動を持たない。

### `legend`

```txt
levels {
legend {
. = empty
* = Goal Box
+ = Goal Player
}
}
```

`legend` は `levels` 直下で level/render 用の文字対応を定義する。`puzzle` 直下の `legend` は読まない。

`empty` は object ではなく、何もない cell を表す予約語。

3D の `levels3` では、`.` は empty 文字として予約されているので `legend` に書かなくてよい。`_ = empty` のように別文字を empty にする書き方や、`. = Floor` のように `.` を object に割り当てる書き方は使わない。floor などの実体 object は `, = Floor` のように別の文字へ割り当てる。

右辺は既存の object / schema / group / layer tag selector に解決される必要がある。`legend` は新しい object を定義しない。未知の名前は parse error。

複数 object を右辺に書くと overlay 表示になる。

```txt
* = Goal Box
+ = Goal Player
```

単体 object の表示文字も `levels { legend { ... } }` で定義する。

### `render_overlay`

```txt
render_overlay Button Box X
render_overlay Goal Box *
```

```txt
render_overlay <selector> <selector> [selector...] <char>
```

複数 selector が同じ cell にあるときの表示文字を定義する。組み合わせが単一 concrete object set に解決できる場合、その文字は level parse でも使える。

`legend * = Goal Box` は overlay 表示だけを定義する簡潔な書き方で、`render_overlay` は同じ役割を明示 directive として書く形式。

## Sounds Syntax

`sounds` は音源定義だけを持つ top-level block。盤面状態や rule core には入らない。

```txt
sounds {
sfx click seed=746670 type=jump volume=1.2
music loop seed=123456 tone=0.62 bpm=104 volume=1.4
}
```

`sfx` は one-shot sound effect、`music` は loop 用の background track。`seed` は必須。`sfx type` は省略時 `random`。標準の seeded SFX type は `jump`、`step`、`pickup`、`hit`、`drag`、`water`、`lock`、`explosion`、`laser`、`powerup`、`select`、`error`。`type=puzzlescript` は PuzzleScript の numeric sound seed 互換 generator を選ぶための import 用 type。`music tone` / `bpm` / `volume` は省略時 `0.62` / `104` / `0.8`。`volume` は 0 以上の gain multiplier で、1 より大きい値は増幅として扱われる。

presentation として明示的に鳴らす音は scene / component の effect が所有する。

```txt
button "Start" -> input start_game
input start_game {
sfx click
play_music loop
goto playing
}

input level_clear {
sfx clear
stop_music loop
goto level_select
}
```

## Theme Syntax

`theme` は HTML 表示用の top-level metadata。game state の遷移、rule、solver key には入らない。
theme の見た目の identity は HTML adapter の CSS preset が持つ。`.puzzle` 側の
`theme` 宣言は preset 名の選択と、作者に公開する少数の調整項目だけを持つ。

```txt
theme clean {
accent_color #2f7ebc
ui_font sans-serif
}
```

`theme <theme>` は preset 名だけを選ぶ。`theme <theme> { ... }` は同じ preset を選び、詳細設定を上書きする。各行は公開された調整項目
`<setting> <value>` を canonical とする。互換 syntax として `<setting> = <value>` も読む。
公開項目のうち、色は `accent_color`、`background_color`、`text_color` の 3 つだけである。
UI の線、選択状態、panel、popup、盤面背景は preset がこの 3 色の alpha だけで作る。
追加の非色設定は `ui_font`、`title_font`、`control_radius`、`panel_radius`。
これらは HTML adapter の CSS custom property に lower され、preset CSS の値を上書きする。
値は space を含まない compact CSS token にする。
複数 theme 宣言は import 展開後の順序で preset 名または同じ項目を上書きする。theme 未指定時の default theme name は `clean`。

標準 preset は `clean`、`terminal`、`paper`、`pixel`、`candy`、`blueprint`、`noir`。HTML adapter は対応する `theme-clean` / `theme-terminal` / `theme-paper` / `theme-pixel` / `theme-candy` / `theme-blueprint` / `theme-noir` CSS preset を同梱し、そこで各 theme の見た目の identity を定義する。

外部 CSS / JS は `assets` block で明示する。puzzle と同じ folder からの相対 path だけを書ける。

```txt
assets {
css "game.css"
script "visuals.js"
file "sprites/player.png"
}
```

`css` は HTML adapter が stylesheet として読み込む。`script` は rendered scene snapshot から追加表示を作るための補助 JS で、puzzle state、transition、undo stack、level index を直接変更してはならない。`file` は script や visual sprite から `api.assetUrl("sprites/player.png")` / `source: "sprites/player.png"` として参照する静的 asset を standalone HTML export に埋め込む。script が盤面に追従する場合は `window.PuzzleStudio.registerAssetScript({ setup(api) { api.onRender(...) } })` を使う。

scene では、scene が focus されたタイミングを `on_scene_start` lifecycle block として扱える。BGM の開始/停止など、puzzle 初期化ではなく presentation に属する処理に使う。`on_level_start` は puzzle lifecycle block なので scene には置けない。

```txt
on_scene_start {
stop_music loop
play_music loop
}
```

scene / component RHS の canonical form は、`input <name>`、`component_effect <name>`、bare scene routine name、または direct scene effect。`input <name>` は focus 中の scene transition または puzzle transition に渡る semantic input で、1 回の遷移中に必ず 1 つだけ存在する原因として扱う。`component_effect <name>` は `level_menu` の cursor 移動や enter のように component が所有する操作に使う。scene / presentation / lifecycle effect は `effect` wrapper を付けずに直接書く。

scene effect は `sfx <name>`、`play_music <name>`、`pause_music [name]`、`resume_music [name]`、`stop_music [name]`、`goto <scene>`、`goto <scene>(<level>)`、`start <scene>`、`start <scene>(<level>)`、`clear_undo_history`、`clear_game_progress`、`<target>.restart`、`<target>.next_level` など。scene navigation の canonical form は `goto` と `start` だけ。`goto` は固定 scene node へ切り替え、既存の scene state を保持する。`start` は target scene state を初期化してから `goto` する。level scene への入場も `goto sokoban`、`goto sokoban(level_name)`、`goto playing(level)` のように scene call として書く。level 指定なしの `goto <level scene>` は保存済みまたは選択中の `current_level` を使い、なければ最初の level に入る。`resume` / `continue` / `open` / `enter` / `back` / `close` は canonical scene navigation ではない。旧 `start levels ... in <scene>` / `continue levels ... in <scene>` は読まない。通常の clear / advance / restart は model window component と puzzle lifecycle の責務なので、scene からの target-qualified level effect は button、menu、debug、hub、分岐演出などの明示的な介入に限る。`play_sfx <name>` と `effect <effect>` wrapper は読まない。Effect は単体でも列でも同じ場所に書ける。列の分割は effect vocabulary が所有し、`button "New Game" -> goto playing play_music music` のように component 側は列を特別扱いしない。曖昧な引数を避けたい場合は `on_scene_start` / `if` block、または scene `rules` 内の block に 1 行ずつ書く。`then` による inline sequence は使わない。

game progress は scene effect から明示的に操作できる。`clear_game_progress` は `level.cleared` を全 level で false にし、`current_level` を初期状態に戻し、`persistent var` を既定値に戻す。細かく操作する場合は `set current_level = <level>`、`clear current_level`、`set level.cleared = true|false`、`set level(<level>).cleared = true|false`、`reset persistent_vars`、`reset <persistent var>` を使う。undo/redo 履歴だけを捨てる場合は `clear_undo_history` を使う。

`message <expr>` は scene / component effect として popup message を表示し、既定で `default_wait_time` だけ待つ。`expr` は quoted text、scene `var`、top-level `var`、または effect binding を参照できる。

`wait [duration]` は scene / component effect sequence の presentation wait。`duration` は `wait 0.1s`、`wait 1s`、`wait 100ms` のように秒またはミリ秒で書く。`wait` 単体は既定で `0.2s` 待つ。top-level に `default_wait_time = 500ms` のように書くと、bare `wait` と message の既定待ち時間を変更できる。

```txt
var hint = "Push the box onto the goal"

scene playing {
layout {
sokoban
}
on_scene_start {
message hint
}
}
```

scene condition は current level context を読める。level 固有の演出は `message` に level 指定を持たせず、scene / lifecycle 側の `if` で囲む。level 進行そのものを scene condition の標準責務にしない。

```txt
on_scene_start {
if level.name == microban_01 {
message hint
}
}

rules {
if level.name == microban_03 and sokoban.special_clear -> {
message "Secret route"
goto secret_clear
}
}
```

authoring で推奨する level 指定は `level.name`。`level.label` は表示名として読めるが、現時点では `level.name` と同じ値を返す。`level.last` / `level.has_next` は真偽 condition として使える。

puzzle rule の rewrite effect としても `message "text"` / `message <path>` / `sfx <name>` を書ける。`message` は popup を出し、既定で `default_wait_time` だけ後続 effect / 後続 rule segment を待たせる。この effect は board state ではなく presentation command なので、`puzzle-core` の状態には残らない。

```txt
[ Player Goal ] -> message "You found the goal"
[ Player Box ] -> message hint
[ Player | Box | ] -> [ | Player | Box ] sfx push
```

同じ turn 中に同じ `sfx` が複数回要求されても、再生 event は 1 回にまとめられる。`again` による follow-up turn は別 turn なので、各 automatic turn で同じ `sfx` を最大 1 回ずつ鳴らせる。

model 内の `sounds` block では、object が実際に move として lower された rule firing に SFX を結びつけられる。

```txt
puzzle sokoban {
sounds {
move Box -> sfx push
cantmove Box -> sfx bump
undo -> sfx back
restart -> sfx reset
}

layers {
actor = Player Box
}
}
```

`move <selector> -> sfx <name>` / `cantmove <selector> -> sfx <name>` の `<selector>` は通常の object selector / group / schema selector。`sounds` が `layers` より前にあっても、同じ puzzle scope の最終 catalog に対して解決する。これは runtime event watcher ではなく lowering sugar で、rewrite alternative が standard move の move / blocked move に対応するときだけ、その rule に `sfx` emission を付ける。remove+add として書かれた変化は move ではないので対象外。

`undo -> sfx <name>` / `restart -> sfx <name>` は、同じ model の session 操作が成功したときに one-shot SFX を鳴らす。これは rule input ではなく play/session 操作の presentation event なので、undo stack が空の undo では鳴らず、active puzzle がない restart でも鳴らない。top-level `sounds` は音源定義だけを持つため、これらの操作割り当ては puzzle 内の `sounds` に書く。

PuzzleScript の `Sounds` section は object move / create / SFX0 などの runtime event に seed を結びつける。importer は `sfx0 12345` のような単純な named seed を `sounds { sfx sfx0 seed=12345 type=puzzlescript }` に lower し、rule suffix の `SFX0` は明示的な `sfx sfx0` として鳴らす。PuzzleScript importer の event-based sounds、たとえば object movement / create sounds はまだ canonical syntax へ自動変換しない。

## Menu / Scene Syntax

### `scene`

```txt
scene playing {
state {
puzzle sokoban
}
layout {
sokoban
}
rules {
step sokoban
}
}
```

`scene` はゲーム全体の場面を定義する。renderer 固有の HTML ではなく、runtime が scene model に落とすための構造化 metadata である。`screen <name>` は読まない。`state { puzzle sokoban }` は scene slot と model 名を同じ `sokoban` にする標準形で、`step sokoban` はその slot を現在 input で 1 turn 進める。level clear / advance の通常処理は、scene の condition transition ではなく、puzzle rule effect と model window component の lifecycle に閉じる。

top-level に `puzzle sokoban { ... }` を定義し、同名の `scene sokoban` がなければ、同じ名前の playable scene が自動で追加される。これは `state { puzzle sokoban }`、`layout { sokoban }`、`rules { step sokoban }` を持つ scene と同等で、`goto sokoban(first)` のように直接入れる。`puzzle3 push3d { ... }` でも同じ規則で `puzzle3` window の scene が追加される。モデル block 内に `layout { ... }` を書いた場合は、その `layout` が同名 scene の layout になり、bare `puzzle` / `puzzle3` はそのモデル自身の window を意味する。作者が `scene sokoban { ... }` を明示した場合は、その明示 scene が override であり、自動 scene は追加されない。

`state` は scene-local な状態 slot を定義し、`layout` はその表示 component tree を定義する。値は現時点では bool / integer / symbol / quoted text を読める。

`puzzle sokoban` は scene-local な puzzle state slot を model と同じ名前で定義する。複数 instance が必要な場合は `sokoban1 = puzzle sokoban` のように明示名を付けられるが、これは advanced な形として扱う。

scene は 2D / 3D model の違いを直接所有しない。scene が所有するのは root layout、component tree、入力、遷移で、model の違いは model window component に閉じる。`layout { ... }` 直下に component を改行で並べる形は、暗黙の `column` として扱う。作者は通常、細かい幅・高さ・gap を書かず、どの component があり、どの選択肢が縦積み・横並び・matrix なのかを書く。root scene の論理サイズ、標準 gap、文字・button metrics は default / theme / renderer が持つ。

`choice` は方向キー・ゲームパッドで選ばれる主選択肢、`button` は click/tap や明示 key binding で押す補助操作である。標準 UI focus cursor に入るのは `choice` だけで、`button` は入らない。`text` / `title` / `subtitle` は cell を占有するが選択対象ではない。`row` は children を横に、`column` / `box` は縦に連結して論理 grid を作る。方向入力は同じ行または同じ列の次の `choice` にだけ移動し、欠けている cell へ斜めに補正しない。Enter/Space/x は focused choice を実行する。scene はデフォルトで input を component 群へ broadcast し、各 component が関係する input だけに反応する。これは UI focus であり、puzzle の cursor movement ではない。

renderer は component を sizing class で扱う。`title` / `subtitle` / `text` / `button` は flow content、`puzzle` / `puzzle3` / `frame` は ratio content、`level_menu` / `menu` / `for` は collection content、`row` / `column` / `box` は container である。ratio content は割り当てられた slot 内で aspect ratio を守って contain される。`size` / `gap` / `align` は既存ファイル向けに読めるが、新しい例では default に任せる。

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

3D model window でも、scene / layout / component の形は同じにする。違うのは state slot の initializer と model window component だけ。

```txt
scene playing3d {
state {
puzzle3 board = push3d
}
layout {
puzzle3 board
row {
button "Restart" -> board.restart
button "Levels" -> goto level_select
}
}
}
```

2D puzzle でも、model 自体に属する表示補助は `puzzle` 内の `render` block に書く。

```txt
puzzle sokoban {
render {
grid occupied_cells
}
}
```

`grid occupied_cells` は object が存在する cell の外周を表示する読み取り補助。`grid all_cells` にすると空セルも含めて全 cell に grid を表示する。どちらも floor や当たり判定を追加するものではなく、level、rule、win condition には影響しない。省略時は表示しない。

3D camera の初期 view と操作可否は scene ではなく 3D model の `render` block に書く。

```txt
puzzle3 push3d {
render {
camera yaw=34 pitch=38 zoom=1 interactive_look interactive_zoom
grid occupied_cells
viewport {
smoothscreen 7 7
focus Player
}
pixelate scale=4
shade
}
}
```

`camera yaw=34 pitch=38 interactive_look` のような inline group は、`camera { yaw = 34; pitch = 38; interactive_look }` と同じ意味である。bare option は有効化、値を持つ option は `key=value` で書く。`interactive_look` は pointer drag で視線方向を変える設定、`interactive_zoom` は wheel/pinch 系の zoom 操作を許す設定である。これは `input` 名ではなく、`puzzle3` component が自分の表示 box 内で始まった raw pointer gesture を camera view state に使ってよいという許可である。`zoom = 1` が `zoomscreen` / `smoothscreen` の通常倍率で、`zoom` や interactive zoom はその framing に対する上書き倍率として扱う。model `rules` の `if input == ...` には渡らず、undo/restart/transition state にも入らない。旧 `debug_camera` や `camera_yaw` 系、`interactive_look = true` のような boolean assignment は受け付けない。

`viewport { zoomscreen 7 7 }` は、親 scene から渡された display の `W x H` 枠に対して、focus object を中心にした `7 x 7 x full` の仮想 world-space box をどう描くかを決める。3D visual はその box を現在の camera yaw/pitch で投影し、与えられた display に収まる最大倍率にする。`full` は現在 level の全 height。`zoomscreen 7 7 3` と書くと高さも focus 周りの 3 cell として扱う。`smoothscreen` は同じ framing を目標にするが、描画用 view が遅れて追従する。`focus Player` は追従対象を指定する。これは描画 framing であり、外側 object の culling ではない。

Scene layout は `puzzle3` を固定 4:3 display として扱う。`puzzle3` は可変 window ではなく、その固定 display の内側に 3D visual を描く component である。scene は level の幅、focus object、`zoomscreen` の有無、投影後の見え方を layout 判断に使わない。`zoomscreen` は、親から渡された frame `W x H` と viewport 指定の cell frame `W cells x H cells` から、3D visual が display 内の描画位置と倍率を決める機能である。

3D model `rules` では `set yaw = <deg>` / `set pitch = <deg>` / `set zoom = <n>` を、rule 発火時の camera view-state 更新として書ける。`reset_camera` は camera view を `render { camera { ... } }` の初期値に戻す。これは盤面 state ではなく表示 command なので、solver、win condition、undo/restart の state には入らない。

`grid occupied_cells` は object が存在する cell の外周 edge を表示する読み取り補助。floor や volume の追加ではなく、level、collision、rule、win condition には影響しない。省略時は表示しない。

`pixelate` / `pixelate scale=4` は 3D canvas の描画後 pixel 化 postprocess を有効にする。`scale` は一度縮小する倍率で、省略時は `4`。省略時は pixel 化しない。

`render { shade }` は 3D sprite voxel の面ごとの明暗付けを有効にする表示設定。sprite data や puzzle state には影響しない。省略時も on。

3D object は `sprites3` に同名 sprite が定義されている場合だけ voxel sprite を描く。sprite 未指定の object に暗黙の cube や色は割り当てない。位置や占有を読みたい場合は `grid occupied_cells` などの debug 表示を使う。

`sprites3` の sprite entry は、object 名、色行、voxel rows の順に書く。色行だけなら 1x1x1 の filled cube sprite になる。再利用する voxel pattern は `shape <name> { ... }` で定義し、sprite entry 側では色行の次に bare shape ref だけを書く。

```txt
sprites3 simple {
Floor
#90ee90
}
```

Canonical な generic scene component は `title`、`subtitle`、`text`、`choice`、`button`、`row`、`column`、`box`、`for`、`level_menu`、`menu`。Model window component は `puzzle` と `puzzle3`。`layout` は component ではなく scene root layout block。`panel` は component keyword ではない。

`scene puzzle [name]` は puzzle state を主モデルに持つ playable scene を定義する。`name` を省略すると `playing` になる。中の `layers` は board/object layer、`layout` は画面配置を意味する。scene-local な puzzle slot を明示しない場合は、`<name>` state slot が暗黙に `puzzle <name>` として用意される。`board` は予約 slot 名ではない。明示した slot がある場合はそれが primary puzzle slot になり、`update <slot>` で現在 input をその puzzle transition に渡せる。

```txt
scene puzzle {
layout {
puzzle playing
}

layers {
actor = Player Box Wall
floor = Goal
}

rules {
once right [ Player | Box | no Wall ] -> [ | Player | Box ]
if win_conditions -> next_level
}

input right {
update board
}
}
```

`scene level_menu [name]` の typed scene template は読まない。level list は通常 scene の `layout` 内に `level_menu` component として置く。`show_index = <true|false>`、`show_solved = <true|false>`、`layout = list`、`columns = <n>`、`wrap = <true|false>`、`locked = disabled|hidden`、`button ...` などの option は `level_menu { ... }` 内に書く。

```txt
scene level_select {
layout {
level_menu {
show_index = true
show_solved = true
columns = 4
wrap = true
button "Title" -> goto title
}
}
}
```

`columns = <n>` は level item を n 列の matrix として配置する。`layout = list` は通常の縦 list。matrix では `left` / `right` が隣の item、`up` / `down` が列数ぶん前後の item に移動する。`wrap = true` は cursor の端越えを循環させ、`wrap = false` で無効にする。

```txt
scene play_level {
state {
puzzle sokoban
}
layout {
message = "Push the box"
sokoban
text message
}
}
```

現在の標準 component:

```txt
sokoban
text "Level clear"
text message
choice "Resume" -> resume
button "Title" -> goto title
box {
text message
button "Restart" -> playing.restart
}
row {
button "Title" -> goto title
button "Restart" -> playing.restart
}
column {
button "Restart" -> sokoban.restart
button "Level Select" -> goto level_select
}
level_menu {
show_index = true
}
```

Bare `sokoban` は scene state の puzzle slot を表示する。明示名を付けた advanced case では `puzzle sokoban1` のようにも書ける。

`text` は quoted text、scene state 変数、または `for` binding の path を表示する。

```txt
text "Paused"
text message
text level.label
```

`choice` と `button` は押されたときに input、component effect、scene-local routine、または scene effect を発行する。`choice` は標準 cursor で選ばれる主選択肢、`button` は補助操作である。旧 `button "Label" = name` と `choice "Label" action name` は読まない。`-> input <name>`、`-> component_effect <name>`、bare routine name、または direct scene effect を使う。bare routine name は同じ scene 内の `routine <name> { ... }` を要求する。

```txt
choice "Resume" -> resume
choice "Start" -> goto sokoban
button "Restart" -> playing.restart
button "Title" -> goto title
button level.label -> playing.goto level
```

`box` / `row` / `column` は layout component を入れ子にする layout primitive。`box` は純粋な配置用の矩形で、背景・枠線・装飾をデフォルトでは持たない。renderer はこれを HTML 固有の DOM ではなく、構造化された layout tree として受け取る。`panel` は layout primitive ではなく、canonical syntax では使わない。`layout` 直下の改行並びは暗黙の `column` なので、縦積みだけなら `column { ... }` は省略してよい。`size <w> <h>`、`gap <n>`、`align <x> [y]` は既存ファイル向けに読めるが、canonical authoring では default / theme に任せ、選択肢の縦・横・matrix 構造を優先して書く。標準 UI focus は `choice` の論理構造から決まるため、細かい座標ではなく `row` / `column` の論理構造を書く。

`for` は collection の各 item から layout node を生成する projection primitive。固定 component を並べる場合も、collection を表示する場合も、最終的には `row` / `column` の children として扱われる。`for` 自体は cursor、enter、scroll を所有しない。

level list の基本形は、通常の `scene` の `layout` に `level_menu { ... }` を置くこと。component が cursor 移動、enter、locked 表示、多すぎる項目の scroll を所有する。

```txt
scene level_select {
layout {
text "Select a level"
level_menu {
show_index = true
show_solved = true
button "Title" -> goto title
}
}
}
```

`level_menu` は level 選択用 component なので、`up` / `down` / `left` / `right` / `enter` の cursor 動作と、多すぎる項目の scroll を所有する。通常は key binding を書かなくてよい。既定では `w/a/s/d` と arrow keys が移動し、Enter/Space/x が選択 level を開始する。これは `level_menu` template の主動作なので、通常 `choose_level` transition のような中継は書かない。`level_menu` は inline source や `->` effect を取らない。表示する level の絞り込みは scene の `resources { levels ... }` で指定する。

この構文では旧 `show index`、`columns <n>`、裸の `wrap`、`action <name>` は読まない。`level_menu` を選んだ時点で enter は選択 level 開始を意味する。

level の開始、読み込み、restart は level scene / puzzle slot に対する effect として書ける。ただし通常の clear / advance / restart は level scene 内の model window component と puzzle lifecycle が所有する。scene からの target effect は、title/menu から入る、button で明示 restart する、hub から特定 level に飛ぶ、通常 clear とは別の例外 flow に入る、などの介入だけに使う。canonical な開始は `goto sokoban` または `goto sokoban(level_name)`。level 指定なしの `goto <level scene>` は保存済みまたは選択中の `current_level` を使い、なければ最初の level に入る。独自 scene なら `scene playing(level) { state { sokoban(level) } layout { sokoban } rules { step sokoban } }` として `goto playing(level)` で入る。旧 `start levels ... in <scene>` / `continue levels ... in <scene>` は読まない。`playing.restart` は playing scene の現在 level を初期状態に戻し、`playing.next_level` は playing scene を次 level で開始し、`playing.previous_level` は前 level で開始する。`playing.goto <level>` は指定 level で playing scene に移る。`board.restart` のように puzzle slot を target にした場合は、その puzzle state を初期状態に戻す。`board.next_level` はその puzzle を所有する level scene を進める。

### `keys`

```txt
keys {
d ArrowRight -> right
a ArrowLeft -> left
r -> restart
}
```

`keys` は owner-scoped な raw key から semantic input、scene routine、または scene effect への対応表。puzzle 内では puzzle rules が読む semantic input に変換し、scene 内では scene-local routine や明示 effect を呼ぶ。

```txt
<key> [<key> ...] -> <input-or-routine-or-effect>
```

通常文字は `d` のように書く。特殊キーは `ArrowRight` / `ArrowLeft` / `ArrowUp` / `ArrowDown` / `Enter` / `Space` / `Escape` / `Tab` / `Backspace` のように名前で書く。`r -> restart` は model default mapping だが、`r -> my_restart` のように同じ key を別 input に割り当てると、その key は `restart` ではなく自作 input として解釈される。

scene では bare identifier の RHS は scene-local `routine` 呼び出しとして扱う。`q Escape -> level_select` は複数 key から `routine level_select` を呼び、`Escape -> goto title` は key から直接 scene effect を実行する。`keys` では `=` を使わない。

```txt
scene title {
keys {
Enter Space -> confirm
Escape -> goto title
}
button "Play" -> input confirm
routine confirm {
goto playing
}
}
```

button click も `button "Play" -> input confirm` のように semantic input を出せる。keyboard confirm を同じ semantic input として扱いたい場合は `keys { Enter Space -> input confirm }` と明示する。key から単に named effect sequence を実行したい場合は `keys { Enter Space -> confirm }` と `routine confirm { ... }` を使う。

```txt
my_restart -> restart
```

model `rules` の `<input> -> <effect>` は `if input == <input> { <effect> }` の sugar。model rules 内に `restart` input handler がなければ、default として `restart -> restart` が追加される。model rules から固定 scene node へ移る場合は `goto <scene>` または `start <scene>` を effect として直接書ける。scene key dispatch は `rules` ではなく `keys` と `routine` で書く。

scene が level lifecycle に介入したい場合は、button や scene transition から `playing.restart` / `board.restart` のように target を明示する。これは通常進行の書き方ではなく、ユーザー操作や特殊 flow のための escape hatch である。

クリア表示を標準の level advance から分離したい場合は、別の固定 scene node へ `goto level_clear` する。盤面の上に一時 popup を出したい場合は scene navigation ではなく、現在 scene の state と component 表示で表す。

```txt
scene playing {
state {
puzzle sokoban
}
layout {
sokoban
}
rules {
step sokoban
if sokoban.needs_manual_clear -> goto level_clear
}
}

scene level_clear {
layout {
box {
text "Level clear"
button "Next Level" -> playing.next_level
button "Restart" -> playing.restart
button "Level Select" -> goto level_select
}
}
}
```

## Comments

`//` 以降はコメントとして扱われる。

`#` は level の壁など通常文字として使えるため、コメント記号にはしない。
