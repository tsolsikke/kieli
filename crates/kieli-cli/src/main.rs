//! kieli-core をホストの端末で試すためのプログラム。
//!
//! 標準入力から 1 行ずつ読み、行の中の並びを順に処理して、1 つごとに結果を 1 行で出す。
//! 書き方は、設計文書(`docs/design.md`)の 10 にある。読めない行があったときは、その行を飛ばして続け、
//! 終わりの値を 1 にする。

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use kieli_cli::Cli;

fn main() -> ExitCode {
    let mut cli = Cli::new();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut failed = false;
    for (index, line) in io::stdin().lock().lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("kieli-cli: 標準入力を読めませんでした: {error}");
                return ExitCode::FAILURE;
            }
        };
        let written = match cli.run_line(index + 1, &line) {
            Ok(lines) => lines.iter().try_for_each(|line| writeln!(out, "{line}")),
            Err(error) => {
                failed = true;
                writeln!(out, "{error}")
            }
        };
        if written.is_err() {
            return ExitCode::FAILURE;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
