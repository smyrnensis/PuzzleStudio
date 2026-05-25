# HistoryManager 仕様書

> 詳細な API は `docs/フレームワーク計画.md` セクション 7.8 を参照。

---

## 1. 概要

`HistoryManager` は、GameState の履歴管理と undo/redo 機能を提供する独立したコンポーネントである。GameManager と連携し、プレイヤーの操作履歴を記録・管理することで、直感的な操作取り消し機能を実現する。

---

## 2. 基本責務

### 2.1 履歴記録
- GameState のスナップショット保存

### 2.2 Undo/Redo 機能
- 前の状態への復元（undo）
- 取り消した操作の再実行（redo）

---

## 3. データ構造

### 3.1 内部状態

```typescript
class HistoryManager {
  private stack: GameState[];  // 履歴スタック
  private pointer: number;     // 現在位置
}
```

**履歴管理の仕組み:**

1. **通常の状態保存**: 新しい状態が `stack` の末尾に追加され、`pointer` が最新位置を指す
2. **Undo 実行**: `pointer` を 1 つ戻し、その位置の状態を返す
3. **Redo 実行**: `pointer` を 1 つ進め、その位置の状態を返す
4. **新しい操作**: undo 後に新しい操作を行うと、`pointer` より後の履歴は削除される

**例:**

```typescript
// 状態A → 状態B → 状態C と進んだ場合
stack = [StateA, StateB, StateC]
pointer = 2  // StateC を指す

// 2回 undo した場合
pointer = 0  // StateA を指す
// 現在の状態は StateA、StateB と StateC に redo できる

// StateA から新しい状態D に進んだ場合
stack = [StateA, StateD]  // StateB と StateC は削除
pointer = 1  // StateD を指す
```

---

## 4. 主要な機能

### 4.1 状態の保存

#### `push(state: GameState): void`

- 現在の GameState を履歴に保存
- `pointer` より先の履歴（redo 用）は破棄される

### 4.2 Undo/Redo 操作

#### `undo(): GameState | null`

- 履歴スタック内で 1 つ前の状態に戻る
- `pointer` を 1 つ減らし、その位置の状態を返す
- 戻れる状態がない場合は `null` を返す

#### `redo(): GameState | null`

- 取り消した操作を再実行
- `pointer` を 1 つ増やし、その位置の状態を返す
- 再実行できる状態がない場合は `null` を返す

### 4.3 状態確認

#### `canUndo(): boolean`

- undo 可能かどうかを返す

#### `canRedo(): boolean`

- redo 可能かどうかを返す

#### `current(): GameState | null`

- 現在の状態を返す。履歴が空なら `null`

### 4.4 履歴クリア

#### `clear(): void`

- 全ての履歴をクリア
- **レベル終了時** に呼び出す
- **リスタート時には呼び出さない**（リスタートは履歴を保持し、undo で巻き戻せる）

---

## 5. クラス定義

```typescript
export class HistoryManager {
  private pointer: number;
  private stack: GameState[];

  constructor();

  push(state: GameState): void;
  undo(): GameState | null;
  redo(): GameState | null;
  canUndo(): boolean;
  canRedo(): boolean;
  clear(): void;
  current(): GameState | null;
}
```

---

## 6. 設計ポイント

- **履歴サイズ制限なし**: GameState はイミュータブルかつ比較的軽量（グリッドベースのパズルゲーム）なので、実質無制限
- **deepClone は呼び出し元の責務**: HistoryManager は受け取った参照をそのまま保存する。GameState はイミュータブルなので参照の共有は安全
- **restart の実装**: GameManager が初期状態の GameState を `push` する操作として実装。undo で restart 自体を巻き戻せる
