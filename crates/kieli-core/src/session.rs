//! 入力セッション(1 つの入力先に対応する状態)。

use alloc::string::String;
use core::mem;
use core::ops::Range;

use crate::width::to_full_width;
use crate::{Event, InputMode};

/// 起動したときのモード。その後は、接続層が利用者の選んだモードを持ち続ける(設計文書の 2.3)。
const DEFAULT_MODE: InputMode = InputMode::Direct;

/// 未確定文字の長さの上限の既定値(UTF-8 のバイト数)。暫定の値で、仕様の値ではない(設計文書の 5)。
const DEFAULT_MAX_PREEDIT_BYTES: usize = 1024;

/// 未確定文字が無いときに、ひらがなのモードの Space で確定する文字(設計文書の 2.3)。
const IDEOGRAPHIC_SPACE: char = '\u{3000}';

/// セッションを作るときの設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// 始めの入力モード。入力モードは seat ごとに 1 つで、接続層が持っている値を渡す。
    pub mode: InputMode,
    /// 未確定文字の長さの上限(UTF-8 のバイト数)。
    pub max_preedit_bytes: usize,
}

impl Default for Config {
    /// 直接入力のモードで、上限は 1,024 バイト。
    fn default() -> Self {
        Self {
            mode: DEFAULT_MODE,
            max_preedit_bytes: DEFAULT_MAX_PREEDIT_BYTES,
        }
    }
}

/// 取っておいたキーが、後の段階で受け持つ機能。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// かな漢字変換(Space)。
    Conversion,
    /// 未確定文字の中を動く(左右・Home・End)。
    CursorMovement,
    /// 変換・候補の操作(上下・Tab)。
    CandidateOperation,
}

/// 1 つのイベントを処理した結果の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeKind {
    /// 何もしなかった。キーはアプリへ渡す。
    Passed,
    /// 未確定文字が変わった(文字を足した、消した)。
    Updated,
    /// 確定した。`consumed` が false なら、確定した文字を送ってから、キーをアプリへ渡す。
    Committed,
    /// 未確定文字を捨てた。
    Canceled,
    /// 入力モードを変えた。未確定文字があったときは、確定してから変えている(`commit` に入る)。
    ModeChanged,
    /// 後の段階で使うキーで、まだ機能が無い。未確定文字は変えない。
    Unsupported(Feature),
    /// 未確定文字の長さの上限を越えるので、受け付けなかった。未確定文字は変えない。
    OverLimit,
}

/// 1 つのイベントを処理した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// 結果の種類。
    pub kind: OutcomeKind,
    /// キーを消費したか。false なら、接続層はキーをアプリへ渡す。
    ///
    /// false でも、[`Outcome::commit`] が空でないことがある(確定してからキーを渡す形)。
    /// そのときは、確定した文字を先に送ってから、キーを渡す。
    pub consumed: bool,
    /// 確定した文字。無ければ空。
    pub commit: String,
    /// 処理した後の未確定文字の全体。無ければ空。カーソルは終わりにある。
    pub preedit: String,
}

/// 未確定文字と、その中のカーソル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preedit<'a> {
    text: &'a str,
    cursor: Range<usize>,
}

impl<'a> Preedit<'a> {
    fn at_end(text: &'a str) -> Self {
        Self {
            text,
            cursor: text.len()..text.len(),
        }
    }

    /// 未確定文字(UTF-8)。
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// カーソルの位置(UTF-8 のバイト)。始まりと終わりは、どちらも `text` の文字境界にある。
    pub fn cursor(&self) -> Range<usize> {
        self.cursor.clone()
    }
}

/// 1 つの入力先に対応する、入力セッションの状態。
///
/// 状態はインスタンスごとに持ち、大域の状態にはしない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    max_preedit_bytes: usize,
    /// 利用者が選んだモード。直接入力の欄でも上書きしない。
    mode: InputMode,
    preedit: String,
    sensitive: bool,
    direct_field: bool,
}

/// イベントに対して行うこと。[`Session::would_consume`] と [`Session::handle`] は、どちらもこれを元にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// 消費しない。何もしない。
    Pass,
    /// 未確定文字に足す。
    Append(char),
    /// 未確定文字の最後の 1 文字を消す。
    DeleteLast,
    /// 未確定文字を確定する。
    CommitPreedit,
    /// 未確定文字を確定してから、キーをアプリへ渡す(消費しない)。
    CommitThenPass,
    /// 未確定文字を捨てる。
    CancelPreedit,
    /// 決まった文字を確定する(未確定文字が無いとき)。
    CommitText(char),
    /// 未確定文字を確定してから、入力モードを変える。
    SwitchMode(InputMode),
    /// 取っておいたキー。何もしない。
    Unsupported(Feature),
    /// 上限を越えるので、受け付けない。
    Refuse,
}

impl Action {
    fn consumed(self) -> bool {
        !matches!(self, Self::Pass | Self::CommitThenPass)
    }

    fn kind(self) -> OutcomeKind {
        match self {
            Self::Pass => OutcomeKind::Passed,
            Self::Append(_) | Self::DeleteLast => OutcomeKind::Updated,
            Self::CommitPreedit | Self::CommitThenPass | Self::CommitText(_) => {
                OutcomeKind::Committed
            }
            Self::CancelPreedit => OutcomeKind::Canceled,
            Self::SwitchMode(_) => OutcomeKind::ModeChanged,
            Self::Unsupported(feature) => OutcomeKind::Unsupported(feature),
            Self::Refuse => OutcomeKind::OverLimit,
        }
    }
}

impl Session {
    /// 空のセッションを作る。
    pub fn new(config: Config) -> Self {
        Self {
            max_preedit_bytes: config.max_preedit_bytes,
            mode: config.mode,
            preedit: String::new(),
            sensitive: false,
            direct_field: false,
        }
    }

    /// イベントを消費するかを調べる。状態は変えない。
    ///
    /// Windows の TSF の `OnTestKeyDown` のような、調べるだけの問い合わせに使う。
    /// 同じ状態で、`would_consume(&e)` と `handle(e).consumed` は、いつも同じ値になる。
    pub fn would_consume(&self, event: &Event) -> bool {
        self.plan(event).consumed()
    }

    /// イベントを処理して、状態を進める。
    pub fn handle(&mut self, event: Event) -> Outcome {
        let action = self.plan(&event);
        let commit = self.apply(action);
        Outcome {
            kind: action.kind(),
            consumed: action.consumed(),
            commit,
            preedit: self.preedit.clone(),
        }
    }

    /// 未確定文字を捨てる。入力先を失ったときなどに、接続層が呼ぶ。
    pub fn cancel(&mut self) {
        self.preedit.clear();
    }

    /// 未確定文字を確定し、確定した文字を返す(無ければ空)。Enter と同じ確定の処理を使う。
    pub fn commit(&mut self) -> String {
        mem::take(&mut self.preedit)
    }

    /// 今の未確定文字。
    pub fn preedit(&self) -> Preedit<'_> {
        Preedit::at_end(&self.preedit)
    }

    /// 利用者が選んだ入力モード。直接入力の欄の間も、この値は変わらない。
    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// 実際の動きの入力モード。直接入力の欄の間は、いつも [`InputMode::Direct`]。
    pub fn effective_mode(&self) -> InputMode {
        if self.direct_field {
            InputMode::Direct
        } else {
            self.mode
        }
    }

    /// 未確定文字があるか。
    pub fn is_composing(&self) -> bool {
        !self.preedit.is_empty()
    }

    /// 残さない入力か(`sensitive_data` の欄など)を設定する。
    ///
    /// 今は覚えるだけ。履歴や学習を入れるときに、この値が立っている間の入力を残さないようにする。
    pub fn set_sensitive(&mut self, sensitive: bool) {
        self.sensitive = sensitive;
    }

    /// 残さない入力か。
    pub fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    /// 直接入力の欄(パスワードの欄など)かを設定する。欄の種類から決めるのは、接続層の役目。
    ///
    /// 直接入力の欄にするときは、それまでの未確定文字を捨てる(守る欄へ確定しない)。
    /// 利用者が選んだモード([`Session::mode`])は変えないので、ふつうの欄に戻せば元のモードで動く。
    pub fn set_direct_field(&mut self, direct_field: bool) {
        if direct_field {
            self.cancel();
        }
        self.direct_field = direct_field;
    }

    /// 直接入力の欄か。
    pub fn is_direct_field(&self) -> bool {
        self.direct_field
    }

    /// 今の状態とイベントから、行うことを決める。状態は変えない。
    fn plan(&self, event: &Event) -> Action {
        match *event {
            Event::ToggleMode => return Action::SwitchMode(self.mode.toggled()),
            Event::SetMode(mode) => return Action::SwitchMode(mode),
            _ => {}
        }
        if self.is_composing() {
            match *event {
                Event::Char(c) if !c.is_control() => self.plan_append(c),
                Event::Backspace => Action::DeleteLast,
                Event::Commit => Action::CommitPreedit,
                Event::Cancel => Action::CancelPreedit,
                Event::Space => Action::Unsupported(Feature::Conversion),
                Event::Left | Event::Right | Event::Home | Event::End => {
                    Action::Unsupported(Feature::CursorMovement)
                }
                Event::Up | Event::Down | Event::Tab => {
                    Action::Unsupported(Feature::CandidateOperation)
                }
                // Kieli が使わないキーは、確定してからアプリへ渡す。アプリでの順番と編集の結果は、
                // Seinas との統合で確かめる(設計文書の 2.3)。
                Event::Char(_) | Event::Other => Action::CommitThenPass,
                Event::ToggleMode | Event::SetMode(_) => unreachable!("上で扱った"),
            }
        } else if self.effective_mode() == InputMode::Hiragana {
            match *event {
                Event::Char(c) if !c.is_control() => self.plan_append(c),
                Event::Space => Action::CommitText(IDEOGRAPHIC_SPACE),
                _ => Action::Pass,
            }
        } else {
            Action::Pass
        }
    }

    /// 未確定文字に文字を足す。足した後の全体が上限を越えるなら、受け付けない。
    ///
    /// 未確定文字に残る英字・数字・記号は全角にする(大文字も、打ったまま全角で残す。設計文書の 1.6)。
    fn plan_append(&self, c: char) -> Action {
        let c = to_full_width(c);
        if self.preedit.len() + c.len_utf8() <= self.max_preedit_bytes {
            Action::Append(c)
        } else {
            Action::Refuse
        }
    }

    /// 決めたことを行い、確定した文字を返す。
    fn apply(&mut self, action: Action) -> String {
        match action {
            Action::Pass | Action::Unsupported(_) | Action::Refuse => String::new(),
            Action::Append(c) => {
                self.preedit.push(c);
                String::new()
            }
            // 保留のローマ字はまだ無いので、表示の最後の 1 文字(Unicode のスカラー値 1 つ)を消す。
            Action::DeleteLast => {
                self.preedit.pop();
                String::new()
            }
            Action::CommitPreedit | Action::CommitThenPass => self.commit(),
            Action::CancelPreedit => {
                self.cancel();
                String::new()
            }
            Action::CommitText(c) => String::from(c),
            Action::SwitchMode(mode) => {
                self.mode = mode;
                self.commit()
            }
        }
    }
}
