# aulua build

`build` コマンドは `aulua` の一番主なコマンドです。
`aulua.yaml` の内容に応じてスクリプトソースからスクリプトファイルをビルドします。

`build` コマンドは以下のように実行します。

```bash
aulua build
```

ビルド出力先は `aulua.yaml` の [`build.out_dir`](../config.md#out_dir) で指定します。

コマンドは `aulua.yaml` と同じ場所で実行してください。
