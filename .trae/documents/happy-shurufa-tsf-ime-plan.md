# Happy 输入法 — Windows 系统级编程专用输入法实现计划

## Context

用户要制作一款专为编程打字服务的 Windows 系统级输入法（TSF 框架），核心能力：
1. **自动中英文切换**：按前台进程 + 用户习惯记忆，终端/IDE 默认英文，聊天/浏览默认中文，Shift 手动切换后按进程记忆
2. **中文标点自动转英文**：代码上下文中全角标点（，。；：（）""等）映射为半角
3. **命令联想**：本地词库（git/npm/docker/PowerShell 常用命令+片段）+ 频率学习，可选接入 LLM API 增强
4. **基础拼音输入**：中文模式内嵌小词库（约 5 万高频字词 TSV），不追求完整输入体验
5. **候选悬浮窗**：纯色背景、圆角、细边框、无衬线字体、从下到上进入动画、跟随光标

技术环境：空目录；Rust stable 1.94 (x86_64-pc-windows-msvc)、VS Community 2026、CMake 4.3 已装。

## 技术选型

- **纯 Rust + windows-rs 0.62.x**（锁死 minor 版本）实现 TSF COM DLL。已验证先例：`dec32/ajemi`（注册代码直接参照）、`tsuyoshi-otake/sakura-input`
- windows features：`Win32_UI_TextServices`、`Win32_UI_WindowsAndMessaging`、`Win32_UI_Input_KeyboardAndMouse`、`Win32_Graphics_GDI`、`Win32_Graphics_Dwm`、`Win32_System_Com`、`Win32_System_Registry`、`Win32_System_Threading`、`Win32_UI_HiDpi`
- 持久化：`serde` + `serde_json`（%APPDATA%\happy_shurufa 下 conf.json + freq.json）
- 日志：`log` + 自写轻量文件 logger（DLL 崩溃排查刚需，禁 tracing 等重依赖）
- LLM HTTP：`ureq`（rustls），仅独立线程调用
- 悬浮窗：GDI 绘制 + `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE=ROUND)` 圆角，不引入 Direct2D

## Workspace 结构

```
happy_shurufa/
├── Cargo.toml                  # workspace 根
├── crates/
│   ├── happy-core/             # 纯逻辑 lib，零 Windows 依赖，cargo test 全覆盖
│   │   └── src/                # mode 状态机、拼音引擎、标点映射、命令联想+频率、config
│   ├── happy-ime/              # cdylib，TSF 粘合层
│   │   └── src/
│   │       ├── lib.rs          # DllMain/DllGetClassObject/DllRegisterServer 等导出
│   │       ├── register.rs     # 注册逻辑（参照 ajemi 三步注册）
│   │       ├── tsf/            # COM 接口实现
│   │       ├── window.rs       # 候选悬浮窗
│   │       └── context.rs      # 进程探测
│   └── happy-installer/        # bin 安装器 + scripts/install.ps1
└── docs/                       # 配置/API 文档（不入网页）
```

## TSF 接口清单（happy-ime）

- DLL 导出：`DllMain`（仅缓存 HINSTANCE）、`DllGetClassObject`（IClassFactory）、`DllCanUnloadNow`、`DllRegisterServer/UnregisterServer`
- `ITfTextInputProcessor(Ex)`：Activate/Deactivate，advise sinks、注册 PreservedKey、进程探测定模式
- `ITfKeyEventSink`：按键拦截；Shift 裸按 PRESERVED_KEY 切换中英
- `ITfCompositionSink` + `ITfComposition`：composition 生命周期，GetTextExt 定位悬浮窗
- `ITfEditSession`（多个实现）：一切文本读写的唯一通道，READ/READWRITE 分开
- `ITfDisplayAttributeProvider`：composition 下划线（P2 可后置）
- `ITfThreadMgrEventSink` / `ITfTextLayoutSink`：焦点变化重探进程、布局变化重定位
- `ITfCompartmentEventSink`：与系统托盘中英态同步
- **省略**：ITfFnConfigure、语言栏按钮（托盘图标由 AddLanguageProfile 内嵌 .ico 提供）

**提交流程**：OnKeyDown → `RequestEditSession(TF_ES_SYNC|TF_ES_READWRITE)` → StartComposition + SetText 预编辑 → 候选确认 SetText 终稿 + EndComposition。读上下文用独立 READ session。

## 注册与安装

1. HKLM `SOFTWARE\Classes\CLSID\{CLSID}` + InProcServer32（Apartment，GetModuleFileNameW 取路径）
2. CoCreateInstance `ITfInputProcessorProfiles`：Register + AddLanguageProfile（langid 0x0804）
3. `ITfCategoryMgr` 注册 16 项类别组（TIP_KEYBOARD、UIELEMENTENABLED、IMMERSIVESUPPORT 等）
4. install.ps1（提权）：拷贝 DLL 到 %ProgramFiles% → regsvr32 /s → 提示 Win+Space 选择
5. **P0 实验**：per-user 免管理员路径（HKCU\Software\Classes + HKCU\Software\Microsoft\CTF\TIP），失败回退 HKLM

## 关键实现要点

- **模式状态机**：输入 = (内置进程规则表, 用户 Shift 记忆, 会话态)。探测用 GetForegroundWindow + QueryFullProcessImageNameW。DLL 活在宿主进程内无 UAC 边界问题；AppContainer 下所有 IO 静默降级
- **标点映射**：约 30 项全角→半角查表，纯函数单测覆盖
- **命令联想**：内置 TSV 词库，前缀+子序列匹配 + 频率排序；LLM 走独立线程 + mpsc，request_id 防串台，800ms 超时静默丢弃
- **悬浮窗**：WS_POPUP layered + TOPMOST|NOACTIVATE|TOOLWINDOW；`ITfContextView::GetTextExt` 定位（注意 DPI 物理像素换算 GetDpiForWindow）；SetTimer 动画 120ms ease-out 自下而上
- **稳定性**：DLL 注入所有文本进程，FFI 边界全部 catch_unwind，禁 unwrap，防御式返回 S_OK

## 分阶段计划（每阶段独立验证）

| 阶段 | 内容 | 验证方法 |
|---|---|---|
| P0 | COM 骨架 + 注册 + 直通英文 | regsvr32 成功；Win+Space 出现"Happy 输入法"；notepad 不崩 |
| P1 | Shift 切换 + 进程探测 + 记忆持久化 | 终端默认 EN、记事本默认 CN；切换记忆生效 |
| P2 | composition + 悬浮窗 + 最小拼音引擎 | 打 nihong → 候选窗跟随光标、上屏汉字 |
| P3 | 标点转换 + 命令联想 + 频率持久化 | 终端敲 git che → 联想 checkout；逗号自动半角 |
| P4 | LLM 可选集成 + 配置项 | 断网/超时不卡键入 |
| P5 | 安装器 + 动画/DPI 打磨 + 文档 | 全流程安装/卸载验证 |

## 风险

1. DLL panic 崩宿主 → catch_unwind + 防御式编程（最高优先）
2. per-user 注册无官方背书 → P0 实验决定，失败回退 HKLM
3. 未签名 DLL 被 Defender 拦截 → 自用加排除；开源后建议签名

## 首次完成后

按用户规则询问是否开源（GitHub + Gitee，附 README）。
