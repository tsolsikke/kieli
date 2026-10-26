//! ローマ字からひらがなへのテーブル(設計文書の 1)。
//!
//! テーブルは Kieli の側で一から書いたもの。変わらないデータで、大域の状態ではない。

/// テーブルの 1 行。`input` を打つと `output` を出し、`rest` を保留のローマ字として残す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rule {
    pub(crate) input: &'static str,
    pub(crate) output: &'static str,
    pub(crate) rest: &'static str,
}

const fn r(input: &'static str, output: &'static str) -> Rule {
    Rule {
        input,
        output,
        rest: "",
    }
}

/// 促音:子音を重ねたら「っ」を出し、後の子音を保留に残す。
const fn sokuon(input: &'static str, rest: &'static str) -> Rule {
    Rule {
        input,
        output: "っ",
        rest,
    }
}

/// ローマ字のテーブル。
#[rustfmt::skip]
pub(crate) const RULES: &[Rule] = &[
    // 母音
    r("a", "あ"), r("i", "い"), r("u", "う"), r("e", "え"), r("o", "お"),
    // か行・が行
    r("ka", "か"), r("ki", "き"), r("ku", "く"), r("ke", "け"), r("ko", "こ"),
    r("kya", "きゃ"), r("kyi", "きぃ"), r("kyu", "きゅ"), r("kye", "きぇ"), r("kyo", "きょ"),
    r("ga", "が"), r("gi", "ぎ"), r("gu", "ぐ"), r("ge", "げ"), r("go", "ご"),
    r("gya", "ぎゃ"), r("gyi", "ぎぃ"), r("gyu", "ぎゅ"), r("gye", "ぎぇ"), r("gyo", "ぎょ"),
    // c で始まる綴り(か行・さ行・ち)
    r("ca", "か"), r("ci", "し"), r("cu", "く"), r("ce", "せ"), r("co", "こ"),
    r("cha", "ちゃ"), r("chi", "ち"), r("chu", "ちゅ"), r("che", "ちぇ"), r("cho", "ちょ"),
    r("cya", "ちゃ"), r("cyi", "ちぃ"), r("cyu", "ちゅ"), r("cye", "ちぇ"), r("cyo", "ちょ"),
    // q で始まる綴り
    r("qa", "くぁ"), r("qi", "くぃ"), r("qu", "く"), r("qe", "くぇ"), r("qo", "くぉ"),
    // さ行・ざ行・じ
    r("sa", "さ"), r("si", "し"), r("su", "す"), r("se", "せ"), r("so", "そ"),
    r("shi", "し"), r("sha", "しゃ"), r("shu", "しゅ"), r("she", "しぇ"), r("sho", "しょ"),
    r("sya", "しゃ"), r("syi", "しぃ"), r("syu", "しゅ"), r("sye", "しぇ"), r("syo", "しょ"),
    r("za", "ざ"), r("zi", "じ"), r("zu", "ず"), r("ze", "ぜ"), r("zo", "ぞ"),
    r("zya", "じゃ"), r("zyi", "じぃ"), r("zyu", "じゅ"), r("zye", "じぇ"), r("zyo", "じょ"),
    r("ja", "じゃ"), r("ji", "じ"), r("ju", "じゅ"), r("je", "じぇ"), r("jo", "じょ"),
    r("jya", "じゃ"), r("jyi", "じぃ"), r("jyu", "じゅ"), r("jye", "じぇ"), r("jyo", "じょ"),
    // た行・だ行
    r("ta", "た"), r("ti", "ち"), r("tu", "つ"), r("te", "て"), r("to", "と"),
    r("tsu", "つ"), r("tsa", "つぁ"), r("tsi", "つぃ"), r("tse", "つぇ"), r("tso", "つぉ"),
    r("tya", "ちゃ"), r("tyi", "ちぃ"), r("tyu", "ちゅ"), r("tye", "ちぇ"), r("tyo", "ちょ"),
    r("tha", "てゃ"), r("thi", "てぃ"), r("thu", "てゅ"), r("the", "てぇ"), r("tho", "てょ"),
    r("twu", "とぅ"),
    r("da", "だ"), r("di", "ぢ"), r("du", "づ"), r("de", "で"), r("do", "ど"),
    r("dya", "ぢゃ"), r("dyi", "ぢぃ"), r("dyu", "ぢゅ"), r("dye", "ぢぇ"), r("dyo", "ぢょ"),
    r("dha", "でゃ"), r("dhi", "でぃ"), r("dhu", "でゅ"), r("dhe", "でぇ"), r("dho", "でょ"),
    r("dwu", "どぅ"),
    // な行・ん
    r("na", "な"), r("ni", "に"), r("nu", "ぬ"), r("ne", "ね"), r("no", "の"),
    r("nya", "にゃ"), r("nyi", "にぃ"), r("nyu", "にゅ"), r("nye", "にぇ"), r("nyo", "にょ"),
    // 「n」は、後に母音・y・n・' のほかの文字が来たとき(と確定のとき)に「ん」になる(設計文書の 1.3)。
    r("n", "ん"), r("nn", "ん"), r("n'", "ん"), r("xn", "ん"),
    // は行・ば行・ぱ行・ふぁ行
    r("ha", "は"), r("hi", "ひ"), r("hu", "ふ"), r("he", "へ"), r("ho", "ほ"),
    r("hya", "ひゃ"), r("hyi", "ひぃ"), r("hyu", "ひゅ"), r("hye", "ひぇ"), r("hyo", "ひょ"),
    r("fa", "ふぁ"), r("fi", "ふぃ"), r("fu", "ふ"), r("fe", "ふぇ"), r("fo", "ふぉ"),
    r("fya", "ふゃ"), r("fyu", "ふゅ"), r("fyo", "ふょ"),
    r("ba", "ば"), r("bi", "び"), r("bu", "ぶ"), r("be", "べ"), r("bo", "ぼ"),
    r("bya", "びゃ"), r("byi", "びぃ"), r("byu", "びゅ"), r("bye", "びぇ"), r("byo", "びょ"),
    r("pa", "ぱ"), r("pi", "ぴ"), r("pu", "ぷ"), r("pe", "ぺ"), r("po", "ぽ"),
    r("pya", "ぴゃ"), r("pyi", "ぴぃ"), r("pyu", "ぴゅ"), r("pye", "ぴぇ"), r("pyo", "ぴょ"),
    // ま行
    r("ma", "ま"), r("mi", "み"), r("mu", "む"), r("me", "め"), r("mo", "も"),
    r("mya", "みゃ"), r("myi", "みぃ"), r("myu", "みゅ"), r("mye", "みぇ"), r("myo", "みょ"),
    // や行
    r("ya", "や"), r("yu", "ゆ"), r("ye", "いぇ"), r("yo", "よ"),
    // ら行
    r("ra", "ら"), r("ri", "り"), r("ru", "る"), r("re", "れ"), r("ro", "ろ"),
    r("rya", "りゃ"), r("ryi", "りぃ"), r("ryu", "りゅ"), r("rye", "りぇ"), r("ryo", "りょ"),
    // わ行
    r("wa", "わ"), r("wi", "うぃ"), r("we", "うぇ"), r("wo", "を"),
    // ゔ
    r("va", "ゔぁ"), r("vi", "ゔぃ"), r("vu", "ゔ"), r("ve", "ゔぇ"), r("vo", "ゔぉ"),
    // 小書き文字(x の形と l の形。設計文書の 1.5)
    r("xa", "ぁ"), r("xi", "ぃ"), r("xu", "ぅ"), r("xe", "ぇ"), r("xo", "ぉ"),
    r("la", "ぁ"), r("li", "ぃ"), r("lu", "ぅ"), r("le", "ぇ"), r("lo", "ぉ"),
    r("xya", "ゃ"), r("xyu", "ゅ"), r("xyo", "ょ"),
    r("lya", "ゃ"), r("lyu", "ゅ"), r("lyo", "ょ"),
    r("xtu", "っ"), r("xtsu", "っ"), r("ltu", "っ"), r("ltsu", "っ"),
    r("xwa", "ゎ"), r("lwa", "ゎ"),
    r("xka", "ヵ"), r("xke", "ヶ"), r("lka", "ヵ"), r("lke", "ヶ"),
    // 促音(n・l・x のほかの子音を重ねる。設計文書の 1.4)
    sokuon("bb", "b"), sokuon("cc", "c"), sokuon("dd", "d"), sokuon("ff", "f"),
    sokuon("gg", "g"), sokuon("hh", "h"), sokuon("jj", "j"), sokuon("kk", "k"),
    sokuon("mm", "m"), sokuon("pp", "p"), sokuon("qq", "q"), sokuon("rr", "r"),
    sokuon("ss", "s"), sokuon("tt", "t"), sokuon("vv", "v"), sokuon("ww", "w"),
    sokuon("yy", "y"), sokuon("zz", "z"),
    sokuon("tch", "ch"),
    // ひらがなのモードの記号(設計文書の 1.6 の 5 種類)
    r("-", "ー"), r(",", "、"), r(".", "。"), r("[", "「"), r("]", "」"),
];

/// `input` と同じ入力の行。
pub(crate) fn exact(input: &str) -> Option<&'static Rule> {
    RULES.iter().find(|rule| rule.input == input)
}

/// `prefix` で始まり、`prefix` より長い入力の行があるか(次の文字を待つべきか)。
pub(crate) fn has_longer(prefix: &str) -> bool {
    RULES
        .iter()
        .any(|rule| rule.input.len() > prefix.len() && rule.input.starts_with(prefix))
}

/// テーブルの入力に使われる文字か(英小文字、`'`、記号の 5 種類)。
/// 大文字は、ローマ字として扱わない(設計文書の 1.6)。
pub(crate) fn is_romaji_char(c: char) -> bool {
    c.is_ascii_lowercase() || matches!(c, '\'' | '-' | ',' | '.' | '[' | ']')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_unique() {
        for (i, a) in RULES.iter().enumerate() {
            for b in &RULES[i + 1..] {
                assert_ne!(a.input, b.input, "同じ入力の行が 2 つある");
            }
        }
    }

    #[test]
    fn every_rule_is_well_formed() {
        for rule in RULES {
            assert!(
                !rule.input.is_empty() && !rule.output.is_empty(),
                "{rule:?}"
            );
            assert!(rule.input.chars().all(is_romaji_char), "{rule:?}");
            // 残す入力は、入力より短い(照らし直しが終わる)。
            assert!(rule.rest.len() < rule.input.len(), "{rule:?}");
            assert!(rule.input.ends_with(rule.rest), "{rule:?}");
        }
    }

    #[test]
    fn the_symbols_are_in_the_table() {
        // 決定:ひらがなのモードの記号は 5 種類だけ(設計文書の 1.6)。
        let symbols: [(&str, &str); 5] = [
            ("-", "ー"),
            (",", "、"),
            (".", "。"),
            ("[", "「"),
            ("]", "」"),
        ];
        for (input, output) in symbols {
            assert_eq!(exact(input).map(|rule| rule.output), Some(output));
        }
        let others = RULES
            .iter()
            .filter(|rule| !rule.input.starts_with(|c: char| c.is_ascii_lowercase()))
            .count();
        assert_eq!(others, 5, "記号の行が 5 種類のほかにある");
    }
}
