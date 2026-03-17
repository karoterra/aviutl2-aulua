# 変数展開

スクリプトソースに `${VAR_NAME}` のように書いたところは、 `aulua.yaml` の `variables` で指定した文字列に置き換わります。

`project` と `sources` で同じ名前の変数が指定されている場合は `sources` の値が優先されます。
未定義の変数がスクリプトソース内に見つかった場合は警告を表示します。

## 変数名

変数名に使用できる文字は以下の通りです。
- 英字（`A-Za-z`）
- 数字（`0-9`）
- アンダーバー（`_`）

## 例

設定ファイル

```yaml:aulua.yaml
project:
  variables:
    HOGE: ほげ
    FUGA: ふが

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
```

ビルド結果

```lua:スクリプト.anm2
-- HOGE: ほげ
-- FUGA: フガ
```
