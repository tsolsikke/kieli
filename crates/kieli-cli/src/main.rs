//! kieli-core をホストの端末で試すためのプログラム。
//!
//! 今は骨組みだけで、セッションを作って、その状態を出す。

use kieli_core::{Config, Session};

fn main() {
    let session = Session::new(Config::default());
    println!(
        "kieli-cli: mode={:?} preedit={:?}",
        session.mode(),
        session.preedit().text()
    );
}
