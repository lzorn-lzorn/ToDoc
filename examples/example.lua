--- ToDoc Lua 示例文件
--- 展示注释解析能力

local util = require("util")
require 'mod.alpha'

---@brief: 计算两个数字之和
---@param \name{a} \type{number} \content[markdown]{第一个数}
---@param \name{b} \type{number} \default{0} 第二个数
---@return \type{number} \content{返回求和结果}
---@note \content{这是一个示例函数}
---@todo \content{后续支持整数溢出检查}
---@export
local function add(a, b)
  return a + b
end

---@brief \content[markdown]{模块方法 **ping**}
function M:ping(name)
  return "pong " .. name
end
