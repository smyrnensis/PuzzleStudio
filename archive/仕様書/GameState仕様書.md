# GameState Dictionary仕様書

## 1. 概要

本仕様書は、ルールベース・グリッドパズルゲームフレームワークにおけるゲーム状態 (`GameState`) を定義するGodot Dictionary形式を定めます。このデータ構造は、特定のレベルの盤面状態とレベル横断的なグローバル変数を管理します。

---

## 2. GameState 基本構造

`GameState`は、ゲームの現在の状態を表すGodotの`Dictionary`オブジェクトです。

```gdscript
# GameState Example
var game_state: Dictionary = {
	"global_state": {
		"moves_count": 120,
		"level_complete": false,
		"width": 10,
		"height": 8,
		"time_remaining": 300
	},
	"puzzle_state": [
		# 2D Array of cells
		# puzzle_state[y][x] = Array of object strings
	]
}
```

---

## 3. 各フィールドの詳細

### 3.1. global_state (Dictionary)

特定のレベル内で、セルの位置に紐付かない状態変数を管理します。これらの変数は、ルールシステムの`conditions`や`effects`で参照・更新されます。

**サポートされる型:**
- `int`: 整数値（スコア、移動回数、時間など）
- `bool`: 真偽値（フラグ、状態など）
- `String`: 文字列（状態名、メッセージなど）

**例:**
```gdscript
var global_state: Dictionary = {
	"moves_count": 45,           # int
	"level_complete": false,     # bool
	"player_name": "Player1",    # String
	"width": 10,                 # int
	"height": 8,                 # int
	"boxes_on_goal": 0,          # int
	"total_boxes": 3             # int
}
```

### 3.2. puzzle_state (Array[Array[Array[String]]])

盤面の状態を表現する3次元配列構造です。

**構造:**
```gdscript
# puzzle_state[y][x] = Array of object strings
var puzzle_state: Array = [
	# Row 0 (y=0)
	[
		["Floor", "Player"],  # Cell (0,0)
		["Floor", "Box"],     # Cell (1,0)
		["Goal"]              # Cell (2,0)
	],
	# Row 1 (y=1)
	[
		["Floor"],            # Cell (0,1)
		["Floor"],            # Cell (1,1)
		["Floor"]             # Cell (2,1)
	]
]
```

**オブジェクト表現:**
ゲーム内のオブジェクトは `"Name:tag1:tag2..."` 形式の文字列で表現されます。

- タグなし: `"Player"`
- 1つのタグ: `"Player:right"`
- 複数のタグ: `"Player:moving:right"`

---

## 4. GameStateの配置と管理方針

### 4.1. 配置場所
GameStateは**GameManager**が保持し、管理する。

### 4.2. 参照方針
- **読み取り**: 必要に応じて他のコンポーネントがGameManagerから取得
- **直接操作禁止**: GameState自体を直接変更してはならない

### 4.3. 操作権限
以下のスクリプト/コンポーネントのみがGameStateを操作できる：

| コンポーネント           | 操作権限            | 役割                            |
| :---------------- | :-------------- | :---------------------------- |
| **GameManager**   | 完全な読み書き         | GameStateの所有者。レベル読み込み、保存、全体制御 |
| **RuleProcessor** | 読み取り + 変更後状態の返却 | ルール適用によるGameStateの変化を生成して返す   |

### 4.4. 操作の原則
- **イミュータブル操作**: GameStateを直接変更せず、常に新しいインスタンスを生成
- **単一の情報源**: GameManagerが唯一の正式なGameStateを保持
- **明確な所有権**: GameStateの変更は必ずGameManagerを経由

---

## 5. 具体例

```gdscript
# 完全なGameStateの例
var complete_gamestate: Dictionary = {
	"global_state": {
		"moves_count": 15,
		"level_complete": false,
		"width": 3,
		"height": 2,
		"boxes_on_goal": 1,
		"total_boxes": 2,
		"time_remaining": 240
	},
	"puzzle_state": [
		# Row 0 (y=0)
		[
			["Floor", "Player:right"],  # (0,0)
			["Floor", "Box"],           # (1,0)
			["Goal", "Box:on_goal"]     # (2,0)
		],
		# Row 1 (y=1)
		[
			["Floor"],                  # (0,1)
			["Floor"],                  # (1,1)
			["Floor"]                   # (2,1)
		]
	]
}
```

この仕様により、GameStateは明確な責任分離のもとで管理され、データの整合性と操作の安全性が確保されます。