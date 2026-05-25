# ルールシステムJSON仕様書 v0.5

## 1. 概要

本仕様書は、ターンベース・グリッドベースパズルゲームフレームワークにおけるルールシステムのJSON形式を定義します。ルールは「パターンマッチングと置換」の原理に基づき、現在の盤面状態から特定のパターンを検索し、新しいパターンで置き換えることでゲーム状態を更新します。

## 2. ルールの基本構造

### 2.1 RuleData形式

```json
{
	"rule_groups": [
		{
			"name": "movement",
			"application": "until_stable",
			"rules": [
				{
					"application": "once",
					"conditions": {
						"game_started": true,
					},
					"before": [ 
						{
							"direction": "any",
							"pattern": [
								{ "position": {"x": 0, "y": 0}, "objects": ["Player"] },
								{ "position": {"x": 1, "y": 0}, "objects": [] },
							]
						}
					],
					"after": [
						{
							"pattern": [
								{ "position": {"x": 0, "y": 0}, "objects": [] },
								{ "position": {"x": 1, "y": 0}, "objects": ["Player"] },
							]
						}
					],
					"effects": {
						"sound": "move",
					}
				}
			]
		},
		{
			"name": "gravity",
			"enabled": false,
			"application": "until_stable",
			"rules": [
				{
					"application": "until_stable",
					"before": [
						{
							"direction": "down",
							"pattern": [
								{ "position": {"x": 0, "y": 0}, "objects": ["@falling"] },
								{ "position": {"x": 0, "y": 1}, "no_objects": ["@solid"] }
							]
						}
					],
					"after": [
						{
							"pattern": [
								{ "position": {"x": 0, "y": 0}, "objects": [] },
								{ "position": {"x": 0, "y": 1}, "objects": ["@falling"] }
							]
						}
					]
				}
			]
		}
	]
}
```

### 2.2 ルールグループ要素の詳細

|フィールド|型|必須|説明|
|:--|:--|:--|:--|
|**name**|string|○|ルールグループの名前。他のルールグループから呼び出しの際に参照されます。|
|enabled|boolean|×|ルールグループの有効/無効フラグ。デフォルトはtrue。|
|application|string|×|ルールグループの適用方法: `"once"` または `"until_stable"`。デフォルトは `"until_stable"`。|
|**rules**|array|○|このグループに含まれるルールの配列。|

### 2.3 Rule要素の詳細

| フィールド          | 型       | 必須  | 説明                                                               |
| :------------- | :------ | :-- | :--------------------------------------------------------------- |
| enabled        | boolean | ×   | ルールの有効/無効フラグ。デフォルトはtrue。                                         |
| application    | string  | ×   | ルールの適用方法: `"once"` または `"until_stable"`。デフォルトは `"until_stable"`。 |
| **conditions** | object  | ×   | ルール適用のためのグローバル変数条件。                                              |
| **before**     | array   | ○   | マッチング対象の盤面パターン（チェック条件を含む）。                                       |
| after          | array   | ○   | 置換後の盤面パターン。                                                      |
| effects        | object  | ×   | ルール適用時の副作用（サウンド再生、変数変更など）。                                       |
| description    | string  | ×   | デバッグ用のルールの名前。つけないのが基本。                                           |

### 2.4 実行順序

ルールグループは`rule_groups`配列の順序で実行されます。配列の先頭から末尾に向かって、各ルールグループが順次処理されます。

## 3. パターン定義

### 3.1 `before`ブロックのパターン要素

`before`ブロックは、複数のパターンを含む配列です。各パターンは`direction`、`tag`、`pattern`を持ちます。

```json
{
  "direction": "any",
  "pattern": [
    { "position": {"x": 0, "y": 0}, "objects": ["Player"] },
    { "position": {"x": 1, "y": 0}, "objects": ["Box:color"] },
    { "position": {"x": 2, "y": 0}, "no_objects": ["Wall"] }
  ]
}
```

### 3.2 direction

`before`ブロックの各パターンで指定されます。パターンの方向を指定します。

- `none` : 1×1の置換で方向の区別がない場合。1通りのルールを表す
- `up`/`down`/`left`/`right`: (x,y)=(1,0)を上/下/左/右とする1通りのルールを表す
- `vertical`: (x,y)=(1,0)を上下方向とする2通りのルールを表す
- `horizontal`: (x,y)=(1,0)を左右方向とする2通りのルールを表す
- `any`: (x,y)=(1,0)を上下左右方向とする4通りのルールを表す（デフォルト）

### 3.3 パターン要素のキー

|キー|役割|説明|
|:--|:--|:--|
|`objects`|**置換対象**|そのセルに特定のオブジェクトが存在することを示します。このキーで指定されたセルは、`after`ブロックによる**置換の対象**となります。|
|`has_objects`|**チェックのみ**|そのセルに特定のオブジェクトが**存在すること**を条件としてチェックします。置換の対象にはなりません。|
|`no_objects`|**チェックのみ**|そのセルに特定のオブジェクトが**存在しないこと**を条件としてチェックします。置換の対象にはなりません。|

### 3.4 Objects指定

オブジェクトの指定方法：

```json
// 具体的なオブジェクト
["Player:red:right"]

// タグを指定しない場合
["Player:color:direction"]

// タグのバインディング、beforeでマッチしたタグをafterで参照
["Player:$color:direction"]

// 相対方向指定
["Player:color:>"]

// プロパティによる指定
["@movable"]  // movableプロパティを持つ任意のオブジェクト

// 否定条件
["!movable"]  // movableプロパティを持たない任意のオブジェクト

// 複数条件
["@wall||@movable"]  // wallであるかmovableである任意のオブジェクト

// 複数オブジェクト
["Box", "Wall"] 
```

### 3.5 Position

位置は相対座標で指定します。

```json
// 相対座標
{"x": 0, "y": 0}  // 基準点
{"x": 1, "y": 0}  // 基準点から右に1グリッド
```

### 3.6 絶対方向と相対方向

- ">" は相対的な右方向の予約語で、"right"と類似の意味ですが、ルールを回転させたときに共に方向を変えます。
- 同様に "^", "v", "<", ">" は相対的な上下左右を表します。
- 逆にrightと書くと、絶対的な方向を指定することになります。

### 3.7 複数の`before`ブロック

`before`の中に複数のパターンを並べることができます。このとき、`after`も同じ個数のパターンを持たなければいけません。それぞれのパターンでは相対位置は異なる位置を指すことができます。つまり任意の位置にある特定のパターンと、任意の位置にある別のパターンを指定して、かつ2つのパターンの位置関係は指定しない、ということが可能になる。

```json
{
  "before": [
    {
      "direction": "any",
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["Player"] },
        { "position": {"x": 1, "y": 0}, "objects": ["Button"] }
      ]
    },
    {
      "direction": "any",
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["Gate:close"] }
      ]
    }
  ],
  "after": [
    {
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": [] },
        { "position": {"x": 1, "y": 0}, "objects": ["Player", "Button"] }
      ]
    },
    {
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["Gate:open"] }
      ]
    }
  ]
}
```

### 3.8 `after`ブロックのルール

`after`ブロックには、`before`ブロック内で`objects`キーを使って指定されたセルに対する変更のみを記述します。各`after`パターンは対応する`before`パターンと同じ構造を持ちます。

```json
{
  "pattern": [
    { "position": {"x": 0, "y": 0}, "objects": [] },
    { "position": {"x": 1, "y": 0}, "objects": ["Player"] }
  ]
}
```

## 4. 条件 (`conditions`)

`conditions`ブロックは、ルールが適用されるための追加条件として、グローバル変数の状態をチェックします。

```json
{
  "conditions": {
    "game_started": true,
    "level_unlocked": { "gte": 5 }
  }
}
```

### 4.1 比較演算子

|演算子|説明|
|:--|:--|
|`eq`|等しい（デフォルト）|
|`ne`|等しくない|
|`gt`|より大きい|
|`gte`|以上|
|`lt`|より小さい|
|`lte`|以下|

## 5. 効果 (`effects`)

`effects`ブロックは、ルールの適用時に発生する副作用を定義します。グローバル変数の変更、サウンドの再生、メッセージの表示、他のルールグループの呼び出しなどが可能です。

```json
{
  "effects": {
    "set": {
      "level_complete": true
    },
    "change": {
      "moves_count": 1,
      "time_remaining": -1
    },
    "sound": "success",
    "message": "Level Clear!",
    "call": "physics"
  }
}
```

|キー|説明|
|:--|:--|
|`set`|グローバル変数の値を指定した値に設定します。|
|`change`|グローバル変数の値を指定した数だけ増減させます（正の数で増加、負の数で減少）。|
|`sound`|指定されたサウンドエフェクトを再生します。|
|`message`|指定されたメッセージを表示します。|
|`call`|指定されたルールグループを呼び出します。|

## 6. 具体的なルール記述例

### 6.1 プレイヤーが箱を押す

```json
{
  "application": "once",
  "before": [
    {
      "direction": "any",
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["Player"] },
        { "position": {"x": 1, "y": 0}, "objects": ["Box"] },
        { "position": {"x": 2, "y": 0}, "no_objects": ["Wall"] }
      ]
    }
  ],
  "after": [
    {
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": [] },
        { "position": {"x": 1, "y": 0}, "objects": ["Player"] },
        { "position": {"x": 2, "y": 0}, "objects": ["Box"] }
      ]
    }
  ],
  "effects": {
    "sound": "push"
  }
}
```

### 6.2 重力

```json
{
  "application": "until_stable",
  "before": [
    {
      "direction": "down",
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["@falling"] },
        { "position": {"x": 0, "y": 1}, "no_objects": ["@solid"] }
      ]
    }
  ],
  "after": [
    {
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": [] },
        { "position": {"x": 0, "y": 1}, "objects": ["@falling"] }
      ]
    }
  ]
}
```

### 6.3 箱がゴールに到達

```json
{
  "application": "once",
  "before": [
    {
      "direction": "none",
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["Box"], "has_objects": ["Goal"] }
      ]
    }
  ],
  "after": [
    {
      "pattern": [
        { "position": {"x": 0, "y": 0}, "objects": ["Box:on_goal"] }
      ]
    }
  ],
  "effects": {
    "sound": "success",
    "change": {
      "boxes_on_goal": 1
    }
  }
}
```

### 6.5 複数のルールグループを使った例

```json
{
  "rule_groups": [
    {
      "name": "player_movement",
      "application": "once",
      "rules": [
        {
          "application": "once",
          "before": [
            {
              "direction": "any",
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": ["Player"] },
                { "position": {"x": 1, "y": 0}, "objects": [] }
              ]
            }
          ],
          "after": [
            {
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": [] },
                { "position": {"x": 1, "y": 0}, "objects": ["Player"] }
              ]
            }
          ]
        }
      ]
    },
    {
      "name": "physics",
      "application": "until_stable",
      "rules": [
        {
          "description": "gravity",
          "application": "until_stable",
          "before": [
            {
              "direction": "down",
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": ["@falling"] },
                { "position": {"x": 0, "y": 1}, "no_objects": ["@solid"] }
              ]
            }
          ],
          "after": [
            {
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": [] },
                { "position": {"x": 0, "y": 1}, "objects": ["@falling"] }
              ]
            }
          ]
        },
        {
          "description": "collision",
          "application": "once",
          "before": [
            {
              "direction": "any",
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": ["@moving"] },
                { "position": {"x": 1, "y": 0}, "objects": ["@solid"] }
              ]
            }
          ],
          "after": [
            {
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": ["@moving:stopped"] },
                { "position": {"x": 1, "y": 0}, "objects": ["@solid"] }
              ]
            }
          ]
        }
      ]
    },
    {
      "name": "gate_actions",
      "application": "once",
      "rules": [
        {
	      "conditions": {"pushed": true}
          "before": [
            {
              "direction": "none",
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": ["Gate:close"] }
              ]
            }
          ],
          "after": [
            {
              "pattern": [
                { "position": {"x": 0, "y": 0}, "objects": ["Gate:open"] }
              ]
            }
          ]
        }
      ]
    }
  ]
}
```

## 7. ルールグループ適用アルゴリズム

### 7.1 基本的な実行フロー

1. `rule_groups`配列を順番に処理します。
2. 各ルールグループについて：
    1. グループの`enabled`フラグをチェックします。
    2. グループの`application`設定に従って実行します：
        - `"once"`: グループ内のルールを1度だけ順次実行
        - `"until_stable"`: グループ内のルールによる変化がなくなるまで繰り返し実行

### 7.2 ルールグループ内の実行

1. グループ内のルールを定義順に処理します。
2. 各ルールについて：
    1. 盤面全体から、すべての`before`パターン（`objects`, `has_objects`, `no_objects`の全て）に完全に一致する位置を左上から順に探索します。
    2. マッチする箇所が見つかった場合、`conditions`に記述されたグローバル変数条件をチェックします。
    3. 全ての条件を満たす場合、`before`で`objects`キーで指定されたセルを、対応する`after`パターンの内容で置き換えます。
    4. `effects`を適用します。
    5. `effects`に`call`が指定されている場合、指定されたルールグループを呼び出します。
    6. ルールの`application`が`"once"`の場合、このルールの適用は1回で終了し、次のルールへ進みます。
    7. ルールの`application`が`"until_stable"`の場合、盤面に変化がなくなるまで同じルールの適用を試みます。

### 7.3 グループ単位での安定化

- グループの`application`が`"until_stable"`の場合、グループ内のいずれかのルールが盤面を変更する限り、グループ全体を繰り返し実行します。
- これにより、複数のルールが相互作用する物理演算（重力と衝突など）を効率的に処理できます。

### 7.4 ルールグループの呼び出し

- ルールの`effects`に`call`が指定されている場合、そのルールグループが即座に実行されます。
- 呼び出されたルールグループは、呼び出し元のルール適用が完了した後に実行されます。
- ルールグループの呼び出しは、現在の実行コンテキストから独立して動作します。

### 7.5 実行例

```json
{
  "rule_groups": [
    {
      "name": "player_movement",
      "application": "once",
      "rules": []
    },
    {
      "name": "physics",
      "application": "until_stable", 
      "rules": []
    },
    {
      "name": "special_effects",
      "application": "once",
      "rules": []
    }
  ]
}
```

この例では：

1. `player_movement`グループが1度実行されます
2. `physics`グループが安定するまで繰り返し実行されます
3. `special_effects`グループが1度実行されます
4. いずれかのルールで`"call": "physics"`が指定されていれば、そのタイミングで`physics`グループが追加実行されます