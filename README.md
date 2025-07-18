# aulua

**AviUtl2 Lua script build & install tool**

[![CI](https://github.com/karoterra/aviutl2-aulua/actions/workflows/ci.yaml/badge.svg)](https://github.com/karoterra/aviutl2-aulua/actions/workflows/ci.yaml)

`aulua` は、AviUtl2 用の Lua スクリプトのビルドやインストールを支援する CLI ツールです。

## 機能概要

- HLSLファイルなど別ファイルのインクルード展開
- 変数の展開
- `--select` や `--track` などを分かりやすい形式からAviUtl2用に変換
- 指定のフォルダにインストール

## インストール方法

### Releases からダウンロード

[Releases](https://github.com/karoterra/aviutl2-aulua/releases/) から最新版をダウンロードし、 `aulua.exe` を PATH の通った場所に配置する。

### Cargo でインストール

```bash
cargo install --git https://github.com/karoterra/aviutl2-aulua
```

## 使い方

### ビルドからインストールの流れ

スクリプトのソースファイルとあわせて
`aulua.yaml` という名前で以下のような設定ファイルを作成します。

```yaml
project:
  variables:
    VERSION: v1.0.0
    AUTHOR: karoterra

build:
  out_dir: build

install:
  out_dir: C:\ProgramData\aviutl2\Script\karoterra\

scripts:
  - name: 色覚シミュレーションKR.anm2
    sources:
      - path: script/色覚シミュレーションKR.in.anm2
        variables:
          INFO: 色覚シミュレーションKR for AviUtl2
```

`aulua.yaml` と同じ場所で以下のコマンドを実行すると `build.out_dir` で指定したフォルダにスクリプトが出力されます。
`build.out_dir` を指定しなかった場合は `build` フォルダに出力されます。

```bash
aulua build
```

以下のコマンドを実行すると `install.out_dir` で指定したフォルダにビルド済みスクリプトがコピーされます。
`install.out_dir` を指定しなかった場合は `%PROGRAMDATA%\aviutl2\Script` フォルダに出力されます。

```bash
aulua install
```

### ビルド

`sources` で `label` を指定すると@ラベルを付与します。

`---$include "other_file.hlsl"` のように書いた行はそのファイルの中身に置き換わります。
ファイルはスクリプトソースファイルからの相対パスで指定してください。

`${VAR_NAME}` のように書いたところは `variables` で指定した文字列に置き換わります。
`project` と `sources` で同じ名前の変数が指定されている場合は `sources` の値が優先されます。
未定義の変数がスクリプトソース内に見つかった場合は警告を表示します。

以下のように書いたところはトラックバー定義に置き換わります。
```lua
-- ビルド前

---$track:X速度
---min=-10
---max=10
---step=0.01
local vx = 0

-- ビルド後

--track@vx:X速度,-10,10,0,0.01
```

以下の様に書いたところはチェックボックス定義に置き換わります。
```lua
-- ビルド前

---$check:重力
local grav = 0

-- ビルド後

--check@grav:重力,0
```

以下の様に書いたところは色設定項目定義に置き換わります。
```lua
-- ビルド前

---$color:図形色
local col = 0xffffff

-- ビルド後

--color@col:図形色,0xffffff
```

以下の様に書いたところはファイル選択項目定義に置き換わります。
```lua
-- ビルド前

---$file:画像ファイル
local path = ""

-- ビルド後

--file@path:画像ファイル
```

以下の様に書いたところはフォント設定項目定義に置き換わります。
```lua
-- ビルド前

---$font:フォント名
local font = "MS UI Gothic"

-- ビルド後

--font@font:フォント名,MS UI Gothic
```

以下の様に書いたところは図形設定項目定義に置き換わります。
```lua
-- ビルド前

---$figure:先端図形
local fig = "三角形"

-- ビルド後

--figure@fig:先端図形,三角形
```

以下の様に書いたところはリスト選択項目定義に置き換わります。
```lua
-- ビルド前

---$select:装飾タイプ
---標準文字=0
---影付き文字=1
---影付き文字(薄)=2
---縁取り文字=3
---縁取り文字(細)=4
---縁取り文字(太)=5
---縁取り文字(角)=6
local deco = 0

-- ビルド後

--select@deco:装飾タイプ,標準文字=0,影付き文字=1,影付き文字(薄)=2,縁取り文字=3,縁取り文字(細)=4,縁取り文字(太)=5,縁取り文字(角)=6
```

以下の様に書いたところは変数項目定義に置き換わります。
```lua
-- ビルド前

---$value:数値
local num = 0
---$value:文字列
local text = "0"
---$value:配列
local table = {0,0,0}

-- ビルド後

--value@num:数値,0
--value@text:文字列,"0"
--value@table:配列,{0,0,0}
```

### インストール

以下のコマンドを実行するとファイルはコピーせずに、どのファイルがどこにコピーされるかだけ表示します。

```bash
aulua install --dry-run
```

## ライセンス

このソフトウェアは MIT ライセンスのもとで公開されます。
詳細は [LICENSE](LICENSE) を参照してください。
