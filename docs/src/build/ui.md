# UI設定

ユーザーに入力させる各種設定項目については以下のような変換を行います。

## トラックバー

ビルド前
```lua
---$track:X速度
---min=-10
---max=10
---step=0.01
local vx = 0
```

ビルド後
```lua
--track@vx:X速度,-10,10,0,0.01
```

以下のように1行で書くこともできます。

ビルド前
```lua
---$track:X速度, min = -10, max = 10, step = 0.01
local vx = 0
```

ビルド後
```lua
--track@vx:X速度,-10,10,0,0.01
```

## チェックボックス

ビルド前
```lua
---$check:重力
local grav = 0
---$check:速度
local speed = false
---$checksection:重力
local grav = false
```

ビルド後
```lua
--check@grav:重力,0
--check@speed:速度,false
--checksection@grav:重力,false
```

## 色

ビルド前
```lua
---$color:図形色
local col = 0xffffff
```

ビルド後
```lua
--color@col:図形色,0xffffff
```

## ファイル選択

ビルド前
```lua
---$file:画像ファイル
local path = ""
```

ビルド後
```lua
--file@path:画像ファイル
```

## フォルダ選択

ビルド前
```lua
---$folder:フォルダ
local path = ""
```

ビルド後
```lua
--folder@path:フォルダ
```

## フォント

ビルド前
```lua
---$font:フォント名
local font = "MS UI Gothic"
```

ビルド後
```lua
--font@font:フォント名,MS UI Gothic
```

## 図形

ビルド前
```lua
---$figure:先端図形
local fig = "三角形"
```

ビルド後
```lua
--figure@fig:先端図形,三角形
```

## リスト選択

ビルド前
```lua
---$select:装飾タイプ
---標準文字=0
---影付き文字=1
---影付き文字(薄)=2
---縁取り文字=3
---縁取り文字(細)=4
---縁取り文字(太)=5
---縁取り文字(角)=6
local deco = 0
```

ビルド後
```lua
--select@deco:装飾タイプ,標準文字=0,影付き文字=1,影付き文字(薄)=2,縁取り文字=3,縁取り文字(細)=4,縁取り文字(太)=5,縁取り文字(角)=6
```

## テキスト

ビルド前
```lua
---$text:テキスト
local txt = "デフォルト文字\n次の行"
---$string:文字列
local str = "デフォルト文字"
```

ビルド後
```lua
--text@txt:テキスト,デフォルト文字\n次の行
--string@str:文字列,デフォルト文字
```

## 変数

ビルド前
```lua
---$value:数値
local num = 0
---$value:文字列
local text = "0"
---$value:配列
local table = {0,0,0}
```

ビルド後
```lua
--value@num:数値,0
--value@text:文字列,"0"
--value@table:配列,{0,0,0}
```
