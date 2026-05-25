
## 1\. ObjectDBの実装 (GDScript)

まず、`ObjectDB`の機能を持つスクリプトを作成します。これは、ゲーム起動時に一度だけJSONを読み込み、必要な情報をいつでも提供できるようにするものです。

#### `ObjectDB.gd`

```gdscript
# ObjectDB.gd
# ゲーム内のオブジェクト定義を一元管理するシングルトン。
# 起動時にJSONファイルを読み込み、オブジェクトに関する情報を提供する。
extends Node

# オブジェクト定義JSONファイルのパス
const OBJECT_DEF_PATH = "res://gamedata/object_definitions.json"

# パースした定義を格納する辞書 { "オブジェクト名": {定義データ} }
var _definitions: Dictionary = {}


func _ready():
	_load_object_definitions()


# JSONファイルを読み込み、データを辞書に格納する
func _load_object_definitions():
	if not FileAccess.file_exists(OBJECT_DEF_PATH):
		push_error("ObjectDB Error: Definition file not found at %s" % OBJECT_DEF_PATH)
		return

	var file = FileAccess.open(OBJECT_TDEF_PATH, FileAccess.READ)
	var content = file.get_as_text()
	var json_data = JSON.parse_string(content)

	if json_data == null:
		push_error("ObjectDB Error: Failed to parse JSON from %s" % OBJECT_DEF_PATH)
		return
	
	if not json_data.has("objectDefinitions"):
		push_error("ObjectDB Error: JSON must have a root 'objectDefinitions' key.")
		return

	# オブジェクト名をキーにして辞書に格納し、高速にアクセスできるようにする
	for obj_def in json_data["objectDefinitions"]:
		if obj_def.has("name"):
			_definitions[obj_def["name"]] = obj_def
		else:
			push_warning("ObjectDB Warning: Found an object definition without a 'name'.")


### Public API ###

# 指定されたオブジェクトのプロパティリストを取得する
func get_properties(object_name: String) -> Array:
	if _definitions.has(object_name) and _definitions[object_name].has("properties"):
		return _definitions[object_name]["properties"]
	return []


# 指定されたオブジェクトのレイヤーIDを取得する
func get_layer_id(object_name: String) -> int:
	if _definitions.has(object_name) and _definitions[object_name].has("layerId"):
		return _definitions[object_name]["layerId"]
	
	push_warning("ObjectDB Warning: Could not find layerId for '%s'." % object_name)
	return -1 # エラーを示す値


# オブジェクトのインスタンス文字列を生成する
func create_instance_string(object_name: String, tags: Dictionary = {}) -> String:
	if not _definitions.has(object_name):
		push_error("ObjectDB Error: Cannot create instance of unknown object '%s'." % object_name)
		return ""

	var instance_str = object_name
	var obj_def = _definitions[object_name]

	if not obj_def.has("tags"):
		return instance_str

	# 定義されている各タググループについて処理
	for group_name in obj_def["tags"]:
		var tag_def = obj_def["tags"][group_name]
		var final_tag_value = null
		
		# 引数で指定されていればそれを使う
		if tags.has(group_name):
			final_tag_value = tags[group_name]
		# そうでなければデフォルト値を使う
		elif tag_def.has("default"):
			final_tag_value = tag_def["default"]
		
		# 最終的なタグがnullでなければ文字列に追加
		if final_tag_value != null:
			instance_str += ":" + final_tag_value
			
	return instance_str


# インスタンス文字列を解析して、名前とタグの辞書を返す
func parse_instance_string(instance_string: String) -> Dictionary:
	var parts = instance_string.split(":")
	if parts.is_empty():
		return {}

	var object_name = parts[0]
	var result = {
		"name": object_name,
		"tags": {}
	}
	
	if not _definitions.has(object_name) or not _definitions[object_name].has("tags"):
		return result
		
	var defined_tags = _definitions[object_name]["tags"]
	
	# "Player:right" の "right" がどのグループに属するか逆引きする
	for i in range(1, parts.size()):
		var tag_value = parts[i]
		var found = false
		for group_name in defined_tags:
			if tag_value in defined_tags[group_name]["values"]:
				result["tags"][group_name] = tag_value
				found = true
				break
		if not found:
			push_warning("ObjectDB: Could not resolve tag '%s' for object '%s'." % [tag_value, object_name])
			
	return result

```

-----

## 2\. プロジェクトへの配置方法

`ObjectDB`をプロジェクト全体から簡単に呼び出せるように、**Autoload (シングルトン)** として設定するのが最も効果的です。

### ステップ1: ファイルを配置する

1.  Godotエディタの「ファイルシステム」ドックで、`res://`直下に`scripts`と`gamedata`というフォルダを作成します。

2.  `ObjectDB.gd`を`res://scripts/`フォルダの中に保存します。

3.  仕様書で定義した`object_definitions.json`ファイルを`res://gamedata/`フォルダの中に作成・保存します。

    **`res://gamedata/object_definitions.json` の内容例:**

    ```json
    {
      "objectDefinitions": [
        {
          "name": "Player",
          "layerId": 1,
          "properties": ["Movable", "PlayerControlled"],
          "tags": {
            "direction": {
              "values": ["up", "down", "left", "right"],
              "default": "down"
            }
          }
        },
        { "name": "Wall", "layerId": 1, "properties": ["Solid"] },
        { "name": "Goal", "layerId": 0 }
      ]
    }
    ```

### ステップ2: Autoloadに設定する

1.  メニューから `プロジェクト` -\> `プロジェクト設定` を開きます。
2.  `Autoload` タブに切り替えます。
3.  `パス`の右にあるフォルダアイコンをクリックし、`res://scripts/ObjectDB.gd`を選択します。
4.  `ノード名`が自動的に`ObjectDB`になります（この名前でグローバルにアクセスします）。
5.  `追加`ボタンを押します。

これで設定は完了です。`ObjectDB`はゲーム実行時に自動的にインスタンス化され、どのスクリプトからでも`ObjectDB`という名前で直接アクセスできるようになります。

-----

## 3\. 使い方（他のスクリプトからの呼び出し例）

Autoloadに設定したことで、どのスクリプトからでも以下のように簡単に`ObjectDB`の関数を呼び出せます。

```gdscript
# 例えば、ゲームのメインロジックを担う GameManager.gd の中で

func _on_player_move_input(direction):
	# "Player"が"Movable"プロパティを持っているか確認
	var props = ObjectDB.get_properties("Player")
	if "Movable" in props:
		print("Player is movable!") # -> Player is movable!

	# プレイヤーの移動先レイヤーIDを取得
	var layer = ObjectDB.get_layer_id("Player")
	print("Player is on layer: %d" % layer) # -> Player is on layer: 1

	# "right"方向のプレイヤーインスタンス文字列を生成
	var player_instance_str = ObjectDB.create_instance_string("Player", {"direction": "right"})
	print(player_instance_str) # -> Player:right

	# 文字列から情報を解析
	var parsed_data = ObjectDB.parse_instance_string(player_instance_str)
	print(parsed_data) # -> {name:Player, tags:{direction:right}}
```