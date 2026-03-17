# モジュール埋め込み

スクリプトソースに `---$embed` と書いた次の行の `require()` は、Lua モジュールがスクリプトファイルに埋め込まれます。

埋め込む Lua モジュールはスクリプトソースと同じ場所から探します。
ほかの場所にある Lua モジュールを参照できるようにするには [`build.embed_search_dirs`](../config.md#embed_search_dirs) を設定してください。

## 例

スクリプトソース

```lua:スクリプトソース.anm2
---$embed
local mylib = require("mylib")
```

```lua:mylib.lua
local M = {}
function M.add(x, y)
    return x + y
end
return M
```

ビルド結果

```lua:スクリプト.anm2
-- aulua embed: mylib
local function __aulua_embed_1__()
local M = {}
function M.add(x, y)
    return x + y
end
return M

end
local mylib = __aulua_embed_1__()
```
