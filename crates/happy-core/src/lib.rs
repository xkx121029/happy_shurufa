//! Happy 输入法纯逻辑核心。
//!
//! 本 crate 不依赖任何 Windows API，所有逻辑均可单测：
//! - [`mode`]：中英文模式状态机（内置进程规则 + 用户按进程记忆）
//! - [`punct`]：中文标点 → 英文标点映射
//! - [`pinyin`]：内嵌词库拼音引擎（二分前缀查找 + 贪音节分段）
//! - [`lexicon`]：编程命令/片段联想（内置词库 + 频率排序）
//! - [`freq`]：频率学习（持久化 JSON）
//! - [`config`]：配置加载/保存（%APPDATA%\happy_shurufa\conf.json）
//! - [`ai`]：可选 LLM 联想（独立线程 + mpsc 回传，超时静默丢弃）

pub mod ai;
pub mod config;
pub mod freq;
pub mod lexicon;
pub mod mode;
pub mod punct;
pub mod pinyin;
