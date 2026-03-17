# インクルード

スクリプトソースに `---$include "other_file.hlsl"` のように書いた行はそのファイルの中身に置き換わります。

ファイルはスクリプトソースファイルからの相対パスで指定してください。

## 例

設定ファイル

```yaml:aulua.yaml
scripts:
  - name: スクリプト.anm2
    sources:
      - path: スクリプトソース.anm2
```

スクリプトソース

```lua:スクリプトソース.anm2
--[[pixelshader@psmain:
---$include "シェーダー.hlsl"
]]
```

```hlsl:シェーダー.hlsl
float4 psmain(float4 pos : SV_Position) : SV_Target {
    return float4(1.0);
}
```

ビルド結果

```lua:スクリプト.anm2
--[[pixelshader@psmain:
float4 psmain(float4 pos : SV_Position) : SV_Target {
    return float4(1.0);
}

]]
```
