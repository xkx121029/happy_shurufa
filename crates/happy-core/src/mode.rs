//! 中英文模式状态机。
//!
//! 切换依据（优先级从高到低）：
//! 1. 用户在本进程内按 Shift 手动切换的记忆（持久化）
//! 2. 内置进程规则（终端/IDE → 英文，聊天/浏览器 → 中文）
//! 3. 未知进程默认中文

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Chinese,
    English,
}

impl Mode {
    pub fn flipped(self) -> Self {
        match self {
            Mode::Chinese => Mode::English,
            Mode::English => Mode::Chinese,
        }
    }
}

/// 进程默认规则表（可被配置追加）。键为可执行文件名小写，值为子串匹配规则。
pub const BUILTIN_EN_PROCESSES: &[&str] = &[
    // 终端
    "windowsterminal", "openconsole", "conhost", "cmd", "powershell", "pwsh",
    "wt", "alacritty", "wezterm", "mintty", "hyper", "putty", "mstsc",
    // 编辑器 / IDE
    "code", "devenv", "sublime_text", "idea64", "clion64", "pycharm64",
    "goland64", "webstorm64", "rider64", "datagrip64", "phpstorm64",
    "studio64", "notepad++.exe", "gvim", "nvim", "emacs", "codeblocks",
    "eclipse", "dejavu", "vim",
    // 开发工具
    "git-", "gitx", "postman", " insomnia", "dbeaver", "heidisql",
    "docker", "procexp", "procmon", "wireshark", "virtualbox", "vmware",
    "regedit", "putty64",
];

pub const BUILTIN_CN_PROCESSES: &[&str] = &[
    "wechat", "weixin", "qq", "wetype", "dingtalk", "dingding", "feishu",
    "lark", "telegram", "discord", "slack", "thunder", "chrome", "msedge",
    "firefox", "opera", "brave", "vivaldi", "360se", "360chrome", "qqbrowser",
    "sogou_explorer", "notepad", "explorer", "onenote", "evernote", "word",
    "winword", "excel", "powerpnt", "onenotep", "wps", "et", "wpp",
    "cloudmusic", "kugou", "kuwo", "qqmusic", "bilibili", "steam",
];

pub fn builtin_default(exe_lower: &str) -> Mode {
    if BUILTIN_EN_PROCESSES.iter().any(|k| exe_lower.contains(k)) {
        return Mode::English;
    }
    if BUILTIN_CN_PROCESSES.iter().any(|k| exe_lower.contains(k)) {
        return Mode::Chinese;
    }
    Mode::Chinese
}

/// 用户手动切换记忆（进程 → 模式），持久化到 modes.json。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ModeMemory {
    pub manual: BTreeMap<String, Mode>,
}

impl ModeMemory {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
            let _ = std::fs::write(path, json);
        }
    }
}

/// 模式状态机。
pub struct ModeState {
    pub memory: ModeMemory,
    pub process: String,
    pub current: Mode,
    /// 会话内临时禁用手动记忆（如配置关闭学习时）
    pub learning: bool,
}

impl ModeState {
    pub fn new(learning: bool) -> Self {
        Self {
            memory: ModeMemory::default(),
            process: String::new(),
            current: Mode::Chinese,
            learning,
        }
    }

    /// 前台进程变化时调用：按 记忆 > 内置规则 > 默认 决定模式。
    pub fn on_process_change(&mut self, exe_lower: &str) {
        if self.process == exe_lower {
            return;
        }
        self.process = exe_lower.to_string();
        self.current = self.mode_for_process(exe_lower);
    }

    pub fn mode_for_process(&self, exe_lower: &str) -> Mode {
        if let Some(m) = self.memory.manual.get(exe_lower) {
            return *m;
        }
        builtin_default(exe_lower)
    }

    /// Shift 手动切换：翻转并记录到当前进程记忆。
    /// 返回切换后的新模式。
    pub fn toggle(&mut self) -> Mode {
        self.current = self.current.flipped();
        if self.learning && !self.process.is_empty() {
            self.memory.manual.insert(self.process.clone(), self.current);
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 内置规则() {
        assert_eq!(builtin_default("windowsterminal.exe"), Mode::English);
        assert_eq!(builtin_default("code.exe"), Mode::English);
        assert_eq!(builtin_default("pwsh.exe"), Mode::English);
        assert_eq!(builtin_default("wechatappex.exe"), Mode::Chinese);
        assert_eq!(builtin_default("msedge.exe"), Mode::Chinese);
        assert_eq!(builtin_default("unknown.exe"), Mode::Chinese);
    }

    #[test]
    fn 进程切换与手动记忆() {
        let mut state = ModeState::new(true);
        state.on_process_change("windowsterminal.exe");
        assert_eq!(state.current, Mode::English);
        // 手动切换成中文并记忆
        assert_eq!(state.toggle(), Mode::Chinese);
        state.on_process_change("msedge.exe");
        assert_eq!(state.current, Mode::Chinese);
        // 回到终端，应用记忆
        state.on_process_change("windowsterminal.exe");
        assert_eq!(state.current, Mode::Chinese);
        // 关闭学习后切换不记忆
        state.learning = false;
        state.toggle();
        state.on_process_change("code.exe");
        assert_eq!(state.current, Mode::English);
    }

    #[test]
    fn 记忆序列化() {
        let mut mem = ModeMemory::default();
        mem.manual.insert("code.exe".into(), Mode::Chinese);
        let json = serde_json::to_string(&mem).unwrap();
        let back: ModeMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.manual["code.exe"], Mode::Chinese);
    }
}
