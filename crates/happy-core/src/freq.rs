//! 频率学习：记录用户提交的候选/命令，用于联想排序，持久化到 freq.json。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FreqMap(BTreeMap<String, u32>);

impl FreqMap {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(json) = serde_json::to_vec(self) {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
            let _ = std::fs::write(path, json);
        }
    }

    /// 记一次使用。
    pub fn bump(&mut self, key: &str) {
        if key.is_empty() {
            return;
        }
        *self.0.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn get(&self, key: &str) -> u32 {
        self.0.get(key).copied().unwrap_or(0)
    }

    /// 缩减体积：仅保留次数 >= 2 的条目。
    pub fn compact(&mut self) {
        self.0.retain(|_, v| *v >= 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 计数与持久化() {
        let dir = std::env::temp_dir().join("happy_shurufa_test_freq");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("freq.json");

        let mut freq = FreqMap::default();
        freq.bump("git checkout");
        freq.bump("git checkout");
        freq.bump("git status");
        freq.save(&path);

        let loaded = FreqMap::load(&path);
        assert_eq!(loaded.get("git checkout"), 2);
        assert_eq!(loaded.get("git status"), 1);
        assert_eq!(loaded.get("git push"), 0);
    }

    #[test]
    fn 紧凑化() {
        let mut freq = FreqMap::default();
        freq.bump("a");
        freq.bump("b");
        freq.bump("b");
        freq.compact();
        assert_eq!(freq.get("a"), 0);
        assert_eq!(freq.get("b"), 2);
    }
}
