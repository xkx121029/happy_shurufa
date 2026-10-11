//! 内嵌词库拼音引擎。
//!
//! 词库格式（pinyin.dict，按键排序的 TSV）：
//! `无声调拼音串\t候选1|候选2|...`
//!
//! 查找策略：
//! 1. 构建期词库按键排序，运行时惰性建立行偏移索引
//! 2. 对原始输入串做前缀二分搜索（输入 "nihao" 命中 nihao 行，"niha" 命中其前缀行）
//! 3. 精确匹配的 key 排最前，其余按 key 长度升序（越短越接近完整拼音）

use std::sync::OnceLock;

const DICT: &str = include_str!("../res/pinyin.dict");
const SYLLABLES: &str = include_str!("../res/syllables.txt");

/// 惰性索引：词库各行 (key, values) 的切片，按 key 排序。
fn index() -> &'static Vec<(&'static str, &'static str)> {
    static INDEX: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    INDEX.get_or_init(|| {
        DICT
            .lines()
            .filter_map(|line| {
                let (key, values) = line.split_once('\t')?;
                Some((key, values))
            })
            .collect()
    })
}

/// 合法音节集合（用于贪心分段与有效性判断）。
fn syllable_set() -> &'static Vec<&'static str> {
    static SET: OnceLock<Vec<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set: Vec<&'static str> = SYLLABLES.lines().filter(|l| !l.is_empty()).collect();
        set.sort_unstable();
        set
    })
}

pub struct PinyinEngine;

impl PinyinEngine {
    /// 对输入串联想候选。`limit` 为最大候选数。
    pub fn suggest(input: &str) -> Vec<String> {
        Self::suggest_with(input, 9)
    }

    pub fn suggest_with(input: &str, limit: usize) -> Vec<String> {
        let input = input.trim().to_lowercase();
        if input.is_empty() || limit == 0 {
            return Vec::new();
        }
        let dict = index();
        let mut out: Vec<String> = Vec::with_capacity(limit);

        // 1. 精确匹配的 key 优先
        if let Ok(pos) = dict.binary_search_by(|(k, _)| (*k).cmp(input.as_str())) {
            push_values(&mut out, dict[pos].1, limit);
        }
        if out.len() >= limit {
            return out;
        }

        // 2. 前缀行：取所有以 input 开头的 key，按 (key 长度, 原序) 收集
        //    词库按 key 字典序排列，前缀行是连续区间，但需要按 key 长度排优先级，
        //    所以限制扫描窗口，收集后统一排序。
        let mut rows: Vec<(usize, &str)> = Vec::new();
        let start = lower_bound(dict, &input);
        for (key, values) in &dict[start..] {
            if !key.starts_with(&input) || rows.len() >= 400 {
                break;
            }
            rows.push((key.len(), values));
        }
        rows.sort_by_key(|(len, _)| *len);
        for (_, values) in rows {
            if out.len() >= limit {
                break;
            }
            push_values(&mut out, values, limit);
        }
        out
    }

    /// 贪心音节分段（用于显示分隔与 AI 上下文提示）。
    /// 无法分段的部分原样返回。
    pub fn segment(input: &str) -> Vec<String> {
        let syllables = syllable_set();
        let mut out = Vec::new();
        let bytes: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            let mut matched = 0;
            for len in (1..=6).rev() {
                if i + len > bytes.len() {
                    continue;
                }
                let piece: String = bytes[i..i + len].iter().collect();
                if syllables.binary_search(&piece.as_str()).is_ok() {
                    out.push(piece);
                    matched = len;
                    break;
                }
            }
            if matched == 0 {
                // 未识别的单字符原样保留
                out.push(bytes[i].to_string());
                matched = 1;
            }
            i += matched;
        }
        out
    }
}

fn lower_bound(dict: &[(&str, &str)], key: &str) -> usize {
    let mut lo = 0usize;
    let mut hi = dict.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if dict[mid].0 < key {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

fn push_values(out: &mut Vec<String>, values: &str, limit: usize) {
    for word in values.split('|') {
        if out.len() >= limit {
            return;
        }
        // 词库单音节条目按码位罗列，开头往往是扩展区生僻字（㒾、𠀍 等）。
        // 过滤掉这些生僻区，常用字自然靠前，避免「是/时/事」被挤出候选。
        if !word.is_empty() && word.chars().all(is_common_char) {
            out.push(word.to_string());
        }
    }
}

/// 是否属于适合作为候选的常用汉字。
/// 排除 CJK 扩展 A（U+3400–U+4DBF）、扩展 B~F（非 BMP，U+20000 起）
/// 以及兼容汉字区（U+F900–U+FAFF）——这些几乎都是生僻字。
fn is_common_char(c: char) -> bool {
    let v = c as u32;
    !(0x3400..=0x4DBF).contains(&v)
        && !(0x20000..=0x2FA1F).contains(&v)
        && !(0xF900..=0xFAFF).contains(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 完整拼音() {
        let got = PinyinEngine::suggest("nihao");
        assert!(!got.is_empty(), "nihao 应有候选");
        assert!(got.iter().any(|c| c == "你好"), "候选应包含你好: {got:?}");
    }

    #[test]
    fn 不完整拼音前缀() {
        let got = PinyinEngine::suggest("niha");
        assert!(!got.is_empty(), "nihao 前缀应命中");
        assert!(got.iter().any(|c| c == "你好"));
    }

    #[test]
    fn 单字() {
        let got = PinyinEngine::suggest("shi");
        assert!(got.iter().any(|c| c == "是" || c == "时" || c == "事"));
    }

    #[test]
    fn 空输入与垃圾输入() {
        assert!(PinyinEngine::suggest("").is_empty());
        assert!(PinyinEngine::suggest("zzzzzzz").is_empty());
    }

    #[test]
    fn 分段() {
        let seg = PinyinEngine::segment("nihaoshijie");
        assert_eq!(seg, vec!["ni", "hao", "shi", "jie"]);
    }
}
