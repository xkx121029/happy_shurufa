//! 配置：加载/保存到 %APPDATA%\happy_shurufa\conf.json。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// LLM 联想配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// 是否启用 AI 联想（默认关闭）
    pub enabled: bool,
    /// OpenAI 兼容接口地址，如 https://api.example.com/v1/chat/completions
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// 触发联想的最小 token 长度
    pub trigger_len: usize,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            trigger_len: 4,
            timeout_ms: 800,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 候选窗字体（无衬线）
    pub font_name: String,
    /// 候选窗字号（pt）
    pub font_size: i32,
    /// 最大候选数
    pub candidate_count: usize,
    /// 中文模式下标点一律输出半角（编程专用，默认开启）
    pub halfwidth_punct: bool,
    /// 是否启用进程规则自动切换
    pub builtin_rules: bool,
    /// 是否学习用户手动切换（Shift 记忆）
    pub mode_learning: bool,
    /// 是否启用命令联想
    pub command_suggest: bool,
    /// AI 联想
    pub ai: AiConfig,
    /// 用户追加的进程规则（exe 小写子串 → cn/en）
    pub process_rules: std::collections::BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_name: "Microsoft YaHei UI".into(),
            font_size: 12,
            candidate_count: 9,
            halfwidth_punct: true,
            builtin_rules: true,
            mode_learning: true,
            command_suggest: true,
            ai: AiConfig::default(),
            process_rules: Default::default(),
        }
    }
}

/// 配置与数据目录：%APPDATA%\happy_shurufa。
pub fn data_dir() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(appdata).join("happy_shurufa"))
}

pub fn config_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("conf.json"))
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Self {
        match std::fs::read(path) {
            Ok(data) => match serde_json::from_slice::<Config>(&data) {
                Ok(mut conf) => {
                    conf.normalize_rules();
                    conf
                }
                Err(e) => {
                    log::warn!("配置文件解析失败，使用默认配置: {e}");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = config_path() else {
            return Ok(());
        };
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, json)
    }

    /// 进程规则键统一小写；值仅接受 cn/en。
    fn normalize_rules(&mut self) {
        self.process_rules.retain(|_, v| {
            let v = v.to_lowercase();
            v == "cn" || v == "en"
        });
    }

    /// 查询进程模式规则（用户规则优先于内置规则）。
    pub fn rule_for(&self, exe_lower: &str) -> Option<bool> {
        // true = English, false = Chinese
        for (k, v) in &self.process_rules {
            if exe_lower.contains(k.as_str()) {
                return Some(v == "en");
            }
        }
        if !self.builtin_rules {
            return None;
        }
        match crate::mode::builtin_default(exe_lower) {
            crate::mode::Mode::English => Some(true),
            crate::mode::Mode::Chinese => Some(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 默认配置序列化往返() {
        let conf = Config::default();
        let json = serde_json::to_string(&conf).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert!(back.halfwidth_punct);
        assert_eq!(back.font_name, "Microsoft YaHei UI");
    }

    #[test]
    fn 兼容缺省字段() {
        let back: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(back.candidate_count, 9);
        assert!(!back.ai.enabled);
    }

    #[test]
    fn 规则查询() {
        let mut conf = Config::default();
        conf.process_rules.insert("myapp".into(), "en".into());
        assert_eq!(conf.rule_for("myapp.exe"), Some(true));
        assert_eq!(conf.rule_for("code.exe"), Some(true));
        assert_eq!(conf.rule_for("wechat.exe"), Some(false));
        conf.builtin_rules = false;
        assert_eq!(conf.rule_for("code.exe"), None);
        assert_eq!(conf.rule_for("myapp.exe"), Some(true));
    }
}
