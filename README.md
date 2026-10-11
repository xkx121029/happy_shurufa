# Happy 输入法 · 专用于编程打字的 Windows 输入法

一款 Windows 系统级（TSF）输入法，专为**编程场景**打造：

- **自动中英切换**：按前台进程自动判定（IDE/终端 → 英文，聊天/文档 → 中文），并学习你的 Shift 手动切换习惯，同一进程下次自动恢复。
- **中文符号自动转英文符号**：中文模式下标点一律输出半角，写代码不再被打断（`，`→`,`、`（）`→`()` 等）。
- **命令联想**：英文模式下根据光标前上下文联想命令/标识符，`Tab` 或数字键补全，本地词库 + 可选 LLM。
- **纯本地优先**：词库、词频学习全部本地完成；LLM 联想为可选，超时自动降级。

## 功能速览

| 场景 | 行为 |
| --- | --- |
| 切到 VS Code / 终端 | 自动切英文模式 |
| 切到 微信 / 记事本 | 自动切中文模式 |
| 中文模式打 `if` + 空格 | 无匹配拼音则 `if` 原样上屏，标点全半角 |
| 中文模式打 `nihao` | 候选窗出「你好」，数字键选择 |
| 英文模式打 `for` | 联想窗提示 `for`、`foreach`、`format`，Tab 补全 |
| Shift 单击 | 手动切换中英，且被记忆 |

## 安装

需要 Windows 10 1809+ / Windows 11 与 [Rust](https://rustup.rs) 工具链。

```powershell
# 管理员 PowerShell（推荐，注册到 HKLM）
.\scripts\install.ps1 -Release

# 或无提权安装（注册到 HKCU，仅当前用户可用）
.\scripts\install.ps1
```

安装完成后按 `Win + Space` 切换到「Happy 输入法」。

卸载：

```powershell
.\scripts\uninstall.ps1
```

## 配置

配置文件：`%APPDATA%\happy_shurufa\conf.json`（自动生成默认值，见 [docs/configuration.md](docs/configuration.md)）。

## 构建

```powershell
cargo build --release        # 产物 target\release\happy_ime.dll
cargo test -p happy-core     # 纯逻辑单元测试
```

## 项目结构

```
crates/
├── happy-core/       纯逻辑库：模式状态机 / 拼音引擎 / 标点转换 / 命令词库 / 频率学习 / 配置 / LLM 客户端
├── happy-ime/        TSF 粘合层：COM DLL（cdylib）、edit session、按键分发、候选悬浮窗
└── happy-installer/  安装/卸载器
scripts/              install.ps1 / uninstall.ps1 / gen-dict.ps1（词库生成）
docs/                 configuration.md（配置文档）
```

## 技术要点

- **纯 Rust + windows-rs 0.62**，无 C++ 依赖；COM 线程模型 Apartment。
- 所有文档读写经 **edit session** 完成，按键处理走 `ITfKeyEventSink`，预编辑经 `ITfComposition`。
- 候选窗为独立的 `WS_EX_NOACTIVATE | WS_EX_TOPMOST` 窗口：DWM 圆角、纯色、无衬线字体、自下而上 120ms ease-out 进入动画。
- FFI 出口全部带 `catch_unwind` 防护——DLL 注入所有文本进程，一次 panic 都不能崩宿主。

## 兼容性说明

- 部分 IDE（尤其自绘 UI 的 JetBrains 系列）对 TSF 支持有限，若候选窗不显示请反馈 issue。
- 单元测试 exe 可能被杀毒软件误报拦截，可将项目目录加入白名单。

## License

MIT
