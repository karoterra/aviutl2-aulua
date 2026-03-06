# 設定ファイル aulua.yaml

`aulua` ではプロジェクトの設定ファイルを `aulua.yaml` に記載します。
以下は設定ファイルのサンプルです。

```yaml
{{#include config_example.yaml}}
```

## project

プロジェクト全体に関する設定を指定します。

```yaml
{{#include config_example.yaml:3:6}}
```

`variables`
  : プロジェクト全体で使用する変数を定義します。
    この変数はビルド時にスクリプトファイルに埋め込まれます（[関連項目](build/variables.md)）。
    上記サンプルでは `VERSION`, `AUTHOR` という変数を定義していますが、変数の個数・名前は自由に指定可能です。
    ただし変数名に使える文字は英字（`A-Za-z`）、数字（`0-9`）、アンダーバー（`_`）です。

## build

ビルドに関する設定を指定します。

```yaml
{{#include config_example.yaml:8:11}}
```

`out_dir`
  : [`build`](commands/build.md) コマンドを実行した際のスクリプトファイル出力先ディレクトリを指定します。デフォルト値は `build` です。

`embed_search_dirs`
  : [`build`](commands/build.md) コマンドの[モジュール埋め込み](build/embed.md)処理の際に追加で参照するディレクトリを指定します。


## install

インストールに関する設定を指定します。

```yaml
{{#include config_example.yaml:13:14}}
```

`out_dir`
  : [`install`](commands/install.md) コマンドを実行した際のスクリプトファイル出力先ディレクトリを指定します。デフォルト値は `%PROGRAMDATA%\aviutl2\Script` です（`%PROGRAMDATA%` は環境変数）。

## scripts

ビルド・インストールの対象となるスクリプトに関する設定を指定します。

```yaml
{{#include config_example.yaml:16:32}}
```

`[].name`
  : ビルド結果として出力されるスクリプトファイル名を指定します。

`[].sources`
  : スクリプトファイルのビルドに使用するソースを配列で指定します。

`[].sources[].path`
  : スクリプトソースのパスを指定します。 `aulua.yaml` からの相対パスで指定してください。

`[].sources[].label`
  : スクリプトに `@` ラベルを付与します。省略した場合は付与されません。

`[].sources[].variables`
  : スクリプトソースで使用する変数を定義します。
    この変数はビルド時にスクリプトファイルに埋め込まれます（[関連項目](build/variables.md)）。
    上記サンプルでは `INFO` という変数を定義していますが、変数の個数・名前は自由に指定可能です。
    ただし変数名に使える文字は英字（`A-Za-z`）、数字（`0-9`）、アンダーバー（`_`）です。
    `project.variables` に同じ名前の変数が存在する場合は、スクリプトソースの変数の値が使用されます。

## スキーマ

`aulua.yaml` のスキーマは [`schema`](commands/schema.md) コマンドで出力できます。
また[こちらのURL](https://raw.githubusercontent.com/karoterra/aviutl2-aulua/refs/heads/main/schema/aulua.schema.json)からも参照可能です。

例えば Visual Studio Code の [YAML 拡張機能](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)は

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/karoterra/aviutl2-aulua/refs/heads/main/schema/aulua.schema.json
```

と先頭に書いてあるYAMLの構造が正しいか検証してくれたり、キーの候補を表示してくれます。
