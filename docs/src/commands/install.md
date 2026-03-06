# aulua install

`install` コマンドはビルドしたスクリプトファイルを所定のディレクトリにコピーします。

`install` コマンドは以下のように実行します。

```bash
aulua install
```

コピー先は `aulua.yaml` の [`install.out_dir`](../config.md#out_dir-1) で指定します。

コマンドは `aulua.yaml` と同じ場所で実行してください。

## `--dry-run`

`--dry-run` オプションを指定するとファイルはコピーせず、どのファイルがどこにコピーされるかだけ表示します。

```bash
aulua install --dry-run
```
