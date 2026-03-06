# aulua init

`aulua` を利用する際にはスクリプトファイルのソースのほかに `aulua.yaml` という名前の設定ファイルがプロジェクトごとに必要です。
`init` コマンドはこのようなテンプレートを生成してプロジェクトの開始をサポートします。

`init` コマンドは以下のように実行します。

```bash
aulua init
```

`init` コマンドを実行すると以下のファイルが生成されます。

```
project-dir/
├── .gitignore
├── .gitattributes
├── .editorconfig
├── aulua.yaml
└── script/
    ├── SampleAnimationEffect.anm2
    └── SampleShader.hlsl
```

`aulua.yaml` はプロジェクトの設定ファイルです。

スクリプトソースのサンプルとして `SampleAnimationEffect.anm2`, `SampleShader.hlsl` を出力しています。
スクリプトソースは `script/` ディレクトリに配置する必要はないので、プロジェクトの方針に合わせて `src/` ディレクトリにしたり、 `aulua.yaml` と同じ場所に配置しても問題ありません。

`.gitignore`, `.gitattributes`, `.editorconfig` はお節介で出力しているファイルであり、 `aulua` の動作には必要ありません。

## ディレクトリの指定

`init` コマンドを実行するとカレントディレクトリにテンプレートを生成しますが、出力先のディレクトリを指定することもできます。

```bash
aulua init path/to/dir
```
