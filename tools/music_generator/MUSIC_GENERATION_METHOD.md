# 次世代 Music Generator 仕様案

この文書は、現行実装の説明ではなく、次に作る music generator の仕様案である。

目的は、コードを読まなくても生成方法を再現できるくらい、生成原理を薄く圧縮すること。
そのために、音楽的な印象を後から補修するのではなく、最初から再現可能な確率モデルとして定義する。

## 目的

この generator は、AI が知識で作曲するものではない。

人間が定義した小さな生成文法と確率分布の中を、seed が探索する仕組みにする。

したがって、信頼できる説明は次の形でなければならない。

> この順番で latent variables を sample し、その値から motif / section /
> bar / event を deterministic に展開する。

文書だけで同じ generator を再実装できないなら、generator 側がまだ複雑すぎる。

## 基本原理

生成は次の 5 層だけにする。

1. `SongWorld`
2. `MotifSet`
3. `SectionPlan`
4. `BarProjection`
5. `Rendering`

各層は、前の層からだけ作られる。
下位層が上位層を勝手に引き直してはいけない。

```txt
seed
  -> SongWorld
  -> MotifSet
  -> SectionPlan
  -> BarProjection
  -> Rendering
```

この順序が generator の説明性そのものになる。

## 乱数の扱い

乱数は、各層の latent variables を sample するためだけに使う。

bar event を作る途中で、その場の都合で新しい音楽判断を追加しない。
bar で使う乱数は、上位層で決まった motif と transform を揺らすための局所 jitter に限る。

乱数 stream は用途別に分ける。

```txt
styleSeed       -> scale, tempo range, timbre family
compositionSeed -> motif set, section plan, harmony path
realizationSeed -> bar-level jitter
```

同じ stream を複数の意味に使わない。
これにより、ある要素を変更しても別の層の乱数列が崩れにくくなる。

## SongWorld

`SongWorld` は曲全体の物理法則である。

ここで決めるもの:

- scale field
- harmonic space
- tempo
- meter
- global density range
- register ranges
- timbre fields
- available renderers

ここでは melody や section を作らない。
この層は「この曲世界で使える材料と座標系」を決めるだけ。

### scale field

scale は固定リストから選ぶが、以後の pitch は scale degree 空間で扱う。

```txt
pitch = tonic + scaleDegreeToSemitone(degree) + octave
```

pitch の乱数は直接 MIDI note を選ばない。
必ず degree / register / contour から導く。

### timbre field

timbre は named preset ではなく、seeded spectral field から生成する。

ただし、timbre 生成の仕様は composition 仕様から分ける。
composition は `rendererId` と `registerRange` を持ち、実際の音色パラメータは Rendering 層で決まる。

## MotifSet

`MotifSet` は曲中で再利用される musical identity の集合である。

motif は次の 4 要素で定義する。

```txt
Motif {
  rhythmCell
  accentProfile
  contourCell
  transformFamily
}
```

motif は bar の中で作らない。
section の中でも作らない。
曲の上位層で先に作る。

### rhythmCell

`rhythmCell` は motif の最重要同一性である。

形式:

```txt
rhythmCell = [0..1 の normalized onset positions]
```

例:

```txt
[0.08, 0.34, 0.72]
```

bar に投影するときは、

```txt
step = round(barStart + onset * usableSpan)
```

とする。

同じ motif family 内では、rhythmCell は原則として変えない。
変奏は主に pitch / register / duration / density で行う。

rhythm を変える場合も、別の rhythmCell を作るのではなく、`transformFamily` の中の
限定された transform として扱う。

### accentProfile

`accentProfile` は motif の中でどの onset が前景になるかを決める。

形式:

```txt
accentProfile = [0..1 の weights]
```

`rhythmCell` と同じ長さを持つ。

accent は velocity だけではなく、duration や register の安定性にも影響してよい。

### contourCell

`contourCell` は pitch の相対形である。

形式:

```txt
contourCell = relative degree offsets
```

例:

```txt
[0, +2, +1]
```

これは absolute pitch ではない。
bar や section の register transform によって実音高へ写像される。

### transformFamily

`transformFamily` は motif をどう変奏してよいかの分布である。

含むもの:

- transposition distribution
- register shift distribution
- duration stretch distribution
- density thinning distribution
- ornament probability
- rhythm deformation limit

重要なのは、変奏可能性も motif の一部として先に定義すること。
bar 側が勝手に「今回はこう変える」と決めない。

## SectionPlan

`SectionPlan` は、曲全体の時間軸に motif と transform を配置する。

section は named form ではない。
section は次の値を持つ。

```txt
Section {
  motifId
  energy
  density
  registerCenter
  tension
  closure
  transformBias
}
```

section は motif を生成しない。
section は motif を選び、その motif にどの transform bias をかけるかを決める。

### motifId の遷移

motifId は低周波の Markov process で決める。

入力:

- current motifId
- section index
- phrase position
- energy slope
- closure

出力:

- stay
- switch to related motif
- return to previous motif

ただし section index から形を直接決めない。
同時に、各 section が独立に motif を選ぶわけでもない。

仕様としては、遷移確率だけを定義する。

```txt
P(stay)   = high when closure is low and phrase position is internal
P(switch) = higher near contrast points
P(return) = higher near closure points
```

この確率式から結果が出る。
結果の形を直接指定しない。

## BarProjection

`BarProjection` は、section が持つ motif と transform を 1 bar の event へ写像する。

bar は作曲判断をしない。
bar は projection だけを行う。

入力:

```txt
SongWorld
Motif
Section
barIndex
localPhrasePosition
realizationSeed
```

出力:

```txt
Event[]
```

### projection 手順

1. `rhythmCell` を usable step range に投影する。
2. `accentProfile` から velocity / duration emphasis を決める。
3. `contourCell` を section の registerCenter と transformBias に写像する。
4. density thinning を適用する。
5. ornament を追加する。
6. renderer に必要な event fields を埋める。

この順序を守る。

特に、rhythm projection より前に bar 独自の onset を作らない。
onset の新規追加は ornament として扱い、motif 本体とは分ける。

## Renderer

現行の `carrier` は、次世代仕様では renderer に近い。

renderer は motif をどう鳴らすかを決める。

例:

- lead renderer
- bass renderer
- arp renderer
- percussion renderer
- pad renderer

renderer は motif を作らない。
renderer は event を音域・track・timbre へ写像する。

これにより、`bass-riff` のような名前は generator の中心概念ではなくなる。

```txt
motif + renderer = audible part
```

renderer が違っても、同じ motif を鳴らせる。
同じ renderer でも、別 motif を鳴らせる。

## 生成順序

完全な生成手順は以下。

```txt
1. split seed into streams
2. sample SongWorld
3. sample MotifSet
4. sample SectionPlan
5. for each section:
     for each bar:
       project selected motif into events
6. render events through renderer/timbre fields
7. return playbackScore + debug
```

この 7 ステップで説明できない処理は、原則として設計を見直す。

## debug data

debug は「説明性」の一部なので、生成層と同じ構造で出す。

必要な debug:

```txt
songWorld
motifSet
sectionPlan
barProjections
rendererAssignments
randomStreams
```

特に `motifSet` は必須。
聴こえる motif が debug 上でも見えなければ、説明可能とは言えない。

## tests

テストは音質ではなく、生成文法の不変条件を守る。

必要なテスト:

- same seed produces same `SongWorld`, `MotifSet`, `SectionPlan`, and events
- different seed changes at least one upper-layer latent variable
- renderer does not create motif identity
- same motifId preserves rhythmCell under projection
- different motifId usually changes rhythmCell
- section can change transform without changing motif
- bar projection does not create new primary onsets outside motif/ornament rules
- timbre field is generated, not selected from a small preset table

これらは「いい曲」のテストではない。
説明可能な generator であることのテスト。

## 現行実装との差分

現行実装で問題になる点:

- role/carrier が中心概念になっている
- motif が event 生成の途中で参照される補助情報になっている
- rhythm / contour / accent が明示的な上位 object として debug に出ていない
- section state の値が多くの局所式に散っている
- carrier ごとの branch が生成文法に見えてしまう
- tests が不変条件より症状検出に寄っている

移行方針:

1. `MotifSet` を先に作る。
2. identity event 生成を motif projection に置き換える。
3. carrier を renderer に格下げする。
4. section state を `Section` の transform bias に整理する。
5. debug に `motifSet` と `barProjections` を出す。
6. tests を projection invariants に移す。

## この仕様の信頼 claim

この仕様が実装できた場合、generator について次のように言える。

> この generator は、hand-authored な小さな生成文法と確率分布を持つ。
> seed はその文法上の latent variables を sample し、以後は deterministic
> projection によって event を作る。AI model は runtime に存在せず、完成済み
> phrase のライブラリも持たない。

この claim を文書だけで検証できることが、この仕様の目標である。

## まだ決めていないこと

以下はまだ仕様化できていない。

- harmony を motif と同じ粒度で扱うか、別の harmonic path として扱うか
- percussion identity を rhythmCell とどう統合するか
- ornament と primary onset の境界
- section transition を motif transform として表す方法
- listener にとって memorable かどうかをどう評価するか

ここを曖昧にしたまま実装すると、また局所補修が増える。
