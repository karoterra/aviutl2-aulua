# 変数展開

スクリプトソースに `${VAR_NAME}` のように書いたところは、 `aulua.yaml` の `variables` で指定した文字列に置き換わります。

`project` と `sources` で同じ名前の変数が指定されている場合は `sources` の値が優先されます。

未定義の変数がスクリプトソース内に見つかった場合は警告を表示します。

## 変数名

変数名に使用できる文字は以下の通りです。
- 英字（`A-Za-z`）
- 数字（`0-9`）
- アンダーバー（`_`）

## 組み込み変数

以下の組み込み変数も利用可能です。

| 変数名            | 内容                   |
|-------------------|------------------------|
| `PACKAGE_ID`      | `package.id` の値      |
| `PACKAGE_NAME`    | `package.name` の値    |
| `PACKAGE_VERSION` | `package.version` の値 |

## 例

設定ファイル

```yaml:aulua.yaml
project:
  variables:
    HOGE: ほげ
    FUGA: ふが

package:
  version: 1.0.0

scripts:
  - name: スクリプト.anm2
    sources:
      - path: スクリプトソース.anm2
        variables:
          FUGA: フガ
```

スクリプトソース

```lua:スクリプトソース.anm2
-- HOGE: ${HOGE}
-- FUGA: ${FUGA}
-- PACKAGE_VERSION: ${PACKAGE_VERSION}
```

ビルド結果

```lua:スクリプト.anm2
-- HOGE: ほげ
-- FUGA: フガ
-- PACKAGE_VERSION: 1.0.0
```
