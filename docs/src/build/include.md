# インクルード

スクリプトソースに `---$include "other_file.hlsl"` のように書いた行はそのファイルの中身に置き換わります。

ファイルはスクリプトソースファイルからの相対パスで指定してください。

## 例

`aulua.yaml`
```yaml
scripts:
  - name: スクリプト.anm2
    sources:
      - path: スクリプトソース.anm2
```

`スクリプトソース.anm2`
```lua
--[[pixelshader@psmain:
---$include "シェーダー.hlsl"
]]
```

`シェーダー.hlsl`
```hlsl
float4 psmain(float4 pos : SV_Position) : SV_Target {
    return float4(1.0);
}
```

`スクリプト.anm2`
```lua
--[[pixelshader@psmain:
float4 psmain(float4 pos : SV_Position) : SV_Target {
    return float4(1.0);
}

]]
```
