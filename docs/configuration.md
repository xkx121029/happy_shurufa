# Happy 输入法 · 配置文档

配置文件路径：`%APPDATA%\happy_shurufa\conf.json`（JSON，UTF-8）。
文件不存在或字段缺失时自动使用默认值，无需写全。

## 完整示例

```json
{
  "font_name": "Microsoft YaHei UI",
  "font_size": 12,
  "candidate_count": 9,
  "halfwidth_punct": true,
  "builtin_rules": true,
  "mode_learning": true,
  "command_suggest": true,
  "ai": {
    "enabled": false,
    "base_url": "https://api.example.com/v1/chat/completions",
    "api_key": "sk-xxx",
    "model": "gpt-4o-mini",
    "trigger_len": 4,
    "timeout_ms": 800
  },
  "process_rules": {
    "code": "en",
    "idea64": "en",
    "notepad": "cn"
  }
}
```

## 字段说明

| 字段 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `font_name` | 字符串 | `Microsoft YaHei UI` | 候选窗字体（无衬线） |
| `font_size` | 整数 | `12` | 候选窗字号（pt，随 DPI 缩放） |
| `candidate_count` | 整数 | `9` | 候选窗最大条目数（数字键 1-9 选择） |
| `halfwidth_punct` | 布尔 | `true` | 中文模式下标点一律输出半角（编程专用） |
| `builtin_rules` | 布尔 | `true` | 启用内置进程规则（IDE/终端 → 英文，聊天/文档 → 中文） |
| `mode_learning` | 布尔 | `true` | 学习 Shift 手动切换：记住你在某进程最后一次选择的中英模式 |
| `command_suggest` | 布尔 | `true` | 英文模式下根据上下文联想命令/标识符（Tab 或数字键补全） |
| `ai` | 对象 | 见下 | 可选 LLM 联想 |
| `process_rules` | 对象 | 空 | 用户进程规则：`"exe 名小写子串": "cn" 或 "en"`，优先级高于内置规则 |

## ai 子字段

| 字段 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `enabled` | 布尔 | `false` | 是否启用 AI 联想（关闭时纯本地词库联想） |
| `base_url` | 字符串 | 空 | OpenAI 兼容 chat/completions 接口地址 |
| `api_key` | 字符串 | 空 | API Key（仅存本机） |
| `model` | 字符串 | 空 | 模型名 |
| `trigger_len` | 整数 | `4` | 输入 token 达到该长度后才触发 AI 请求 |
| `timeout_ms` | 整数 | `800` | 请求超时；超时或失败时静默降级为本地联想 |

## 运行时数据（自动生成，无需手改）

- `%APPDATA%\happy_shurufa\mode.json` —— 各进程的模式记忆
- `%APPDATA%\happy_shurufa\freq.json` —— 命令联想词频学习

## 快捷键与交互

| 按键 | 行为 |
| --- | --- |
| `Shift`（单击/key-up） | 中英切换（当前输入内容原样上屏） |
| 字母 | 中文模式进入拼音；英文模式放行并触发命令联想 |
| `1`-`9` | 选择候选（中文候选 / 英文命令联想） |
| `空格` | 选择第一候选；无拼音候选时原样上屏 |
| `回车` | 拼音原样上屏 |
| `Tab` | 英文模式提交第一命令联想 |
| `退格` | 回删拼音字母；删空自动取消组合 |
| `Esc` | 取消组合 / 关闭联想窗 |
