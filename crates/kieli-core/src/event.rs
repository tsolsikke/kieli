//! 接続層から受けるイベントと、入力モード。

/// 入力モード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// ローマ字をひらがなにする。
    Hiragana,
    /// 変換しない。キーをアプリへ渡す。
    Direct,
}

impl InputMode {
    /// 切り替えた後のモード。
    pub fn toggled(self) -> Self {
        match self {
            Self::Hiragana => Self::Direct,
            Self::Direct => Self::Hiragana,
        }
    }
}

/// 接続層が、物理のキーから正規化したイベント。
///
/// keysym・evdev の番号・端末のバイト列・Windows の仮想キーコードからの変換は、接続層が行う。
/// 修飾キーを押しただけのときは、イベントにしない(接続層が修飾キーの状態を持つ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// 接続層が keymap から得た、Shift を反映した後の文字。
    /// Ctrl・Alt・Super を押したままのキーは、[`Event::Other`] にする。
    /// 制御文字は、[`Event::Other`] と同じに扱う。
    Char(char),
    /// Backspace。
    Backspace,
    /// 確定(Enter)。
    Commit,
    /// 取消(Esc)。
    Cancel,
    /// Space。未確定文字があるときは、かな漢字変換のキーとして取っておく。
    Space,
    /// 左。未確定文字の中を動くキーとして取っておく。
    Left,
    /// 右。未確定文字の中を動くキーとして取っておく。
    Right,
    /// Home。未確定文字の中を動くキーとして取っておく。
    Home,
    /// End。未確定文字の中を動くキーとして取っておく。
    End,
    /// Delete。未確定文字の中の編集のキーとして取っておく。
    Delete,
    /// 上。変換・候補の操作のキーとして取っておく。
    Up,
    /// 下。変換・候補の操作のキーとして取っておく。
    Down,
    /// Tab。変換・候補の操作のキーとして取っておく。
    Tab,
    /// PageUp。候補の操作のキーとして取っておく。
    PageUp,
    /// PageDown。候補の操作のキーとして取っておく。
    PageDown,
    /// ファンクションキー。番号は F1 を 1 とする。
    ///
    /// F6〜F10 は、変換のキー(一般的な日本語 IME では、ひらがな・全角カタカナ・半角カタカナ・全角英数・
    /// 半角英数)として取っておく。ほかの番号は、[`Event::Other`] と同じに扱う。
    Function(u8),
    /// 入力モードの切り替え(半角/全角など)。
    ToggleMode,
    /// 入力モードを決めた値にする。
    SetMode(InputMode),
    /// Kieli が使わないキー(Ctrl の組み合わせのショートカット、ファンクションキーなど)。
    Other,
}
