# 設定ファイル aulua.yaml

`aulua` ではプロジェクトの設定ファイルを `aulua.yaml` に記載します。
以下は設定ファイルのサンプルです。

```yaml:aulua.yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/karoterra/aviutl2-aulua/refs/heads/main/schema/aulua.schema.json

project:
  variables:
    VERSION: v1.0.0
    AUTHOR: karoterra

build:
  out_dir: build
  embed_search_dirs:
    - lib

install:
  out_dir: C:\ProgramData\aviutl2\Script\karoterra\

package:
  id: karoterra.ColorVisionSimulation
  name: 色覚シミュレーションKR
  information: 色覚シミュレーションKR for AviUtl2 v{version}
  version: 1.0.0
  uninstall_sub_folder_file: true
  out_dir: build
  file_name: "{id}-v{version}.au2pkg.zip"
  script_sub_dir: "{id}"
  message:
    file: package.txt
    # text: |
    #   aulua.yaml に直接書くこともできます。
  assets:
    - src: docs/README.md
      dest: Script/{id}/docs/README.md

language:
  files:
    - path: Language/Default.ColorVisionSimulation.aul2
      text: false
      tooltip: true
    - path: Language/English.ColorVisionSimulation.aul2

scripts:
  - name: 色覚シミュレーションKR.anm2
    sources:
      - path: script/色覚シミュレーションKR.in.anm2
        variables:
          INFO: 色覚シミュレーションKR for AviUtl2

  - name: "@MyEffect.anm2"
    sources:
      - path: scripts/EffectA.in.anm2
        label: EffectA
        variables:
          INFO: MyEffect (EffectA)
      - path: scripts/EffectB.in.anm2
        label: EffectB
        variables:
          INFO: MyEffect (EffectB)
```

## project

プロジェクト全体に関する設定を指定します。

```yaml
project:
  variables:
    VERSION: v1.0.0
    AUTHOR: karoterra
```

`variables`
  : プロジェクト全体で使用する変数を定義します。
    この変数はビルド時にスクリプトファイルに埋め込まれます（[関連項目](build/variables.md)）。
    上記サンプルでは `VERSION`, `AUTHOR` という変数を定義していますが、変数の個数・名前は自由に指定可能です。
    ただし変数名に使える文字は英字（`A-Za-z`）、数字（`0-9`）、アンダーバー（`_`）です。

## build

ビルドに関する設定を指定します。

```yaml
build:
  out_dir: build
  embed_search_dirs:
    - lib
```

`out_dir`
  : [`build`](commands/build.md) コマンドを実行した際のスクリプトファイル出力先ディレクトリを指定します。デフォルト値は `build` です。

`embed_search_dirs`
  : [`build`](commands/build.md) コマンドの[モジュール埋め込み](build/embed.md)処理の際に追加で参照するディレクトリを指定します。


## install

インストールに関する設定を指定します。

```yaml
install:
  out_dir: C:\ProgramData\aviutl2\Script\karoterra\
```

`out_dir`
  : [`install`](commands/install.md) コマンドを実行した際のスクリプトファイル出力先ディレクトリを指定します。デフォルト値は `%PROGRAMDATA%\aviutl2\Script` です（`%PROGRAMDATA%` は環境変数）。


## package

パッケージに関する設定を指定します。

```yaml
package:
  id: karoterra.ColorVisionSimulation
  name: 色覚シミュレーションKR
  information: 色覚シミュレーションKR for AviUtl2 v{version}
  version: 1.0.0
  uninstall_sub_folder_file: true
  out_dir: build
  file_name: "{id}-v{version}.au2pkg.zip"
  script_sub_dir: "{id}"
  message:
    file: package.txt
    # text: |
    #   aulua.yaml に直接書くこともできます。
  assets:
    - src: docs/README.md
      dest: Script/{id}/docs/README.md
```

`id`
  : パッケージの識別子を指定します。 `package.ini` に使用されます。

`name`
  : パッケージの名称を指定します。 `package.ini` に使用されます。

`information`
  : パッケージの情報を指定します。 `package.ini` に使用されます。

`version`
  : パッケージのバージョンを指定します。

`uninstall_sub_folder_file`
  : アンインストール時のサブフォルダのファイル削除を指定します。 `package.ini` に使用されます。

`out_dir`
  : パッケージファイルの出力先ディレクトリを指定します
    省略時は `build.out_dir` と同じ値になります。

`file_name`
  : 出力されるパッケージファイルのファイル名を指定します。
    省略時は `"{id}-v{version}.au2pkg.zip"` （ `version` 指定時）または `"{id}.au2pkg.zip"` （ `version` 省略時）が使用されます。

`script_sub_dir`
  : ビルドしたスクリプトファイルを配置するディレクトリを指定します。
    `Script` 直下に配置したい場合は空文字列を指定してください。
    省略時は `"{id}"` が使用されます。

`message`
  : `package.txt` に指定するテキストを指定します。
    ファイルを参照する場合は `message.file` にファイルのパスを指定してください。
    テキストを直接指定する場合は `message.text` に指定してください。

`assets`
  : ビルド出力結果以外にパッケージに含めたいファイルを指定します。
    `src` にファイルのパスを、 `dest` にパッケージ内の配置先を指定してください。
    配置先は `Plugin/`, `Language/` など `Script/` 以外も指定可能です。

`information`, `file_name`, `script_sub_dir`, `assets[].dest` では `{...}` 形式のテンプレートを使用可能です。

使用可能な変数:
- `id`
- `name`
- `version`
- `project.variables` の各キー


## language

言語ファイルの生成・更新・検査に関する設定を指定します。

```yaml
language:
  files:
    - path: Language/Default.example.aul2
      text: false
      tooltip: true
    - path: Language/English.example.aul2
```

`files`
  : 管理する言語ファイルを配列で指定します。設定されたファイルを使用して`language update`または`language check`を実行する場合は、1件以上必要です。

`files[].path`
  : 言語ファイルのパスを指定します。相対パスは`aulua.yaml`があるディレクトリを基準に解決されます。
    ファイルの拡張子は大文字小文字を区別し、小文字の`.aul2`である必要があります。ファイル名はUTF-8文字列として扱える必要があります。

`files[].text`
  : `true`の場合、通常翻訳セクションを更新・検査・削除の対象にします。デフォルト値は`true`です。

`files[].tooltip`
  : `true`の場合、Tipsセクションを更新・検査・削除の対象にします。デフォルト値は`true`です。

`text`と`tooltip`の両方を`false`にすることはできず、設定エラーになります。完全に同じ設定パスのファイルを複数回指定した場合は`aulua language`コマンドでエラーになります。

言語ファイルの形式や作業手順については[言語ファイル](language/)、コマンドについては[`aulua language`](commands/language.md)を参照してください。


## scripts

ビルド・インストールの対象となるスクリプトに関する設定を指定します。

```yaml
scripts:
  - name: 色覚シミュレーションKR.anm2
    sources:
      - path: script/色覚シミュレーションKR.in.anm2
        variables:
          INFO: 色覚シミュレーションKR for AviUtl2

  - name: "@MyEffect.anm2"
    sources:
      - path: scripts/EffectA.in.anm2
        label: EffectA
        variables:
          INFO: MyEffect (EffectA)
      - path: scripts/EffectB.in.anm2
        label: EffectB
        variables:
          INFO: MyEffect (EffectB)
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
