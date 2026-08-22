# aulua language

`language` コマンドは、AviUtl2言語ファイル (`.aul2`) を作成・更新・検査します。
コマンドは `aulua.yaml` と同じ場所で実行してください。

言語ファイルの設定方法や基本的な作業手順については、[言語ファイル](../language/)を参照してください。

## `language update`

`language update` は、解析したスクリプトに対して不足しているセクションやキーを言語ファイルへ追加します。

```bash
aulua language update [OPTIONS]
```

### `--script <path>`

`--script` を指定しない場合は、`aulua.yaml` の `scripts` に設定されたスクリプトを解析します。トップレベルの `language.files` が1件以上ある場合は、すべてのスクリプトが対象です。トップレベルに言語ファイルがない場合は、スクリプト別の `language.files` が1件以上あるスクリプトだけを解析します。どちらの場合も、`scripts` の設定自体はすべて検証します。

指定した場合は、設定されたスクリプトの代わりに、指定したビルド済みAviUtl2スクリプトを直接解析します。複数指定でき、指定順に処理されます。

UIの解析方法はファイルの拡張子で決まります。`.tra2`では対応する`--param`だけを解析し、通常の`--track@`などは対象外です。

```bash
aulua language update --script build/Effect.anm2
aulua language update --script build/Effect.anm2 --script build/Object.obj2
```

`--script` は言語ファイルの選択とは独立しており、`--output` と組み合わせることもできます。`--output` を指定しない場合は、トップレベルとスクリプト別を含むすべての設定済み言語ファイルを更新します。各言語ファイルには、`--script` で指定したスクリプトの解析結果が使用されます。

### `--output <path>`

`--output` を指定しない場合は、`aulua.yaml` のトップレベルとスクリプト別の [`language.files`](../config.md#language) に設定されたすべてのファイルを更新します。`--script` を指定していない場合、トップレベルのファイルにはすべてのスクリプト、スクリプト別のファイルには対応する設定から生成される論理スクリプトの解析結果が使用されます。

指定した場合は、設定済みの言語ファイルを使用せず、指定した1つの言語ファイルだけを更新します。相対パスは `aulua.yaml` があるディレクトリを基準に解決され、Text と Tooltip の両方が管理対象になります。

```bash
aulua language update --output Language/English.example.aul2
```

`--output` を指定する場合も `aulua.yaml` は必要です。

### 更新内容

- ファイルが存在しない場合は新規作成する
- 不足しているセクションとキーを追加する
- 既存の値を上書きしない
- 既存のコメントと空行を保持する
- 現在解析したスクリプトから生成対象にならない管理対象外のセクションを保持する

既存`.aul2`の構文が不正な場合や、同名セクションまたは同一セクション内に同名キーが複数ある場合はエラーとなり、そのファイルを更新しません。

スクリプト解析中に未定義の変数が見つかった場合は警告をstderrへ出力しますが、処理は継続します。

### `--prune`

`--prune`を指定すると、通常の更新に加えて、現在解析したスクリプトから生成対象となるセクション内の不要なキーを削除します。

```bash
aulua language update --prune
```

セクション見出し、コメント、空行は削除しません。現在生成対象ではない管理対象外のセクションも変更しないため、以前は管理対象だったセクションを自動削除することはありません。

## `language check`

`language check` は言語ファイルを読み取り、現在のスクリプトに対する差分を検査します。言語ファイルは一切書き換えません。

```bash
aulua language check [OPTIONS]
```

### `--script <path>`

スクリプトの選択方法は[`language update`](#--script-path)と同じです。複数指定した場合は指定順に処理されます。

### `--target <path>`

`--target` を指定しない場合は、トップレベルとスクリプト別に設定されたすべての言語ファイルを、それぞれの対象範囲で検査します。`--script` を指定した場合は、各言語ファイルに指定したスクリプトの解析結果を使用します。

指定した場合は、設定済みの言語ファイルを使用せず、指定した1つの言語ファイルを Text・Tooltip 両方の対象として検査します。相対パスの基準と `aulua.yaml` が必要な点は `language update --output` と同じです。

```bash
aulua language check --target Language/English.example.aul2
```

### 検査内容

次の状態を検出事項として報告します。

- 言語ファイルが存在しない
- 必要なセクションが存在しない
- 必要なキーが存在しない
- 必要なキーの値が空
- 管理対象セクション内に不要なキーがある

現在生成対象ではない管理対象外のセクションとそのキーは検査対象外です。

### 出力と終了コード

検出事項はstdoutへ、未定義変数などの解析時の警告はstderrへ出力します。問題がない場合は、検出事項に関するstdoutへの出力はありません。

```text
Language/English.aul2: keyが存在しません: [effect] Amount
Language/English.aul2:4: 値が空です: [effect] Color
Language/English.aul2:5: 使用されていないkeyです: [effect] Old
```

行番号を持たない検出事項は`<path>: <diagnostic>`、行番号を持つ検出事項は`<path>:<line>: <diagnostic>`の形式です。

| 結果 | 終了コード |
| --- | ---: |
| 検出事項なし | 0 |
| 警告だけ | 0 |
| 検出事項あり | 1 |
| 設定、スクリプト、ファイルの読み込み・解析などの実行エラー | 失敗 |
