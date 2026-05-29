# PuzzleScript 3D Extraction Plan

この文書は、このリポジトリから独立した PuzzleScript 3D 拡張を作るための状況認識、障害、仕様固定方針、当面の実装計画を記録する。

## Goal

最終的な価値は、この実装そのものではなく、PuzzleScript に接続できる 3D 拡張の canonical syntax を定めることである。

このプロジェクトは最終的に残らなくてもよい。残すべきものは次の三つである。

- PuzzleScript 由来の作者が理解できる、狭く安定した 3D 言語仕様
- その仕様を検証できる小さな runtime / editor / examples
- 後続実装が同じ解釈に従える reference behavior

当面は二本建てで進める。

1. 個人プロジェクトとしてのニッチな `.puzzle` 系仕様を、このリポジトリ内で必要な範囲まで進める。
2. PuzzleScript Next + 3D として切り出せる仕様・runtime・editor 面を分離し、コミュニティとの摩擦を減らす。

## Design Additions

既存の中心原則は「3D は 2D に二つの空間方向を足したもの」である。
以下はその補助原則であり、3D 化によって仕様面積や実装分岐が増えすぎることを防ぐための判断基準とする。

### Readable Space Before Expressive Space

3D 化の価値は、複雑な空間を作れることだけではない。PuzzleScript の強さは、盤面状態、移動、衝突、ルール結果を作者とプレイヤーが読めることにある。

したがって 3D syntax、renderer、camera、editor は、まず「現在の空間関係を誤解なく読めるか」で評価する。

帰結:

- dense 3D pattern や oriented frame は表現力が高くても core tutorial の中心に置かない。
- camera、occlusion、clipping、slice display は gameplay state を隠さない方向で設計する。
- debug view、slice view、cell inspector、movement trace は単なる補助 UI ではなく、3D 仕様を検証する観測面として扱う。
- renderer option は rule semantics を変えてはならないが、状態の観測可能性を壊す option も core には入れない。

### Minimize New Learning For PuzzleScript Authors

Canonical syntax は、便利さよりも PuzzleScript 作者が既存の mental model から推測できることを優先する。

新しい構文を core に入れる前に、「これは 2D PuzzleScript のどの概念を 3D に拡張したものか」を説明できなければならない。

帰結:

- `three_dimensions` + ordinary `LEVELS` を author-facing の基本形にする。
- `LEVELS3`、`puzzle3`、`sprites3` のような別言語感の強い名前は canonical にしないか、互換・内部・experimental に留める。
- `left/right/front/back/up/down` は core に置けるが、frame rotation は advanced として扱う。
- 表面文法が増えるときは、実装都合ではなく PS 作者の追加学習量で採否を判断する。

### Dimension Hooks, Not Feature Forks

3D 固有差分は feature fork ではなく dimension hook として現れるべきである。

3D 側に非空間 feature の独自実装が増えた場合、それは 3D runtime の成熟ではなく、共有 PuzzleScript semantics が漏れている兆候として扱う。

帰結:

- `neighbor`、`coordToIndex`、direction table、movement resolution、rule frame、renderer projection は dimension hook として持てる。
- command queue、late phase、random choice、win condition、undo、checkpoint、loop/gosub は dimension hook ではない。
- 3D 実装で非空間 feature 名が増えたら、共有層へ戻すか、2D と同じ contract に抽出する。

### Narrow Promise, Not Merely Small MVP

MVP は機能数が少ないことではなく、公開する約束が狭く、検証可能で、保守できることを意味する。

実装済みでも、説明・テスト・editor・export の contract が揃っていないものは core ではない。

帰結:

- advanced / experimental は実装されていても public promise にしない。
- core に入れた機能は runtime、editor preview、standalone export、docs、diagnostics、examples のどこで何を保証するかを明記する。
- 「動くが PS 作者に説明しにくい」ものは、canonical ではなく experimental として扱う。

## Current Situation

このリポジトリには、既に 3D モデル専用 crate がある。

- `crates/puzzle3d_model` は 3D state、directions、frame、3D parser、transition、level、sprite、session を持つ。
- `crates/lang` は混在 `.puzzle` document の parsing、2D parser、PuzzleScript import、scene routing、highlight/completion を持つ。
- `crates/html_play` と `crates/html_editor/static/*` は editor UI、preview、3D level editor、3D sprite editor、standalone HTML runtime を持つ。
- root `editor.html` は generated artifact なので直接編集しない。

重要な現状判断:

- 3D core は「完全に未分離」ではない。問題は crate 境界より、表面文法・editor contract・HTML runtime・公開仕様がまだこのプロジェクトの `.puzzle` 世界に接続されていること。
- PuzzleScript import は存在するが、現在は PS を canonical `.puzzle` へ変換する補助であり、PS 互換文法を中心に据えた言語ではない。
- 3D syntax の一部は既に動いている。特に `left/right/front/back/up/down`、frame prefix、dense 3D pattern、`render { camera ... viewport ... shade ... }` は実装済みまたは仕様化済みの土台がある。
- editor UI は再利用可能だが、editor が扱う document model と runtime control surface を PS 3D 用に薄くしないと、独立プロジェクトとして保守しにくい。

## Main Obstacles

### 1. Language Identity

最大の障害は実装量ではなく、「これは PuzzleScript の拡張なのか、このプロジェクト独自言語なのか」が曖昧になること。

今の `.puzzle` は scene、theme、assets、sound、level menu、display routines、vars、lifecycle などを含む広い言語になっている。PS 3D として切り出すなら、最初の public contract はもっと狭くする必要がある。

原則:

- PS 互換の基本構造を中心に置く。
- 3D は独立した拡張モジュールとして足す。
- scene/editor convenience は canonical syntax の中心に置かない。
- 便利でも PS author が「別言語を覚えさせられている」と感じる構文は避ける。

### 2. Canonical Syntax Drift

既存実装には、動くが canonical にすべきか未確定の構文が混ざっている。

特に判断が必要な領域:

- `puzzle3` / `levels3` / `sprites3` という名前を PS 3D でも使うか
- 3D dense pattern `[ A | B ; C | D ;; E | F ; G | H ]` を標準に入れるか、advanced に隔離するか
- `frame` prefix を author-facing にするか、まずは advanced にするか
- camera / viewport / shade / pixelate を PS の metadata section 風にするか、現行 `render { ... }` 風にするか
- 2D `zoomscreen` / `smoothscreen` の 3D override を canonical にするか、renderer option に閉じるか

原則:

- 最初の canonical は「Sokoban in 3D を自然に書ける」範囲に絞る。
- 3D 固有でない機能は PS 側の既存 mental model に寄せる。
- advanced 機能は仕様から消すのではなく、core / advanced / experimental に分けて公開する。
- core に入れる構文は、2D PuzzleScript のどの概念を 3D に拡張したものか説明できる必要がある。

### 3. PuzzleScript Semantics Compatibility

PuzzleScript は表面文法だけでなく、rule application、movement、late、again、winconditions、legend/property/aggregate、collision layer の意味が重要。

現在の import は限定的で、PS 互換そのものではない。PS 3D を名乗るなら、最低限どこまで互換にするかを決める必要がある。

障害:

- PS の `again` とこのプロジェクトの canonical `again` は意味が完全には一致しない。
- PS の property / aggregate / synonym / collision layer と、現在の selector / group / layer model は近いが同一ではない。
- PS の rule ordering と movement resolution を 3D に広げると、単純な方向追加以上の設計になる。
- 既存 `.puzzle` の scene/lifecycle が強すぎると、PS 互換の単純さを壊す。

切り分け:

- `PS2D compatibility`: 既存 PS をどれだけ読めるか。
- `3D extension`: 6方向、3D level、3D matching、camera をどう足すか。
- `Studio extras`: editor、scene、theme、assets、export をどこまで載せるか。

### 4. Editor Coupling

editor UI は再利用したいが、現状はこのリポジトリの `.puzzle` document model と密に接続している。

障害:

- highlight/completion は Rust-owned に寄せている途中で、完全な public syntax service になっていない。
- 3D preview は runtime control contract を持ち始めているが、editor 側にはまだ project/workspace/scene/model 前提が残る。
- PS import UI は「PS から `.puzzle` へ変換」なので、PS 3D の native editor とは方向が違う。
- standalone HTML runtime と editor preview runtime の差分を増やすと、reference behavior が壊れる。

原則:

- editor は言語仕様の owner ではない。
- editor が必要とする面は small API にする。
- PS 3D では、最初は single-file editor + preview + export に限定する。

### 5. Renderer And Runtime Split

3D runtime は Rust model と HTML renderer の両方にまたがる。仕様を open source にするなら、どちらが reference behavior かを決める必要がある。

障害:

- 3D state / transition は Rust にあるが、rendering、camera、pixelate、interactive look は HTML 側にもある。
- camera は puzzle state ではなく presentation state なので、solver / replay / undo との境界を間違えると仕様が濁る。
- shade、shadow、3D tween は未実装または不完全で、今 canonical に入れると実装責務が膨らむ。

原則:

- Rule semantics は Rust reference。
- Rendering semantics は最初は HTML reference でもよいが、puzzle state と切り離す。
- `shade` / `shadow` / `tween` は gameplay semantics に影響しない presentation option として扱う。
- renderer は見た目だけでなく観測契約を持つ。camera や display option は rule semantics を変えないが、状態を読めなくする option は core promise に入れない。

### 6. Open Source Readiness

現 workspace の license は `UNLICENSED`。open source 化には技術以外の整理が必要。

障害:

- ライセンス未決定。
- generated artifacts と source owner の区別が必要。
- third-party notices、theme/assets、WASM build outputs、docs publishing の境界確認が必要。
- 既存 `.puzzle` 独自機能を全部公開 API にすると、保守面積が大きすぎる。

原則:

- 最初に公開するのは full studio ではなく、PS 3D spec + minimal reference implementation。
- generated output は release artifact として扱い、source と混同しない。
- 実装維持が難しい機能は experimental と明記する。

## Proposed Product Shape

### Package A: Personal Puzzle Studio

このリポジトリで継続する個人実験ライン。

- `.puzzle` の広い文法を維持する。
- scene、theme、assets、level menu、AI/editing support などを含める。
- 3D PS 仕様の実験場として使う。
- ただし PS 3D canonical syntax と混同しない。

### Package B: PuzzleScript 3D Reference

切り出し先の候補。

- PS-compatible source format を入口にする。
- 3D extension は明示的に module / section として足す。
- UI は現 editor を簡略化して再利用する。
- export は standalone HTML を優先する。
- CLI は `check`, `play/export`, `translate` 程度に絞る。

## Canonical 3D Syntax Draft

この節は現時点の仮仕様。実装済みかどうかではなく、公開する価値があるかで分類する。

### Core

#### Directions

3D directions は次の六つを標準名にする。

```txt
left right front back up down
```

方向の意味:

- `left` / `right`: X axis
- `front` / `back`: depth axis
- `up` / `down`: height axis

`forward` / `backward` は互換 alias に留め、canonical にはしない。

#### Direction Sets

```txt
directions = left right front back up down
horizontal = left right front back
vertical = up down
```

通常移動は `horizontal` を基本にする。重力や階段系 puzzle でのみ `vertical` / `directions` を使う。

#### Relative Directions

3D matching 内の相対方向は次を採用候補にする。

```txt
> < ^ v o x
```

暫定意味:

- `>`: primary positive
- `<`: primary negative
- `^`: secondary positive
- `v`: secondary negative
- `o`: depth positive
- `x`: depth negative

未決定点:

- `o` / `x` は読みにくい可能性がある。
- PS 由来の `^ v < >` に比べて 3D 軸が増えるため、frame 指定なしで使うと混乱しやすい。
- core では direction prefix を優先し、relative markers は advanced に置く選択肢がある。

#### 3D Levels

3D ASCII は layer/slice を空行で分ける方式を第一候補にする。

```txt
level demo {
#####
#P B#
#####

,,,,,
,,G,,
,,,,,
}
```

原則:

- `.` は empty として予約する。
- 同じ level 内の全 slice は同じ width/depth を持つ。
- blank line は 2D の level separator ではなく、3D slice separator として解釈される。
- level entry の外側でのみ次の level へ進む。

### Advanced

#### Dense 3D Matching

候補 syntax:

```txt
[ A | B ; C | D ;; E | F ; G | H ]
```

意味:

- `|`: row 内の cell 区切り
- `;`: row 区切り
- `;;`: depth slice 区切り

判断:

- 表現力は高い。
- ただし PS author には重い。
- core には入れず、advanced matching として明示するのが安全。

#### Frames

候補 syntax:

```txt
up:back:down [ > Player | ^ Box | o Box ] -> ...
```

現実的な読み替え:

```txt
<primary>:<secondary>:<depth> [ ... ]
```

原則:

- frame は 3D dense / relative matching の解釈 frame を指定する。
- `A:B` は第三軸を canonical chirality で補完してよい。
- `A:B:C` は完全指定。
- `frames` / `canonical` / `mirrored` のような全展開系は advanced / experimental に留める。

懸念:

- PS の単純な directional rule と比べて難度が高い。
- frame が必要になる puzzle は高度なので、最初の community-facing tutorial には出さない。

### Presentation

#### Camera

canonical 候補:

```txt
render {
  camera {
    yaw = 34
    pitch = 38
    zoom = 1
    interactive_look = true
    interactive_zoom = true
  }
}
```

原則:

- camera は puzzle state ではない。
- undo / restart / solver key / win condition に入れない。
- rule からの `set yaw`, `set pitch`, `set zoom`, `reset_camera` は presentation emission として扱う。

#### Zoomscreen / Smoothscreen

canonical 候補:

```txt
render {
  viewport {
    zoomscreen 7 7
    focus Player
  }
}
```

3D での意味:

- `zoomscreen w d` は focus 周りの `w x d x full-height` frame を画面に収める。
- `zoomscreen w d h` は height も指定する。
- `smoothscreen` は同じ framing を遅れて追従する。
- culling ではなく framing。外側 object は消さない。

#### Shade / Shadow / Pixelate / Tween

分類:

- `shade`: candidate core presentation option
- `pixelate`: candidate core presentation option
- `shadow`: experimental
- `3D tween`: experimental

原則:

- いずれも rule semantics に影響しない。
- canonical syntax に入れる前に、見た目だけでなく export/replay/editor preview の一貫性を確認する。

## Minimal PS 3D MVP

最初の公開可能ラインは、次を満たせばよい。

- PS-like source を single file で書ける。
- Objects / Legend / CollisionLayers / Rules / WinConditions / Levels 相当がある。
- 3D directions と 3D levels がある。
- 3D Sokoban が 2-3 levels 動く。
- editor UI で edit -> preview -> export ができる。
- canonical examples と diagnostics がある。
- advanced frame / dense pattern は、動いても tutorial の中心にしない。

MVP の意味:

- 「少ない機能」ではなく「狭い public promise」として定義する。
- core として公開する機能は、runtime / editor preview / standalone export / docs / diagnostics / examples の保証範囲を明記する。
- 実装済みでも、観測可能性・説明可能性・互換性の contract が揃っていないものは advanced または experimental に置く。

入れないもの:

- full scene system
- full personal `.puzzle` feature set
- solver
- shadow
- 3D tween
- arbitrary JS gameplay extension
- rich project workspace

## Implementation Plan

### Phase 0: Spec Freeze Draft

- PS 3D の対象範囲を `core`, `advanced`, `experimental` に分ける。
- 3D directions、level slice syntax、camera syntax、viewport syntax の canonical examples を決める。
- 既存 `.puzzle` から持ち込まない機能を明記する。
- `games/spec_3d.puzzle` とは別に、PS 3D 用の最小 example を作る。

### Phase 1: Thin PS 3D Frontend

- 既存 `crates/puzzle3d_model` を reference 3D model として使う。
- PS-like parser / translator を `.puzzle` 広域 parser から分ける。
- まずは PS 3D source を既存 3D model IR へ lower する。
- `.puzzle` personal syntax と PS 3D syntax の diagnostics を混ぜない。

### Phase 2: Minimal Editor Mode

- 現 editor UI から single-file source, preview, export, examples だけを残した PS 3D mode を作る。
- PS import tab は「PS 2D -> canonical `.puzzle`」ではなく、「PS 3D source native preview」に置き換えるか別扱いにする。
- 3D preview は `PuzzleStudioUpdatePuzzle3Preview` 系の explicit control contract だけを使う。
- editor が scene/model internals を再構成しないようにする。

### Phase 3: Runtime / Export Reference

- standalone HTML export を reference playable artifact にする。
- camera、viewport、pixelate、shade の runtime behavior を docs と tests で固定する。
- undo / restart / level advance が PS 3D semantics と矛盾しないことを確認する。
- renderer-only state と puzzle state の境界を docs に書く。

### Phase 4: Open Source Preparation

- License を決める。
- `THIRD_PARTY_NOTICES.md` と bundled assets / themes / wasm outputs を確認する。
- generated artifact と source artifact の扱いを明記する。
- README を PS 3D の最小価値に絞る。
- contribution scope を狭く書く。保守できない機能を public promise にしない。

## Decision Checklist

公開前に決める必要があること:

- `puzzle3` / `levels3` / `sprites3` を PS 3D canonical 名にするか。
- PS section names を大文字のまま拡張するか、小文字/braced syntax に寄せるか。
- `front/back` と `forward/backward` の関係をどう説明するか。
- 3D level slice separator を blank line に固定するか、明示 marker も許すか。
- dense 3D matching を core に入れるか advanced にするか。
- frame syntax を最初から公開するか hidden advanced にするか。
- camera syntax を `render { camera { ... } }` にするか、PS metadata style にするか。
- `shade`, `pixelate`, `shadow`, `tween` の公開レベルをどう分けるか。
- 既存 `.puzzle` scene system を PS 3D reference に含めるか、export/editor internal に落とすか。
- open source license と repository 名。

## Recommended Next Step

次にやるべきことは実装ではなく、`3D_SPEC.md` を作り、core syntax だけを 1-2 ページで固定すること。

その spec には、少なくとも次を含める。

- 3D directions
- 3D level slices
- minimum object/layer/legend/rule/win syntax
- camera/viewport presentation settings
- one complete 3D Sokoban example
- advanced/experimental に回した機能一覧

この spec ができると、実装の切り出し判断は「現コードをどこまで再利用するか」ではなく、「canonical PS 3D behavior を満たす最小実装は何か」に変わる。
