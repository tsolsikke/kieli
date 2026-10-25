//! Kieli の変換の規則と、入力セッションの状態を持つ crate。
//!
//! std を使わず、alloc だけを使う。Wayland・fd・serial・COM・描画・OS 固有のキーコードは持ち込まない
//! (接続層の側で、正規化したイベントに直してから渡す)。
//!
//! 今は、ローマ字のテーブルを使わない動き(文字をそのまま未確定文字に入れ、Backspace・確定・取消・
//! 入力モードの切り替え・長さの上限・取っておいたキー・直接入力の欄を扱う)だけがある。
//! 振る舞いの決まりは、設計文書(`docs/design.md`)の決定による。

#![no_std]

extern crate alloc;

mod event;
mod session;
mod width;

pub use event::{Event, InputMode};
pub use session::{Config, Feature, Outcome, OutcomeKind, Preedit, Session};

#[cfg(test)]
mod tests;
