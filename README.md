
# ToDoc
## How to build

```rust
cargo build --release
cargo run -- --init
```

## How to use
todoc.json 作为配置文件, 有以下字段:
```json
{
    "workspace": ".",
    "cache_dir": ".todoc/cache",
    "doc_target_out_dir": ".todoc/docs",
    "default_format": "markdown",
    "sources": {
        "examples": []
    },
    "source_code": ".",
    "target_code": "html",
    "theme": "default"
}
```
其中,
`workspace` 做为工作目录, 其往往是源码文件的根目录, todoc 会在 workspace 中创建 .todoc 
文件用于管理
`cache_dir` 是缓存文件的路径, 其往往会自动执行, 不用手动修改
`doc_target_out_dir` 是后端代码生成的路径, 例如 html 的路径
`default_format` 是指代代码中 `\content{}` 内使用哪个格式进行解析, 默认是 markdown
`sources` 是实际运行时解析的所有源码, 如果没有 `---<!export>` 记号的源码则不会加入列表
同时, 也可以手动加入其他源码. 亦或者通过 `--file [<file_paht>]` 来添加
`source_code` 指明需要解析的目录位置, 其往往就是 workspace
`target_code` 指代输出文件格式, 例如 html
`theme` 是目前给输出 html 追加的 css 样式, 使用 defualt 即可

### lua
在 lua 源码中, 在文件头填入 `---<!export ${ModuleName}>` 来指明该文件要导出API文档, 且
属于哪个模块, 这里的 `ModuleName` 是用于 todoc 进行检索的, 与源码无关不要求强制指定. 也
可以不填写, 只使用 `---<!export>` 来告知 todoc 需要导出, 此时 todoc 会将 API 自动添加
到 Global 模块.

在 lua 源码的注释中, 你可以使用以下 Tag 来辅助解析


#### @brief
`@brief` 用于指明该函数的行为, 其仅仅支持 `\content[markdown]{}`, 例如:
```lua
---@brief 加法
function Add(a, b)
end
```
会被解析为
```lua
---@brief \content[markdown]{加法}
function Add(a, b)
end
```
`\content[markdown]{xxx}` 内的内容则会按照 markdown 解析, 由于配置中有 `defualt_format`
此时可以 `\content{}` 则会按照配置中的`defualt_format`来解析

#### @param
用于指定参数, 例如
```lua
---@param a \type number 加数1
function Add(a, b, c)
end
```
其会被解析为 `---@param \name{a} \type{number}  \content[markdown]{加数1}` 虽然你可
以手动添加这些标识, 但是对于以上这种情况可以直接按照上例.

`\type` 不是必须的, 可以省略:
```lua
---@param a 加数1
```
会被解析为 `---@param \name{a} \content[markdown]{加数1}`

如果参数名后面有逗号, todoc 会自动去除:
```lua
---@param a, 加数1
```
等效于 `@param a 加数1`

#### @return
`@return` 行为同 `@param`, 只是不支持 `\name`. 用于描述返回值, 例如:
```lua
---@return \type number 两数之和
function Add(a, b)
end
```
其会被解析为 `---@return \type{number} \content[markdown]{两数之和}`

#### @note
`@note` 行为同 `@brief`, 用于给这个 API 做出批注, 例如:
```lua
---@note 这个函数性能较差, 不建议频繁调用
function HeavyWork()
end
```
多个 `@note` 会依次渲染

#### @todo
`@todo` 用于标记待办事项, 行为同 `@brief`, 例如:
```lua
---@todo 需要增加边界检查
function GetItem(idx)
end
```

#### @deprecated
`@deprecated` 标记该 API 已废弃. 可以不带内容, 也可以附加说明:
```lua
---@deprecated 请使用 NewAdd 替代
function OldAdd(a, b)
end
```
如果附加了说明, 会以 `Deprecated: xxx` 的形式追加到备注中

#### @export
`@export` 用于强制导出本函数的文档. 对于 `local function` 默认不会导出, 使用 `@export`
可以让 todoc 也为其生成文档:
```lua
---@brief 内部辅助函数
---@export
local function helper()
end
```

#### @usage
`@usage` 用于记录“哪里使用了这个 API”. 支持以下标签:
- `\content{}`: 用法说明, 行为同前
- `\path{}`: 使用位置的文件路径, 支持绝对路径和相对路径(相对于 workspace)
- `\apiname{}`: 该文件中调用方 API 名称

例如:
```lua
---@usage \content{来自购买流程} \path{examples/ShopHelper.lua} \apiname{BuyItem}
```

也支持语法糖:
```lua
---@usage 在购买流程中使用
--- @  第二行补充
```

生成 HTML 时:
- `path` 会生成可跳转文件链接
- `path + apiname` 若命中 todoc 已管理的导出 API, 会跳转到对应 API 卡片
- 若目标不在 todoc 管理范围内, 仍会保留 path 跳转, apiname 以文本展示

这些不同的类型会使用不同的渲染方式

### 多行延续

所有 Tag 都支持多行延续. 当注释内容过长需要换行时, 在下一行使用 `--- @` 加空白来延续
上一行的内容, todoc 会自动识别并保留换行:
```lua
---@brief 通过 iStoreId 获取价格信息, 注意!
--- @      这个接口只会拿第一个
--- @      要拿到完整信息请使用 ShopManager 的接口
function T.GetPriceInfo(iStoreId)
end
```
这里三行会合并为一个 `@brief`, 且内容中的换行会被保留为实际的换行, 最终在 HTML 中正确
渲染为多行. 这对 `@param` `@note` `@todo` 等所有 Tag 均适用.

如果延续行不以 `--- @` 开头而是直接以 `---` 开头的普通文本, 也会被识别为延续:
```lua
---@param id \type number 商品ID
--- 必须为正整数
--- 无效ID会返回nil
```
上面三行 `@param` 的 content 会合并, 保留换行.

### 正式语法

以上的 Tag 写法实际上是语法糖. todoc 内部的正式语法使用 `\keyword{content}` 的形式:
- `\name{xxx}` 参数名
- `\type{xxx}` 类型名
- `\default{xxx}` 默认值
- `\defualt{xxx}` 默认值(兼容旧写法, 等同于 `\default{xxx}`)
- `\content[format]{xxx}` 内容, `format` 可省略, 省略时使用 `default_format`

你可以直接使用正式语法, 例如:
```lua
---@param \name{id} \type{number} \default{0} \content[markdown]{商品ID, 默认为0}
```
使用语法糖时, todoc 会在解析前自动将其转换为正式语法. 已经使用正式语法的行不会被二次处理

### 文件概述

文件头部连续的注释行（在任何代码之前）会被解析为文件概述, 概述会显示在生成的文档页面顶部:
```lua
---<!export Shop>
-- 商店模块
-- 提供商品查询和购买功能
--
-- 使用前需要先调用 Init

function T.Init()
end
```
其中 `---<!export Shop>` 标记不会出现在概述中, 空行不会中断概述, 遇到第一行代码时概述
结束. 块注释 `--[[ ... ]]` 也可以作为概述

### 依赖识别

todoc 会自动识别文件中的 `require` 调用并记录为依赖:
```lua
local Utils = require("Utils")
local Config = require 'Config'
```
在生成的文档中会列出该文件的依赖列表

### 导出规则

todoc 对不同类型的函数有不同的导出规则:
- `function T.Foo()` 表方法 / `function Foo()` 全局函数: 有任意 Tag 或 `@export` 即导出
- `local function Foo()` 局部函数: 必须显式使用 `@export` 才会导出

如果你定义了 Tag 但不想导出, 可以使用 `@private` 来显式禁止导出, 其优先级最高:
```lua
---@brief 内部辅助函数, 不需要出现在文档中
---@param x \type number 参数
---@private
function T.InternalHelper(x)
end
```
即使有其他 Tag, `@private` 也会阻止该函数被纳入文档

这意味着只要你为表方法或全局函数写了注释 Tag, todoc 就会自动将其纳入文档

### 缓存机制

todoc 会缓存解析结果到 `cache_dir` 指定的目录中. 每个源文件对应一个 JSON 缓存文件,
缓存包含文件的 `last_modified` 时间戳. 当源文件没有修改时, todoc 会直接读取缓存而不是
重新解析, 从而加快文档生成速度. 如果文件被修改, todoc 会自动更新缓存

你也可以使用 `--refresh` 手动触发一次全量重新扫描并更新缓存

### 输出结构

todoc 会将文档输出到 `doc_target_out_dir` 指定的目录中, 并保留源码的目录结构. 每个有
导出 API 的源文件生成一个 HTML 页面, 每个目录生成一个 `index.html` 索引页. 索引页中
列出子目录和该目录下的源文件链接. 即便目录下没有可解析的源码, 目录结构也会被保留

如果工作区中存在 `README.md` 文件, todoc 也会将其渲染为 HTML 页面复制到输出目录中

### 主题

todoc 通过 `theme` 字段支持不同的 CSS 主题:
- `default`: 默认主题, 使用 Gruvbox 风格的代码高亮配色
- `soft`: 柔和主题

主题控制文档页面的整体样式, 包括代码块配色、参数标题颜色、备注样式等

### Use Command In terminal
在终端中, 我们假设你只要一个 todoc.exe 在你的源码目录. 在终端中执行:
```bash
todoc --init 
```
他将会以当前目录作为 workspace 和 source_code, 并初始化和扫描源码并生成文档.

你可以通过
```bash
todoc --browse 
```
来使用默认浏览器打开文档的总目录index.html, 他会保留项目中的目录结构, 即便并没有可以解析
的源码

在终端中, 你可以使用以下命令来查询模块支持哪些Api, 以及这个 Api 属于哪些模块
```bash
todoc --find <ModuleName>
todoc --findapi <ModuleName> <APIName>
```
这里的 `<APIName>` 你可以使用通配符(`.*`)来进行搜索

#### --findapi 扩展用法

`--findapi` 提供了多种搜索模式:
```bash
todoc --findapi Shop Buy.*          # 在 Shop 模块中搜索 Buy 开头的 API
todoc --findapi -g GetItem          # 在 Global 模块中搜索
todoc --findapi -f GetItem          # 跨模块搜索函数名, 结果会显示所属模块
todoc --findapi -r Shop "Get.*Info" # 使用正则表达式搜索
```
- `-g` 等效于将模块名指定为 Global
- `-f` 不限定模块, 在所有导出 API 中搜索函数名
- `-r` 启用正则表达式模式, 默认情况下只有 `.*` 会被识别为通配符

#### --file

对单个文件生成文档:
```bash
todoc --file examples/ShopHelper.lua
```
如果文件没有 `---<!export>` 标记, todoc 会提示并跳过

#### --refresh

重新扫描 sources 中的所有文件并更新缓存:
```bash
todoc --refresh
```
等效于对所有源文件做一次全量解析

#### 其他参数

```bash
todoc --dir <path>       # 覆盖配置中的 workspace
todoc --targetout <path> # 覆盖配置中的 doc_target_out_dir
todoc --config <path>    # 指定配置文件路径, 默认 .todoc/todoc.json
```
