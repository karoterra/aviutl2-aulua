# aulua schema

`schema` コマンドは `aulua.yaml` のスキーマファイルを出力します。

`schema` コマンドは以下のように実行します。

```bash
aulua schema
```

デフォルトではカレントディレクトリに `aulua.schema.json` というファイル名で出力します。

## `-o`, `--output`

出力先ファイルのパスは `-o` または `--output` で指定できます。

```bash
aulua schema -o path/to/file.json
aulua schema --output path/to/file.json
```
