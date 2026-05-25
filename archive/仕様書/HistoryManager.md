# HistoryManager仕様書

## 1. 概要

`HistoryManager`は、GameStateの履歴管理とundo/redo機能を提供する独立したコンポーネントです。GameManagerと連携し、プレイヤーの操作履歴を記録・管理することで、直感的な操作取り消し機能を実現します。

---

## 2. 基本責務

### 2.1. 履歴記録

- GameStateのスナップショット保存
### 2.2. Undo/Redo機能

- 前の状態への復元（undo）
- 取り消した操作の再実行（redo）

---

## 3. データ構造

### 3.1. 履歴エントリ

```gdscript
# HistoryEntry構造
var history_entry: Dictionary = {
	"global_state": {},
	"puzzle_state": []
}
```

### 3.2. 内部状態と履歴管理の仕組み

```gdscript
# HistoryManager内部状態
var _history_stack: Array[Dictionary] = []  # 履歴スタック
var _current_position: int = -1             # 現在位置
var _max_history_size: int = 100000         # 最大履歴数 (絶対に到達しない数)
var _is_enabled: bool = true                # 履歴記録有効フラグ
```

**履歴管理の仕組み:**

1. **通常の状態保存**: 新しい状態が`_history_stack`の末尾に追加され、`_current_position`が最新位置を指す
    
2. **Undo実行**: `_current_position`を1つ戻し、その位置の状態を返す
    
3. **Redo実行**: `_current_position`を1つ進め、その位置の状態を返す
    
4. **新しい操作**: undo後に新しい操作を行うと、`_current_position`より後の履歴は削除される
    

**例:**

```gdscript
# 状態A → 状態B → 状態C と進んだ場合
_history_stack = [StateA, StateB, StateC]
_current_position = 2  # StateCを指す

# 2回undoした場合
_current_position = 0  # StateAを指す
# 現在の状態はStateA、StateBとStateCにredoできる

# StateAから新しい状態Dに進んだ場合
_history_stack = [StateA, StateD]  # StateBとStateCは削除
_current_position = 1  # StateDを指す
```

---

## 4. 主要な機能

### 4.1. 履歴記録

#### `save_state(gamestate: Dictionary, move_number: int = -1) -> bool`

- 現在のGameStateを履歴に保存
- 履歴サイズ制限の適用

**動作:**

- 新しい状態を保存すると、現在位置より後の履歴（redo用）は削除される
- 最大履歴数を超えた場合、古い履歴から削除

#### `clear_history()`

- 全ての履歴をクリア
- レベル終了時に発動
- リスタートのときには発動しない (リスタートは履歴を保持するべき)

### 4.2. Undo/Redo操作

#### `undo() -> Dictionary`

- 履歴スタック内で1つ前の状態に戻る
- `_current_position`を1つ減らし、その位置の状態を返す
- 戻れる状態がない場合は空のDictionaryを返す

```gdscript
# undo動作例
# 現在: _current_position = 2, _history_stack = [A, B, C]
# undo実行後: _current_position = 1, 戻り値 = B
```

#### `redo() -> Dictionary`

- 取り消した操作を再実行
- `_current_position`を1つ増やし、その位置の状態を返す
- 再実行できる状態がない場合は空のDictionaryを返す

```gdscript
# redo動作例
# 現在: _current_position = 1, _history_stack = [A, B, C]
# redo実行後: _current_position = 2, 戻り値 = C
```
