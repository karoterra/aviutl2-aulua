# CI

## GitHub Actions (mise)

GitHub Actions で aulua を利用する場合、 [mise](https://mise.jdx.dev/) を利用する方法があります。

以下のように `mise.toml` のツールに aulua が含まれている場合、

```toml:mise.toml
[tools]
"github:karoterra/aviutl2-aulua" = "latest"
```

ワークフローで `jdx/mise-action` を使えばワークフロー内で aulua を利用可能になります。

```yaml:.github/workflows/ci.yaml
name: CI

on:
  push:
    branches:
      - main
  pull_request:
  workflow_dispatch:

jobs:
  check:
    runs-on: windows-latest
    defaults:
      run:
        shell: bash

    steps:
      - name: Checkout source code
        uses: actions/checkout@v6

      - name: Setup mise
        uses: jdx/mise-action@v4
        with:
          install: true

      - name: You can use aulua
        run: aulua --version
```

詳細は [mise のドキュメント](https://mise.jdx.dev/continuous-integration.html)をご確認ください。


## GitHub Actions (マニュアルセットアップ)

GitHub Actions で aulua を利用する場合、ワークフロー内でバイナリを配置することもできます。

```yaml:.github/workflows/ci.yaml
name: CI

on:
  push:
    branches:
      - main
  pull_request:
  workflow_dispatch:

jobs:
  check:
    runs-on: windows-latest
    defaults:
      run:
        shell: bash

    steps:
      - name: Checkout source code
        uses: actions/checkout@v6

      - name: Setup aulua
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          tag=$(gh api repos/karoterra/aviutl2-aulua/releases/latest --jq '.tag_name')
          archive="aulua-${tag}-windows-x64.zip"
          mkdir -p aulua
          gh release download -R karoterra/aviutl2-aulua $tag -p $archive -D aulua
          unzip aulua/$archive -d aulua
          echo `pwd`/aulua > $GITHUB_PATH

      - name: You can use aulua
        run: aulua --version
```
