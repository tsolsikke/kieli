//! Kieli の変換の規則と、入力セッションの状態を持つ crate。
//!
//! std を使わず、alloc だけを使う。Wayland・fd・serial・COM・描画・OS 固有のキーコードは持ち込まない
//! (接続層の側で、正規化したイベントに直してから渡す)。
//!
//! 今は骨組みだけで、変換の規則はまだ無い。型と関数の形は、`docs/design.md` の案で決める。

#![no_std]

extern crate alloc;

use alloc::string::String;

/// 1 つの入力先に対応する、入力セッションの状態。
///
/// 状態はインスタンスごとに持ち、大域の状態にはしない。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Session {
    preedit: String,
}

impl Session {
    /// 空のセッションを作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 今の未確定文字(UTF-8)。
    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    /// 未確定文字が無いか。
    pub fn is_idle(&self) -> bool {
        self.preedit.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_session_has_no_preedit() {
        let session = Session::new();
        assert_eq!(session.preedit(), "");
        assert!(session.is_idle());
    }

    #[test]
    fn sessions_do_not_share_state() {
        let mut a = Session::new();
        let b = Session::new();
        a.preedit.push('あ');
        assert_eq!(a.preedit(), "あ");
        assert!(b.is_idle());
    }
}
