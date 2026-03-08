# インストール

`aulua` は以下のいずれかの方法でインストールしてください。

## WinGet からインストール

Windows 11 や Windows 10 の最新バージョンには [WinGet](https://learn.microsoft.com/ja-jp/windows/package-manager/) が標準でインストールされているため、ターミナルで以下のコマンドを実行すればインストールできます。

```powershell
winget install -e --id Karoterra.Aulua
```

## Cargo でインストール

[Rust](https://rust-lang.org/) がインストールされている場合は Cargo からインストールできます。

```bash
cargo install --locked aulua
```

## Releases からダウンロード

[Releases](https://github.com/karoterra/aviutl2-aulua/releases/) から最新版をダウンロードし、 `aulua.exe` （あるいは `aulua`） を PATH の通ったディレクトリに配置してください。

`aulua-<version>-windows-x64.zip`
  : Windows 用（例： `aulua-v0.4.1-windows-x64.zip`）

`aulua-<version>-linux-x64.tar.gz`
  : Linux 用（例： `aulua-v0.4.1-linux-x64.tar.gz`）
