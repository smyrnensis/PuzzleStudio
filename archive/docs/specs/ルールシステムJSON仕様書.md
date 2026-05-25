# ルールシステム JSON 仕様書

> 詳細な型定義は `docs/フレームワーク計画.md` セクション 5.4 を参照。
> サンプルゲームデータは `docs/フレームワーク計画.md` セクション 9.1 を参照。

---

## 1. 概要

本仕様書は、ターンベース・グリッドベースパズルゲームフレームワークにおけるルールシステムの JSON 形式を定義する。ルールは「パターンマッチングと置換」の原理に基づき、現在の盤面状態から特定のパターンを検索し、新しいパターンで置き換えることでゲーム状態を更新する。

---

## 2. ルールの基本構造

### 2.1 全体構造

ルールは `GameData` の `ruleGroups` フィールドに定義される。各ターンで実行するルールグループの順序は `turnSequence` で指定する。

```json
{
  "ruleGroups": [
    {
      "name": "player_movement",
      "application": "once",
      "rules": [...]
    },
    {
      "name": "physics",
      "application": "until_stable",
      "rules": [...]
    }
  ],
  "turnSequence": ["player_movement", "physics"]
}
```

### 2.2 ルールグループ

```typescript
interface RuleGroup {
  readonly name: string;
  readonly application: ApplicationMode;
  readonly rules: readonly Rule[];
}
```

| フィールド | 型 | 必須 | 説明 |
|:---|:---|:---|:---|
| `name` | `string` | ○ | グループ名。`turnSequence` や `call` エフェクトで参照される。 |
| `application` | `"once" \| "until_stable"` | ○ | グループの適用方式。 |
| `rules` | `Rule[]` | ○ | グループ内のルール配列（配列順に実行）。 |

### 2.3 ルール

```typescript
interface Rule {
  readonly name?: string;
  readonly direction: RuleDirection;
  readonly application?: ApplicationMode;
  readonly patterns: readonly PatternBlock[];
  readonly conditions?: readonly Condition[];
  readonly effects?: readonly Effect[];
}
```

| フィールド | 型 | 必須 | 説明 |
|:---|:---|:---|:---|
| `name` | `string` | × | デバッグ用のルール名。 |
| `direction` | `RuleDirection` | ○ | パターンの方向指定。 |
| `application` | `"once" \| "until_stable"` | × | ルールの適用方式。デフォルトは `"once"`。 |
| `patterns` | `PatternBlock[]` | ○ | パターンブロックの配列。複数ブロックはすべてマッチする必要がある。 |
| `conditions` | `Condition[]` | × | グローバル変数に対する条件（AND 結合）。 |
| `effects` | `Effect[]` | × | 適用後に実行されるエフェクト。 |

### 2.4 実行順序

`turnSequence` で指定されたルールグループ名の順序に従って実行される。各グループ内のルールは配列の先頭から末尾へ順次処理される。

---

## 3. パターン定義

### 3.1 パターンブロック

パターンは 2D 配列で表現される。位置はインデックスから暗黙的に決まる（明示的な座標指定は不要）。

```typescript
interface PatternBlock {
  readonly before: readonly (readonly PatternCell[])[];  // [row][col]
  readonly after: readonly (readonly AfterPatternCell[])[];  // [row][col]
}
```

**before** と **after** は同じサイズの 2D 配列でなければならない。

**JSON 例（横1行のパターン）:**

```json
{
  "patterns": [{
    "before": [[
      { "objects": ["Player"] },
      { "noObjects": ["@Solid"] }
    ]],
    "after": [[
      { "objects": [] },
      { "objects": ["Player"] }
    ]]
  }]
}
```

**JSON 例（縦2行のパターン）:**

```json
{
  "patterns": [{
    "before": [
      [{ "noObjects": ["@Solid"] }],
      [{ "objects": ["@Falling"] }]
    ],
    "after": [
      [{ "objects": ["@Falling"] }],
      [{ "objects": [] }]
    ]
  }]
}
```

### 3.2 direction（方向）

ルールレベルで指定する（パターンブロックごとではない）。

```typescript
type RuleDirection =
  | "none"       // 方向の区別なし（1通り）
  | "up" | "down" | "left" | "right"  // 特定の1方向（1通り）
  | "vertical"   // 上下の2方向（2通り）
  | "horizontal" // 左右の2方向（2通り）
  | "any";       // 4方向すべて（4通りのバリアントを自動生成）
```

基準方向は **right**（パターンは右向きに記述されていると仮定）。`direction: "any"` の場合、エンジンがパターンを自動回転して 4 方向分のマッチングを行う。

### 3.3 パターンセルの条件

before ブロック内のセルには 3 種類の条件を指定できる：

```typescript
interface PatternCell {
  readonly objects?: readonly string[];    // 置換対象
  readonly hasObjects?: readonly string[]; // 存在確認のみ
  readonly noObjects?: readonly string[];  // 不存在確認
}
```

| キー | 役割 | 説明 |
|:---|:---|:---|
| `objects` | **置換対象** | そのセルに指定オブジェクトが存在することを要求し、after ブロックで置換される |
| `hasObjects` | **チェックのみ** | 存在確認。置換対象にはならない |
| `noObjects` | **チェックのみ** | 不存在確認。指定オブジェクトが存在しないことを要求 |

after ブロックのセルは置換先のみ：

```typescript
interface AfterPatternCell {
  readonly objects?: readonly string[];  // 置換先オブジェクト。空配列は「消す」
}
```

### 3.4 オブジェクト指定

パターン内のオブジェクトには以下の指定方法がある：

```json
// 具体的なオブジェクト名（タグ付き）
["Player:right"]

// 名前のみ（タグは任意にマッチ）
["Player"]

// タグバインディング: before でマッチしたタグ値を after で参照
["Player:$color"]

// 相対方向指定（ルールの回転に追従して変換される）
["Player:>"]

// プロパティによる指定（@プレフィックス）
["@Movable"]

// 複数オブジェクト（同一セル内の複数オブジェクトをマッチ）
["Box", "Wall"]
```

### 3.5 絶対方向と相対方向

- `>`, `<`, `^`, `v` は**相対方向**タグ。ルールの回転に追従して方向が変わる
- `right`, `left`, `up`, `down` は**絶対方向**タグ。ルールを回転しても方向は変わらない

| 元のタグ | right (0°) | down (90°) | left (180°) | up (270°) |
|:---|:---|:---|:---|:---|
| `>` (右) | `>` | `v` | `<` | `^` |
| `<` (左) | `<` | `^` | `>` | `v` |
| `^` (上) | `^` | `>` | `v` | `<` |
| `v` (下) | `v` | `<` | `^` | `>` |

### 3.6 複数パターンブロック

`patterns` 配列に複数のブロックを並べることで、1 つのルールで盤面上の複数箇所を同時にマッチ・置換できる。

**重要**: すべてのパターンブロックがマッチした場合のみ、すべての after パターンが同時に適用される。

```json
{
  "direction": "any",
  "patterns": [
    {
      "before": [[
        { "objects": ["Player"] },
        { "objects": ["Button"] }
      ]],
      "after": [[
        { "objects": [] },
        { "objects": ["Player"] }
      ]]
    },
    {
      "before": [[{ "objects": ["Gate:close"] }]],
      "after": [[{ "objects": ["Gate:open"] }]]
    }
  ]
}
```

この例では、プレイヤーがボタンを押すと同時に、盤面上のどこかにあるゲートが開く。2 つのパターンブロックは独立した位置でマッチングされる。

---

## 4. 条件（conditions）

ルールの適用前にグローバル変数の状態を型付き配列でチェックする。

```typescript
interface Condition {
  readonly variable: string;
  readonly op: ComparisonOp;
  readonly value: number | boolean | string;
}

type ComparisonOp = "==" | "!=" | "<" | "<=" | ">" | ">=";
```

**JSON 例:**

```json
{
  "conditions": [
    { "variable": "_input_direction", "op": "==", "value": "right" },
    { "variable": "level_unlocked", "op": ">=", "value": 5 }
  ]
}
```

複数の条件は AND で結合される（すべて真の場合のみルールが適用される）。

---

## 5. エフェクト（effects）

ルール適用時に発生する副作用を判別共用体の配列で定義する。

```typescript
type Effect =
  | { type: "set"; variable: string; value: number | boolean | string }
  | { type: "change"; variable: string; amount: number }
  | { type: "sound"; soundId: string }
  | { type: "message"; text: string }
  | { type: "call"; groupName: string };
```

**JSON 例:**

```json
{
  "effects": [
    { "type": "set", "variable": "level_complete", "value": true },
    { "type": "change", "variable": "moves", "amount": 1 },
    { "type": "change", "variable": "time_remaining", "amount": -1 },
    { "type": "sound", "soundId": "success" },
    { "type": "message", "text": "Level Clear!" },
    { "type": "call", "groupName": "physics" }
  ]
}
```

| タイプ | 説明 |
|:---|:---|
| `set` | グローバル変数を指定した値に設定 |
| `change` | グローバル変数を指定した数だけ増減（正で増加、負で減少） |
| `sound` | サウンド再生をレンダラーに委譲 |
| `message` | メッセージ表示をレンダラーに委譲 |
| `call` | 指定したルールグループを呼び出し |

---

## 6. 具体的なルール記述例

### 6.1 プレイヤーが箱を押す

```json
{
  "name": "push_box",
  "direction": "horizontal",
  "application": "once",
  "patterns": [{
    "before": [[
      { "objects": ["Player"] },
      { "objects": ["Box"], "noObjects": ["Wall"] },
      { "noObjects": ["@Solid"] }
    ]],
    "after": [[
      { "objects": [] },
      { "objects": ["Player"] },
      { "objects": ["Box"] }
    ]]
  }],
  "conditions": [
    { "variable": "_input_direction", "op": "==", "value": "right" }
  ],
  "effects": [
    { "type": "change", "variable": "moves", "amount": 1 },
    { "type": "change", "variable": "pushes", "amount": 1 },
    { "type": "sound", "soundId": "push" }
  ]
}
```

### 6.2 重力

```json
{
  "name": "gravity",
  "direction": "down",
  "application": "until_stable",
  "patterns": [{
    "before": [
      [{ "objects": ["@Falling"] }],
      [{ "noObjects": ["@Solid"] }]
    ],
    "after": [
      [{ "objects": [] }],
      [{ "objects": ["@Falling"] }]
    ]
  }]
}
```

### 6.3 箱がゴールに到達

```json
{
  "name": "box_enters_goal",
  "direction": "none",
  "patterns": [{
    "before": [[{ "objects": ["Box"], "hasObjects": ["Goal"] }]],
    "after": [[{ "objects": ["BoxOnGoal"] }]]
  }]
}
```

### 6.4 タグバインディングの例

```json
{
  "name": "color_match",
  "direction": "none",
  "patterns": [{
    "before": [[{ "objects": ["Player:$color", "Key:$color"] }]],
    "after": [[{ "objects": ["Player:$color"] }]]
  }],
  "effects": [
    { "type": "sound", "soundId": "collect" }
  ]
}
```

`Player:red` と `Key:red` が同じセルにあれば、Key が消える。`$color` は before でバインドされた値（`"red"`）が after にも引き継がれる。

### 6.5 複数ルールグループの例

```json
{
  "ruleGroups": [
    {
      "name": "player_movement",
      "application": "once",
      "rules": [...]
    },
    {
      "name": "physics",
      "application": "until_stable",
      "rules": [
        {
          "name": "gravity",
          "direction": "down",
          "application": "until_stable",
          "patterns": [{
            "before": [
              [{ "objects": ["@Falling"] }],
              [{ "noObjects": ["@Solid"] }]
            ],
            "after": [
              [{ "objects": [] }],
              [{ "objects": ["@Falling"] }]
            ]
          }]
        }
      ]
    },
    {
      "name": "gate_actions",
      "application": "once",
      "rules": [
        {
          "name": "open_gate",
          "direction": "none",
          "patterns": [{
            "before": [[{ "objects": ["Gate:close"] }]],
            "after": [[{ "objects": ["Gate:open"] }]]
          }],
          "conditions": [
            { "variable": "pushed", "op": "==", "value": true }
          ]
        }
      ]
    }
  ],
  "turnSequence": ["player_movement", "physics", "gate_actions"]
}
```

---

## 7. ルールグループ適用アルゴリズム

### 7.1 基本的な実行フロー

1. `turnSequence` の各グループ名について順番に処理
2. 各ルールグループについて、`application` 設定に従って実行：
   - `"once"`: グループ内のルールを 1 回ずつ順次実行
   - `"until_stable"`: グループ全体で変化がなくなるまで繰り返し実行

### 7.2 ルールグループ内の実行

1. グループ内のルールを定義順に処理
2. 各ルールについて：
   1. `conditions` を評価（不成立なら不適用）
   2. 全 `patterns` ブロックのマッチを検索（1 つでもマッチしなければ不適用）
   3. すべてマッチしたら after パターンを原子的に適用
   4. `effects` を適用
   5. ルールの `application` が `"until_stable"` なら安定するまで繰り返し
   6. ルールの `application` が `"once"` なら 1 回で次のルールへ

### 7.3 グループ単位での安定化

グループの `application` が `"until_stable"` の場合、グループ内のいずれかのルールが盤面を変更する限り、グループ全体を繰り返し実行する。

### 7.4 ルールグループの呼び出し

`call` エフェクトが発生した場合、対象グループが即座に実行される（再帰呼び出し可能）。

> 詳細なアルゴリズムは `docs/フレームワーク計画.md` セクション 8.3, 8.4 を参照。
