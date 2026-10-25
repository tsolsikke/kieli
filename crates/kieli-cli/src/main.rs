//! kieli-core をホストの端末で試すためのプログラム。
//!
//! 今は骨組みだけで、セッションを作って、その状態を出す。

use kieli_core::Session;

fn main() {
    let session = Session::new();
    println!("kieli-cli: preedit={:?}", session.preedit());
}
