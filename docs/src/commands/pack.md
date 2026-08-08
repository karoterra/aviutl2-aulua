# aulua pack

`pack` コマンドは au2pkg パッケージを生成します。

`pack` コマンドは以下のように実行します。

```bash
aulua pack
```

パッケージに関する設定は `aulua.yaml` の [`package` セクション](../config.md#package)で指定します。

コマンドは `aulua.yaml` と同じ場所で実行してください。

## 言語ファイル

`aulua.yaml`の[`language.files`](../config.md#language)に設定した言語ファイルは、すべてパッケージへ格納されます。

元ファイルのディレクトリ部分は引き継がず、ファイル名を使って次のパスへ配置します。

```text
Language/<file name>
```

例えば`translations/English.example.aul2`は、パッケージ内の`Language/English.example.aul2`へ格納されます。元ファイルのバイト列はBOMや改行コードを変換せず、そのまま格納します。

`pack`は言語ファイルの生成、更新、検査、不要なキーの削除を行わず、元の言語ファイルも書き換えません。必要な言語ファイルは事前に[`aulua language update`](language.md#language-update)で作成し、必要に応じて検査してください。

次の場合はpackエラーになります。

- 設定された言語ファイルが存在しない、または読み込めない
- ファイルの拡張子が小文字の`.aul2`ではない
- 設定された複数のファイルが同じファイル名を持つ
- `package.assets`など別の項目とパッケージ内のパスが重複する

`text`と`tooltip`の設定にかかわらず、設定された言語ファイルはすべて格納します。`language update --output`や`language check --target`による上書き指定はpackには影響しません。
