//! kieli-cli の中身。1 行ずつ読んだ並びを kieli-core のイベントにして処理し、結果を 1 行ずつ返す。
//!
//! 書き方は、設計文書(`docs/design.md`)の 10 にある。kieli-cli は、接続層のまねもする
//! (入力モードを持ち続け、入力先を失ったら取り消し、欄の種類から直接入力の欄かを決める)。

use std::fmt;

use kieli_core::{Config, Event, Feature, InputMode, Outcome, OutcomeKind, Session};

/// 行の中の 1 つの並び。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    /// キーのイベント。
    Key(Event),
    /// 入力先を失う(取り消して、新しい入力先として続ける)。
    Leave,
    /// 欄の種類を変える。
    Field(FieldKind),
}

/// 欄の種類(Wayland の content_type をまねたもの)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// ふつうの欄。
    Normal,
    /// パスワードの欄(直接入力の欄。残さない入力)。
    Password,
    /// 数字のパスワードの欄(直接入力の欄。残さない入力)。
    Pin,
    /// 文字を隠す欄(直接入力の欄)。
    Hidden,
    /// 残さない入力の欄(ふつうに変換する)。
    Sensitive,
}

impl FieldKind {
    fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Password => "password",
            Self::Pin => "pin",
            Self::Hidden => "hidden",
            Self::Sensitive => "sensitive",
        }
    }

    fn is_direct(self) -> bool {
        matches!(self, Self::Password | Self::Pin | Self::Hidden)
    }

    fn is_sensitive(self) -> bool {
        matches!(self, Self::Password | Self::Pin | Self::Sensitive)
    }
}

/// 行を読めなかった理由。入力した文字は持たない(知らせに入れないため)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// 引用符が閉じていない。
    UnclosedQuote,
    /// 引用符の中に空白がある(空白は `<Space>` で書く)。
    SpaceInQuotes,
    /// 引用符の中に、`\"` と `\\` のほかの逆スラッシュの形がある。
    UnknownEscape,
    /// 知らない `<…>` の形。
    UnknownKey,
    /// 引用符でも `<…>` でもない並び。
    Unquoted,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnclosedQuote => "引用符が閉じていない",
            Self::SpaceInQuotes => "引用符の中に空白がある(空白は <Space> で書く)",
            Self::UnknownEscape => "知らない逆スラッシュの形がある(使えるのは \\\" と \\\\)",
            Self::UnknownKey => "知らない <…> の形がある",
            Self::Unquoted => "引用符でくくっていない文字がある",
        })
    }
}

/// 1 行を、並びに分ける。
pub fn parse_line(line: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            parse_quoted(&mut chars, &mut tokens)?;
            // 閉じた引用符の直後は、区切りか行の終わりでなければならない。
            if chars.peek().is_some_and(|c| !c.is_whitespace()) {
                return Err(ParseError::Unquoted);
            }
        } else if c == '<' {
            let mut word = String::new();
            for c in chars.by_ref() {
                if c.is_whitespace() {
                    return Err(ParseError::UnknownKey);
                }
                word.push(c);
                if c == '>' {
                    break;
                }
            }
            tokens.push(special(&word).ok_or(ParseError::UnknownKey)?);
        } else {
            return Err(ParseError::Unquoted);
        }
    }
    Ok(tokens)
}

fn parse_quoted(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    tokens: &mut Vec<Token>,
) -> Result<(), ParseError> {
    loop {
        match chars.next() {
            None => return Err(ParseError::UnclosedQuote),
            Some('"') => return Ok(()),
            Some('\\') => match chars.next() {
                Some(c @ ('"' | '\\')) => tokens.push(Token::Key(Event::Char(c))),
                None => return Err(ParseError::UnclosedQuote),
                Some(_) => return Err(ParseError::UnknownEscape),
            },
            Some(c) if c.is_whitespace() => return Err(ParseError::SpaceInQuotes),
            Some(c) => tokens.push(Token::Key(Event::Char(c))),
        }
    }
}

/// `<…>` の形を、並びにする。
fn special(word: &str) -> Option<Token> {
    let inner = word.strip_prefix('<')?.strip_suffix('>')?;
    let key = |event| Some(Token::Key(event));
    match inner {
        "Space" => key(Event::Space),
        "BS" => key(Event::Backspace),
        "Enter" => key(Event::Commit),
        "Esc" => key(Event::Cancel),
        "Left" => key(Event::Left),
        "Right" => key(Event::Right),
        "Home" => key(Event::Home),
        "End" => key(Event::End),
        "Delete" => key(Event::Delete),
        "Up" => key(Event::Up),
        "Down" => key(Event::Down),
        "Tab" => key(Event::Tab),
        "PageUp" => key(Event::PageUp),
        "PageDown" => key(Event::PageDown),
        "Toggle" => key(Event::ToggleMode),
        "Hiragana" => key(Event::SetMode(InputMode::Hiragana)),
        "Direct" => key(Event::SetMode(InputMode::Direct)),
        "Leave" => Some(Token::Leave),
        "Field:normal" => Some(Token::Field(FieldKind::Normal)),
        "Field:password" => Some(Token::Field(FieldKind::Password)),
        "Field:pin" => Some(Token::Field(FieldKind::Pin)),
        "Field:hidden" => Some(Token::Field(FieldKind::Hidden)),
        "Field:sensitive" => Some(Token::Field(FieldKind::Sensitive)),
        _ => {
            if let Some(rest) = inner.strip_prefix("Ctrl+") {
                return (rest.chars().count() == 1).then_some(Token::Key(Event::Other));
            }
            let number: u8 = inner.strip_prefix('F')?.parse().ok()?;
            ((1..=24).contains(&number) && !inner.starts_with("F0"))
                .then_some(Token::Key(Event::Function(number)))
        }
    }
}

/// kieli-cli の状態。接続層のまねとして、入力モードと欄の種類を持ち続ける。
#[derive(Debug, Clone)]
pub struct Cli {
    session: Session,
    field: FieldKind,
    max_preedit_bytes: usize,
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

impl Cli {
    /// 起動したときの状態(直接入力のモード、ふつうの欄、上限は既定の値)。
    pub fn new() -> Self {
        Self::with_max_preedit_bytes(Config::default().max_preedit_bytes)
    }

    /// 未確定文字の長さの上限(UTF-8 のバイト数)を変えて作る。
    pub fn with_max_preedit_bytes(max_preedit_bytes: usize) -> Self {
        Self {
            session: Session::new(Config {
                max_preedit_bytes,
                ..Config::default()
            }),
            field: FieldKind::Normal,
            max_preedit_bytes,
        }
    }

    /// 1 行を処理し、出す行を返す。行を読めなかったときは、誤りの行だけを返し、何も処理しない。
    pub fn run_line(&mut self, line_number: usize, line: &str) -> Result<Vec<String>, String> {
        if line.trim_start().starts_with('#') {
            return Ok(Vec::new());
        }
        let tokens =
            parse_line(line).map_err(|error| format!("誤り\t{line_number} 行目: {error}"))?;
        Ok(tokens.into_iter().map(|token| self.run(token)).collect())
    }

    fn run(&mut self, token: Token) -> String {
        match token {
            Token::Key(event) => {
                let outcome = self.session.handle(event);
                self.describe(&outcome)
            }
            Token::Leave => {
                // 入力先を失ったら取り消す(設計文書の 3)。入力モードは、接続層が持ち続けて新しいセッションに渡す。
                self.session.cancel();
                self.session = Session::new(Config {
                    mode: self.session.mode(),
                    max_preedit_bytes: self.max_preedit_bytes,
                });
                self.field = FieldKind::Normal;
                format!("入力先を失った\t{}", self.state())
            }
            Token::Field(field) => {
                self.field = field;
                self.session.set_direct_field(field.is_direct());
                self.session.set_sensitive(field.is_sensitive());
                format!("欄を変えた\t{}", self.state())
            }
        }
    }

    fn describe(&self, outcome: &Outcome) -> String {
        format!(
            "{}\tconsumed={}\tcommit={:?}\t{}",
            kind_name(outcome.kind),
            if outcome.consumed { "yes" } else { "no" },
            outcome.commit,
            self.state()
        )
    }

    fn state(&self) -> String {
        let mode = match self.session.mode() {
            InputMode::Hiragana => "ひらがな",
            InputMode::Direct => "直接入力",
        };
        format!(
            "preedit={:?}\tmode={mode}\tfield={}",
            self.session.preedit().text(),
            self.field.name()
        )
    }
}

fn kind_name(kind: OutcomeKind) -> &'static str {
    match kind {
        OutcomeKind::Passed => "渡した",
        OutcomeKind::Updated => "更新",
        OutcomeKind::Committed => "確定",
        OutcomeKind::Canceled => "取消",
        OutcomeKind::ModeChanged => "モード",
        OutcomeKind::Ignored => "何もしなかった",
        OutcomeKind::Unsupported(Feature::Conversion) => "未対応(変換)",
        OutcomeKind::Unsupported(Feature::CursorMovement) => "未対応(カーソル)",
        OutcomeKind::Unsupported(Feature::CandidateOperation) => "未対応(候補)",
        OutcomeKind::OverLimit => "上限を越えた",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<Token> {
        text.chars().map(|c| Token::Key(Event::Char(c))).collect()
    }

    #[test]
    fn quoted_text_becomes_characters() {
        assert_eq!(parse_line(r#""ka""#), Ok(chars("ka")));
        assert_eq!(parse_line(r#""<" "\"" "\\""#), Ok(chars("<\"\\")));
        assert_eq!(parse_line("  \"a\"\t\"b\"  "), Ok(chars("ab")));
        assert_eq!(parse_line(""), Ok(Vec::new()));
    }

    #[test]
    fn special_keys() {
        assert_eq!(
            parse_line("<Space> <BS> <Enter> <Esc> <Toggle> <Hiragana> <Direct>"),
            Ok(vec![
                Token::Key(Event::Space),
                Token::Key(Event::Backspace),
                Token::Key(Event::Commit),
                Token::Key(Event::Cancel),
                Token::Key(Event::ToggleMode),
                Token::Key(Event::SetMode(InputMode::Hiragana)),
                Token::Key(Event::SetMode(InputMode::Direct)),
            ])
        );
        assert_eq!(
            parse_line(
                "<Left> <Right> <Home> <End> <Delete> <Up> <Down> <Tab> <PageUp> <PageDown>"
            ),
            Ok([
                Event::Left,
                Event::Right,
                Event::Home,
                Event::End,
                Event::Delete,
                Event::Up,
                Event::Down,
                Event::Tab,
                Event::PageUp,
                Event::PageDown,
            ]
            .map(Token::Key)
            .to_vec())
        );
        assert_eq!(
            parse_line("<F1> <F6> <F24> <Ctrl+s>"),
            Ok(vec![
                Token::Key(Event::Function(1)),
                Token::Key(Event::Function(6)),
                Token::Key(Event::Function(24)),
                Token::Key(Event::Other),
            ])
        );
        assert_eq!(
            parse_line("<Leave> <Field:password> <Field:normal>"),
            Ok(vec![
                Token::Leave,
                Token::Field(FieldKind::Password),
                Token::Field(FieldKind::Normal),
            ])
        );
    }

    #[test]
    fn errors() {
        assert_eq!(parse_line(r#""ka"#), Err(ParseError::UnclosedQuote));
        assert_eq!(parse_line(r#""k a""#), Err(ParseError::SpaceInQuotes));
        assert_eq!(parse_line(r#""\n""#), Err(ParseError::UnknownEscape));
        assert_eq!(parse_line(r#""a"b"#), Err(ParseError::Unquoted));
        assert_eq!(parse_line("ka"), Err(ParseError::Unquoted));
        for word in [
            "<Foo>",
            "<F0>",
            "<F25>",
            "<F06>",
            "<Ctrl+>",
            "<Ctrl+ab>",
            "<Space",
            "<Sp ace>",
        ] {
            assert_eq!(parse_line(word), Err(ParseError::UnknownKey), "{word}");
        }
    }

    #[test]
    fn an_error_line_does_nothing_and_does_not_show_the_input() {
        let mut cli = Cli::new();
        let error = cli.run_line(3, r#"<Hiragana> "secret"#).unwrap_err();
        assert_eq!(error, "誤り\t3 行目: 引用符が閉じていない");
        assert!(!error.contains("secret"));
        // 誤りの行は、前の並び(<Hiragana>)も処理しない。
        assert_eq!(cli.session.mode(), InputMode::Direct);
    }

    #[test]
    fn a_session_from_typing_to_commit() {
        let mut cli = Cli::new();
        let lines = cli.run_line(1, r#"<Hiragana> "nihongo" <Enter>"#).unwrap();
        assert_eq!(lines.len(), 9);
        assert_eq!(
            lines[0],
            "モード\tconsumed=yes\tcommit=\"\"\tpreedit=\"\"\tmode=ひらがな\tfield=normal"
        );
        assert_eq!(
            lines[7],
            "更新\tconsumed=yes\tcommit=\"\"\tpreedit=\"にほんご\"\tmode=ひらがな\tfield=normal"
        );
        assert_eq!(
            lines[8],
            "確定\tconsumed=yes\tcommit=\"にほんご\"\tpreedit=\"\"\tmode=ひらがな\tfield=normal"
        );
    }

    #[test]
    fn every_kind_of_result_is_shown() {
        let mut cli = Cli::new();
        let lines = cli
            .run_line(
                1,
                r#""a" <Hiragana> <Hiragana> "kak" <BS> <Space> <Left> <Up> <Esc> <Space>"#,
            )
            .unwrap();
        let kinds: Vec<&str> = lines
            .iter()
            .map(|line| line.split('\t').next().unwrap())
            .collect();
        assert_eq!(
            kinds,
            [
                "渡した",
                "モード",
                "何もしなかった",
                "更新",
                "更新",
                "更新",
                "更新",
                "未対応(変換)",
                "未対応(カーソル)",
                "未対応(候補)",
                "取消",
                "確定"
            ]
        );

        // 上限(1,024 バイト)は、ひらがなで 341 文字(1,023 バイト)。342 文字目は受け付けない。
        let mut cli = Cli::new();
        let line = format!("<Hiragana> \"{}\"", "a".repeat(342));
        let lines = cli.run_line(1, &line).unwrap();
        assert!(lines[341].starts_with("更新\t"));
        assert!(lines[342].starts_with("上限を越えた\tconsumed=yes\tcommit=\"\""));
    }

    #[test]
    fn leave_and_fields() {
        let mut cli = Cli::new();
        let lines = cli
            .run_line(
                1,
                r#"<Hiragana> "ka" <Leave> "a" <Field:password> "a" <Field:normal> "a""#,
            )
            .unwrap();
        // 入力先を失ったら取り消し、モードは持ち続ける。
        assert_eq!(
            lines[3],
            "入力先を失った\tpreedit=\"\"\tmode=ひらがな\tfield=normal"
        );
        // パスワードの欄に移ると、前の未確定文字を確定せずに捨て、キーを渡す。
        assert_eq!(
            lines[5],
            "欄を変えた\tpreedit=\"\"\tmode=ひらがな\tfield=password"
        );
        assert!(lines[6].starts_with("渡した\t"));
        // ふつうの欄に戻ると、元のモードで変換する。
        assert!(lines[8].starts_with("更新\t") && lines[8].contains("preedit=\"あ\""));
    }

    #[test]
    fn the_shortcut_commits_and_passes() {
        let mut cli = Cli::new();
        let lines = cli
            .run_line(1, r#"<Hiragana> "kan" <Ctrl+s> <F7>"#)
            .unwrap();
        assert!(lines[4].starts_with("確定\tconsumed=no\tcommit=\"かん\""));
        assert!(lines[5].starts_with("渡した\t"));
    }

    #[test]
    fn comment_lines_are_skipped() {
        let mut cli = Cli::new();
        assert_eq!(cli.run_line(1, "# 注釈 \"ka"), Ok(Vec::new()));
    }
}
