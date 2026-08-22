# 言語ファイル

`aulua`は、AviUtl2スクリプトから翻訳対象のTextやTipsを抽出し、言語ファイル（`.aul2`）の作成・更新・検査を支援します。

## 基本的な流れ

1. `aulua.yaml` のトップレベルまたはスクリプト別の [`language.files`](../config.md#language) へ言語ファイルを追加する
2. 必要に応じてスクリプトへ[言語ディレクティブ](directives.md)を記述する
3. [`aulua language update`](../commands/language.md#language-update)を実行する
4. Default以外の言語ファイルを開き、空の値を翻訳する
5. [`aulua language check`](../commands/language.md#language-check)を実行する
6. [`aulua pack`](../commands/pack.md)でパッケージを作成する

スクリプトのUI項目などを変更した後は、再度`language update`を実行してください。不要なキーも削除したい場合だけ`--prune`を指定します。

`pack`は言語ファイルの更新や検査を自動実行しません。パッケージを作成する前に、必要に応じて手動で`language update`と`language check`を実行してください。

## 最小例

`aulua.yaml`へ出力先を設定します。

```yaml
language:
  files:
    - path: Language/Default.example.aul2
    - path: Language/English.example.aul2
```

スクリプトへTipsなどのメタデータを記述します。通常のUI設定については[UI設定](../build/ui.md)も参照してください。

```lua
---$script_tips:サンプル効果です
---$tips:強さを指定します
---$track:強さ, min = 0, max = 100, step = 1
local amount = 0
```

言語ファイルを更新・翻訳してから検査し、パッケージを作成します。

```bash
aulua language update
aulua language check
aulua pack
```

## 生成対象

各論理スクリプトについて、スクリプト名、対応しているUI項目名、トラックバーのゼロ値名称、リスト選択の選択肢などを通常翻訳の対象にします。

UI項目のTipsは`---$tips`を記述した項目だけ、スクリプト全体のTipsは`---$script_tips`を記述した論理スクリプトだけが対象です。個別に翻訳対象から外す場合は`---$nolang`を使用します。詳しい構文は[言語ディレクティブ](directives.md)を参照してください。

### `.tra2`の生成対象

`.tra2`では、`--param`の記法に応じて次の名前を通常翻訳の対象にします。

- `--param:項目名,初期値`では、項目名
- `--param:項目名/check,初期値`では、項目名
- `--param:項目名/select/選択肢名=値/選択肢名=値,初期値`では、項目名と各選択肢名

> [!NOTE]
> チェックボックスとリスト選択の翻訳にはAviUtl2 v2.1.6以降が必要です。

```lua
--param:周期,0.5
--param:加速/check,0
--param:種類/select/直線=1/曲線=2,1
```

`.tra2`の`--param`では、項目名や選択肢名に`::`が含まれる場合も、名前全体を翻訳キーとして使用します。例えば`--param:aaa::種類/select/直線=1/aaa::直線=2,1`からは`aaa::種類`、`直線`、`aaa::直線`を生成します。

次の形式は対象外です。

- 項目名のない`--param:0.5`
- 初期値部分が空の`--param:周期,`
- 未知の項目形式や、`選択肢名=値`の構造を認識できないリスト選択`--param`
- `.tra2`内の`--track@`、`--check@`、`--select@`などの通常UI記法
- `.anm2`、`.obj2`、`.cam2`、`.scn2`内の`--param`

リスト選択の値と初期値の対応、値の型・範囲・重複は検査しません。同名の項目や選択肢は同じ翻訳キーへ集約します。

`.tra2`ではTipsが表示されないため、`---$tips`と`---$script_tips`を使用するとエラーになります。

## `.aul2`の基本形式

通常翻訳は論理スクリプト名のセクション、Tipsは`Tips.<論理スクリプト名>`のセクションへ出力されます。

```ini
[effect]
Amount=Amount

[Tips.effect]
effect.name=サンプル効果です
Amount=強さを指定します
```

- `[section]`がセクション見出し
- `key=value`が1つの翻訳項目
- 物理行の先頭が`;`の行がコメント
- 空白文字だけの行が空行
- 空の値は未翻訳として`language check`の対象

同名セクションが複数ある場合や、同一セクション内に同名キーが複数ある場合はエラーです。構文が不正なファイルも更新・検査できません。

複数行のTipsに含まれる実際の改行は、`.aul2`上では`\n`として出力します。

新規生成する`.aul2`はUTF-8、BOMなし、改行コードLFです。既存ファイルを更新した場合もBOMや改行コードはこの形式へ変換されることがあります。一方、既存の値、コメント、空行、管理対象外のセクションなどの文書内容は保持します。

## Default言語ファイル

ファイル名が次のいずれかに一致する場合、Default言語ファイルとして扱います。

```text
Default.aul2
Default.<name>.aul2
```

例えば`Default.example.aul2`はDefault言語ファイルです。ディレクトリ部分は判定に使用せず、ファイル名の大文字小文字を区別します。そのため`default.aul2`はDefault言語ファイルではありません。

新規項目の初期値は次のようになります。

| ファイル | Text | Tooltip |
| --- | --- | --- |
| Default | キー自身 | ディレクティブに記述したTips本文 |
| 通常 | 空 | 空 |

これは新規項目についての規則です。既存の値はDefault言語ファイルであっても`language update`で上書きしません。

## 管理対象セクションと管理対象外のセクション

現在解析したスクリプトから生成対象となるセクションだけを、更新・検査・削除の管理対象として扱います。

現在生成対象にならない既存セクションは管理対象外として扱い、通常の更新・検査・削除では内容を変更・報告・削除しません。以前は生成対象だったセクションであっても、現在のスクリプトから生成対象でなければ管理対象外です。

このため`language update --prune`は、現在の管理対象セクション内にある不要なキーだけを削除します。
