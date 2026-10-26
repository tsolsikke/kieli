//! 未確定文字の中身(変換済み部分と、保留のローマ字)。

use alloc::string::String;

use crate::romaji::{self, Rule};
use crate::width::to_full_width;

/// 未確定文字。
///
/// 変換済み部分(かなと、未定義の綴りとして残した英字)と、まだかなにならないローマ字(保留)に分けて持つ。
/// 表示は「変換済み部分 + 保留を全角にしたもの」で、未確定文字の長さの上限も表示の長さで見る。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Composition {
    converted: String,
    /// 保留のローマ字(打った ASCII のまま)。
    pending: String,
    /// 表示(`converted` と、`pending` を全角にしたもの)。
    display: String,
}

impl Composition {
    /// 表示。
    pub(crate) fn text(&self) -> &str {
        &self.display
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.display.is_empty()
    }

    /// ひらがなのモードで、文字を 1 つ足す。
    ///
    /// テーブルの入力に使われる文字はローマ字として照らす。ほかの文字(大文字・数字・記号・かななど)は、
    /// 保留を片付けてから、そのまま(ASCII なら全角にして)変換済み部分に足す。
    pub(crate) fn push(&mut self, c: char) {
        if romaji::is_romaji_char(c) {
            let mut input = core::mem::take(&mut self.pending);
            input.push(c);
            self.feed(input);
        } else {
            self.flush_pending();
            self.converted.push(to_full_width(c));
        }
        self.refresh();
    }

    /// 最後の 1 文字を消す。保留のローマ字があれば、その最後の 1 文字。無ければ、表示の最後の 1 文字
    /// (Unicode のスカラー値 1 つ)。
    pub(crate) fn backspace(&mut self) {
        if self.pending.pop().is_none() {
            self.converted.pop();
        }
        self.refresh();
    }

    /// 確定する文字を取り出して、空にする。保留は、テーブルに同じ入力があれば(末尾の `n` など)その出力に、
    /// 無ければ英字(全角)にする。
    pub(crate) fn take(&mut self) -> String {
        self.flush_pending();
        self.display.clear();
        core::mem::take(&mut self.converted)
    }

    /// 空にする。
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// 保留とつないだ入力を、テーブルで照らす。
    ///
    /// 1. 入力より長い行で、入力から始まるものがあれば、次の文字を待つ(入力を保留にする)。
    /// 2. 入力と同じ行があれば、その出力を出し、行の残す入力から照らし直す。
    /// 3. 入力の頭の部分と同じ行があれば、いちばん長いものの出力を出し、残りを照らし直す
    ///    (`nk` の「ん」と `k` など。決まった規則を、未定義の綴りより先に照らす)。
    /// 4. どれでもなければ、未定義の綴り:先頭の 1 文字を英字(全角)のまま変換済み部分に残し、残りを照らし直す。
    fn feed(&mut self, mut input: String) {
        while !input.is_empty() {
            if romaji::has_longer(&input) {
                self.pending = input;
                return;
            }
            if let Some(rule) = romaji::exact(&input) {
                self.converted.push_str(rule.output);
                input = String::from(rule.rest);
                continue;
            }
            if let Some((rule, len)) = longest_prefix(&input) {
                self.converted.push_str(rule.output);
                let mut next = String::from(rule.rest);
                next.push_str(&input[len..]);
                input = next;
                continue;
            }
            let first = input.remove(0);
            self.converted.push(to_full_width(first));
        }
    }

    /// 保留を片付ける。テーブルに同じ入力があればその出力、無ければ英字(全角)にする。
    fn flush_pending(&mut self) {
        let pending = core::mem::take(&mut self.pending);
        if pending.is_empty() {
            return;
        }
        match romaji::exact(&pending) {
            Some(rule) if rule.rest.is_empty() => self.converted.push_str(rule.output),
            _ => self.converted.extend(pending.chars().map(to_full_width)),
        }
    }

    fn refresh(&mut self) {
        self.display.clear();
        self.display.push_str(&self.converted);
        self.display.extend(self.pending.chars().map(to_full_width));
    }
}

/// `input` の頭の部分(`input` より短いもの)と同じ入力の行のうち、いちばん長いもの。
fn longest_prefix(input: &str) -> Option<(&'static Rule, usize)> {
    (1..input.len())
        .rev()
        .find_map(|len| romaji::exact(&input[..len]).map(|rule| (rule, len)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文字を順に足した後の表示。
    fn typed(text: &str) -> String {
        let mut composition = Composition::default();
        for c in text.chars() {
            composition.push(c);
        }
        String::from(composition.text())
    }

    /// 文字を順に足して、確定した文字。
    fn committed(text: &str) -> String {
        let mut composition = Composition::default();
        for c in text.chars() {
            composition.push(c);
        }
        composition.take()
    }

    #[test]
    fn every_rule_converts_alone() {
        // テーブルの行ごとに、その入力だけを打って確定したら、出力(と残す入力の英字)になる。
        for rule in romaji::RULES {
            let mut expected = String::from(rule.output);
            expected.extend(rule.rest.chars().map(to_full_width));
            assert_eq!(committed(rule.input), expected, "{rule:?}");
        }
    }

    #[test]
    fn basic_words() {
        assert_eq!(typed("kana"), "かな");
        assert_eq!(typed("nihongo"), "にほんご");
        assert_eq!(typed("kyouha"), "きょうは");
        assert_eq!(typed("shinbun"), "しんぶｎ");
        assert_eq!(committed("shinbun"), "しんぶん");
    }

    #[test]
    fn n_rules() {
        // 設計文書の 1.3:nn と n' は ん。n の後に母音・y・n・' のほかが来たら ん。確定のときの n も ん。
        assert_eq!(typed("nn"), "ん");
        assert_eq!(typed("n'"), "ん");
        assert_eq!(typed("xn"), "ん");
        assert_eq!(typed("kanji"), "かんじ");
        assert_eq!(typed("nna"), "んあ");
        assert_eq!(typed("nnna"), "んな");
        assert_eq!(typed("n'a"), "んあ");
        assert_eq!(typed("nya"), "にゃ");
        assert_eq!(typed("n"), "ｎ");
        assert_eq!(committed("n"), "ん");
        assert_eq!(committed("kan"), "かん");
    }

    #[test]
    fn sokuon() {
        // 設計文書の 1.4:n・l・x のほかの子音を重ねたら っ。tch も っ。
        assert_eq!(typed("kitte"), "きって");
        assert_eq!(typed("matcha"), "まっちゃ");
        assert_eq!(typed("mma"), "っま");
        assert_eq!(typed("kk"), "っｋ");
        assert_eq!(committed("kk"), "っｋ");
        for c in "bcdfghjkmpqrstvwyz".chars() {
            let mut doubled = String::new();
            doubled.push(c);
            doubled.push(c);
            assert!(typed(&doubled).starts_with('っ'), "{c}");
        }
        // ll・xx は促音にならない(未定義の綴り)。
        assert_eq!(typed("lla"), "ｌぁ");
        assert_eq!(typed("xxa"), "ｘぁ");
    }

    #[test]
    fn small_kana_with_x_and_l() {
        // 設計文書の 1.5:x の形と l の形の両方。xka・xke はカタカナの ヵ・ヶ。
        assert_eq!(typed("xa"), "ぁ");
        assert_eq!(typed("la"), "ぁ");
        assert_eq!(typed("xtu"), "っ");
        assert_eq!(typed("ltsu"), "っ");
        assert_eq!(typed("lyo"), "ょ");
        assert_eq!(typed("xwa"), "ゎ");
        assert_eq!(typed("xka"), "ヵ");
        assert_eq!(typed("lke"), "ヶ");
    }

    #[test]
    fn undefined_spellings_keep_the_first_letter() {
        // 決定:先頭の 1 文字を英字(全角)のまま残し、残りをテーブルで照らし直す(設計文書の 1.6)。
        assert_eq!(typed("qza"), "ｑざ");
        assert_eq!(typed("kqa"), "ｋくぁ");
        assert_eq!(typed("nyk"), "んｙｋ");
        assert_eq!(committed("ky"), "ｋｙ");
    }

    #[test]
    fn the_symbols_win_over_the_full_width_conversion() {
        // 決定:記号 5 種類は、全角にする処理より先にテーブルで変える(設計文書の 1.6)。
        assert_eq!(typed("-,.[]"), "ー、。「」");
        assert_eq!(typed("ra-men"), "らーめｎ");
        // ほかの記号は全角のまま残る。
        assert_eq!(typed("!?/"), "！？／");
    }

    #[test]
    fn capitals_digits_and_kana_are_kept_as_typed() {
        // 決定:大文字はローマ字として扱わず、打ったまま残す。幅は全角(設計文書の 1.6)。
        assert_eq!(typed("Ka"), "Ｋあ");
        assert_eq!(typed("KA"), "ＫＡ");
        assert_eq!(typed("a1b"), "あ１ｂ");
        assert_eq!(typed("あ"), "あ");
        // 保留は、ローマ字でない文字の前で片付ける(末尾の n は ん)。
        assert_eq!(typed("kanA"), "かんＡ");
        assert_eq!(typed("k1"), "ｋ１");
    }

    #[test]
    fn backspace_deletes_a_pending_letter_first() {
        // 決定:保留のローマ字があれば、その最後の 1 文字。無ければ表示の最後の 1 文字(設計文書の 2.2)。
        let mut composition = Composition::default();
        for c in "kya".chars() {
            composition.push(c);
        }
        assert_eq!(composition.text(), "きゃ");
        composition.backspace();
        assert_eq!(composition.text(), "き");

        let mut composition = Composition::default();
        for c in "aky".chars() {
            composition.push(c);
        }
        assert_eq!(composition.text(), "あｋｙ");
        composition.backspace();
        assert_eq!(composition.text(), "あｋ");
        // 消した後も、保留として続けて照らせる。
        composition.push('a');
        assert_eq!(composition.text(), "あか");
    }
}
