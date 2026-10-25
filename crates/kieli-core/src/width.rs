//! 文字の幅の変換。

/// ASCII の図形文字(U+0021〜U+007E)を全角(U+FF01〜U+FF5E)にする。ほかの文字は変えない。
///
/// 半角と全角の英数字・記号は同じ並びで対応しているので、0xFEE0 を足せば全角になる。
pub(crate) fn to_full_width(c: char) -> char {
    match c {
        '!'..='~' => char::from_u32(c as u32 + 0xFEE0).unwrap_or(c),
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_becomes_full_width() {
        assert_eq!(to_full_width('a'), 'ａ');
        assert_eq!(to_full_width('Z'), 'Ｚ');
        assert_eq!(to_full_width('0'), '０');
        assert_eq!(to_full_width('!'), '！');
        assert_eq!(to_full_width('~'), '～');
    }

    #[test]
    fn other_characters_keep_their_width() {
        for c in [' ', 'あ', 'ー', '\u{3000}', 'ａ', '\u{7f}', '\t', '𠮷'] {
            assert_eq!(to_full_width(c), c);
        }
    }
}
