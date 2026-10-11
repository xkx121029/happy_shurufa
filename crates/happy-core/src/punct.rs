//! 中文标点 → 英文标点映射。
//!
//! 编程场景下无论中英模式，符号一律输出半角（可配置）。

/// 全角标点 → 半角标点。
pub fn to_ascii(ch: char) -> Option<char> {
    let mapped = match ch {
        '，' => ',',
        '。' => '.',
        '．' => '.',
        '、' => '\\',
        '：' => ':',
        '；' => ';',
        '？' => '?',
        '！' => '!',
        '（' => '(',
        '）' => ')',
        '【' => '[',
        '】' => ']',
        '〔' => '(',
        '〕' => ')',
        '｛' => '{',
        '｝' => '}',
        '《' => '<',
        '》' => '>',
        '〈' => '<',
        '〉' => '>',
        '“' => '"',
        '”' => '"',
        '‘' => '\'',
        '’' => '\'',
        '「' => '[',
        '」' => ']',
        '『' => '[',
        '』' => ']',
        '～' => '~',
        '—' => '-',
        '－' => '-',
        '・' => '@',
        '·' => '`',
        '％' => '%',
        '＃' => '#',
        '＆' => '&',
        '＊' => '*',
        '＋' => '+',
        '＝' => '=',
        '＠' => '@',
        '￥' => '$',
        '｜' => '|',
        '＂' => '"',
        '＇' => '\'',
        _ => return None,
    };
    Some(mapped)
}

/// 输入字符若为全角标点则转换，否则原样返回。
pub fn normalize(ch: char) -> char {
    to_ascii(ch).unwrap_or(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 常用标点映射() {
        assert_eq!(to_ascii('，'), Some(','));
        assert_eq!(to_ascii('。'), Some('.'));
        assert_eq!(to_ascii('：'), Some(':'));
        assert_eq!(to_ascii('；'), Some(';'));
        assert_eq!(to_ascii('（'), Some('('));
        assert_eq!(to_ascii('）'), Some(')'));
        assert_eq!(to_ascii('“'), Some('"'));
        assert_eq!(to_ascii('’'), Some('\''));
        assert_eq!(to_ascii('、'), Some('\\'));
    }

    #[test]
    fn 非标点原样() {
        assert_eq!(to_ascii('a'), None);
        assert_eq!(to_ascii('汉'), None);
        assert_eq!(normalize('汉'), '汉');
        assert_eq!(normalize('，'), ',');
    }
}
