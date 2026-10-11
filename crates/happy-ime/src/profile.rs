//! 用户数据持久化：模式记忆 modes.json + 频率 freq.json。

use std::path::PathBuf;

use happy_core::config::data_dir;
use happy_core::freq::FreqMap;
use happy_core::mode::ModeMemory;

fn modes_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("modes.json"))
}

fn freq_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("freq.json"))
}

/// 全部 IO 失败静默降级（AppContainer 宿主下可能被拒绝）。
pub fn load_modes() -> ModeMemory {
    modes_path()
        .map(|p| ModeMemory::load(&p))
        .unwrap_or_default()
}

pub fn save_modes(memory: &ModeMemory) {
    if let Some(p) = modes_path() {
        memory.save(&p);
    }
}

pub fn load_freq() -> FreqMap {
    freq_path()
        .map(|p| FreqMap::load(&p))
        .unwrap_or_default()
}

pub fn save_freq(freq: &FreqMap) {
    if let Some(p) = freq_path() {
        freq.save(&p);
    }
}
