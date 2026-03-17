# ファイルの連結

ビルド時にはスクリプトソースに対して[インクルード](include.md)や[変数展開](variables.md)、UI定義の処理を行った後、スクリプトとしてビルド出力先にファイルを出力します。
このとき、1つのスクリプトに対して複数のスクリプトソースが指定されている場合は順番に連結されます。

`label` を指定した場合は@ラベルが付与されます。

## 例

設定ファイル

```yaml:aulua.yaml
scripts:
  - name: "@MyEffect.anm2"
    sources:
      - path: scripts/EffectA.in.anm2
        label: EffectA
      - path: scripts/EffectB.in.anm2
        label: EffectB
```

スクリプトソース

```lua:scripts/EffectA.in.anm2
-- EffectA.in.anm2
```

```lua:scripts/EffectB.in.anm2
-- EffectB.in.anm2
```

ビルド結果

```lua:@MyEffect.anm2
@EffectA
-- EffectA.in.anm2

@EffectB
-- EffectB.in.anm2
```
