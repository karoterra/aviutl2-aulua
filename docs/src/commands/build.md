# aulua build

`build` コマンドは `aulua` の一番主なコマンドです。
`aulua.yaml` の内容に応じてスクリプトソースからスクリプトファイルをビルドします。

`build` コマンドは以下のように実行します。

```bash
aulua build
```

ビルド出力先は `aulua.yaml` の [`build.out_dir`](../config.md#out_dir) で指定します。

コマンドは `aulua.yaml` と同じ場所で実行してください。

## 言語ディレクティブ

`---$tips`、`---$script_tips`、`---$nolang`は言語解析用のメタデータであり、ビルド出力から削除されます。TipsとScriptTipsの複数行の継続行もまとめて削除されます。

言語ディレクティブの構文が不正な場合はビルドエラーになります。通常のUIディレクティブは従来どおりAviUtl2形式へ変換されます。

詳しい構文は[言語ディレクティブ](../language/directives.md)を参照してください。
