# aulua

**AviUtl2 Lua script build & install tool**

[![CI](https://github.com/karoterra/aviutl2-aulua/actions/workflows/ci.yaml/badge.svg)](https://github.com/karoterra/aviutl2-aulua/actions/workflows/ci.yaml)

`aulua` は、AviUtl2 用の Lua スクリプトのビルドやインストールを支援する CLI ツールです。

## 機能概要

- HLSLファイルなど別ファイルのインクルード展開
- 変数の展開
- `--select` や `--track` などを分かりやすい形式からAviUtl2用に変換
- 指定のフォルダにインストール
- au2pkg パッケージの生成

## インストール方法

以下のいずれかの方法でインストールしてください。

### WinGet からインストール

```
winget install -e --id Karoterra.Aulua
```

### Cargo でインストール

Rust がインストールされている場合は Cargo からインストールできます。

```bash
cargo install --locked aulua
```

### Releases からダウンロード

[Releases](https://github.com/karoterra/aviutl2-aulua/releases/) から最新版をダウンロードし、 `aulua.exe` を PATH の通ったディレクトリに配置してください。


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

`build` コマンドでビルドされたスクリプトファイルが `build` ディレクトリに出力されます。

```bash
aulua build
```

`install` コマンドでスクリプトファイルがスクリプトフォルダに配置されます。

```bash
aulua install
```

## ドキュメント

詳しい使い方は[ドキュメント](https://karoterra.github.io/aviutl2-aulua/)をご確認ください。

## ライセンス

このソフトウェアは MIT ライセンスのもとで公開されます。
詳細は [LICENSE](LICENSE) を参照してください。
