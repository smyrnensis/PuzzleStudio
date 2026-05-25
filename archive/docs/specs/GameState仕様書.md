# GameState 仕様書

> 詳細な型定義は `docs/フレームワーク計画.md` セクション 5.2 を参照。

---

## 1. 概要

本仕様書は、ルールベース・グリッドパズルゲームフレームワークにおけるゲーム状態（`GameState`）のデータ構造を定義する。GameState は特定のレベルの盤面状態とグローバル変数を保持する、ゲームの単一の情報源である。

---

## 2. GameState 基本構造

`GameState` は、ゲームの現在の状態を表すイミュータブルなオブジェクトである。

```typescript
interface GameState {
  readonly globalState: GlobalState;
  readonly puzzle: Puzzle;
  readonly width: number;   // グリッドの幅（列数）
  readonly height: number;  // グリッドの高さ（行数）
}
```

### 概念的な構造

```
GameState
├── globalState: Record<string, number | boolean | string>
└── puzzle: Puzzle[y][x] → Cell → readonly ObjectInstance[]
```

---

## 3. 各フィールドの詳細

### 3.1 globalState

特定のレベル内で、セルの位置に紐付かない状態変数を管理する。ルールシステムの `conditions` や `effects` で参照・更新される。

```typescript
type GlobalValue = number | boolean | string;
type GlobalState = Readonly<Record<string, GlobalValue>>;
```

**サポートされる型:**
- `number`: 整数値（スコア、移動回数等）
- `boolean`: 真偽値（フラグ、状態等）
- `string`: 文字列（状態名、方向等）

**例:**

```typescript
const globalState: GlobalState = {
  _input_direction: "none",
  moves: 45,
  level_complete: false,
  boxes_on_goal: 0,
  total_boxes: 3,
};
```

### 3.2 puzzle

盤面の状態を表現する2次元配列。`puzzle[y][x]` でセルにアクセスする。y=0 が最上行、x=0 が最左列。

```typescript
type ObjectInstance = string;  // "Name" | "Name:tag1" | "Name:tag1:tag2"
type Cell = readonly ObjectInstance[];
type Puzzle = readonly (readonly Cell[])[];
```

**オブジェクト表現:**

ゲーム内のオブジェクトは `"Name:tag1:tag2..."` 形式の文字列で表現される。

- タグなし: `"Player"`
- 1つのタグ: `"Player:right"`
- 複数のタグ: `"Player:moving:right"`

**例:**

```typescript
const puzzle: Puzzle = [
  // Row 0 (y=0)
  [
    ["Floor", "Player:right"],  // Cell (0,0)
    ["Floor", "Box"],           // Cell (1,0)
    ["Goal", "BoxOnGoal"],      // Cell (2,0)
  ],
  // Row 1 (y=1)
  [
    ["Floor"],                  // Cell (0,1)
    ["Floor"],                  // Cell (1,1)
    ["Floor"],                  // Cell (2,1)
  ],
];
```

### 3.3 width / height

グリッドの幅と高さ。レベルデータの map から自動計算される。

---

## 4. GameState の管理方針

### 4.1 配置場所

GameState は **GameManager** が保持し、管理する。

### 4.2 参照方針

- **読み取り**: 必要に応じて他のコンポーネントが GameManager から取得
- **直接操作禁止**: GameState 自体を直接変更してはならない

### 4.3 操作権限

| コンポーネント | 操作権限 | 役割 |
|:---|:---|:---|
| **GameManager** | 完全な読み書き | GameState の所有者。レベル読み込み、全体制御 |
| **RuleProcessor** | 読み取り + 新しい状態の返却 | ルール適用による新しい GameState を生成して返す |

### 4.4 イミュータブル操作

TypeScript の `Readonly<T>` 型を活用し、コンパイル時に不変性を強制する。GameState を変更する必要がある場合は、常に新しいインスタンスを生成する。

```typescript
// 正しい: 新しいオブジェクトを生成
const newState: GameState = {
  ...currentState,
  globalState: { ...currentState.globalState, moves: currentState.globalState.moves + 1 },
};

// 間違い: 直接変更（TypeScriptの型システムがコンパイルエラーで防止）
// currentState.globalState.moves = 10;  // Error: readonly
```

---

## 5. 具体例

```typescript
const completeGameState: GameState = {
  width: 3,
  height: 2,
  globalState: {
    _input_direction: "none",
    moves: 15,
    level_complete: false,
    boxes_on_goal: 1,
    total_boxes: 2,
  },
  puzzle: [
    // Row 0 (y=0)
    [
      ["Floor", "Player:right"],  // (0,0)
      ["Floor", "Box"],           // (1,0)
      ["Goal", "BoxOnGoal"],      // (2,0)
    ],
    // Row 1 (y=1)
    [
      ["Floor"],                  // (0,1)
      ["Floor"],                  // (1,1)
      ["Floor"],                  // (2,1)
    ],
  ],
};
```

この構造により、GameState は明確な責任分離のもとで管理され、データの整合性と操作の安全性が確保される。
