//! kieli-core をホストの端末で試すためのプログラム。
//!
//! 標準入力から 1 行ずつ読み、行の中の並びを順に処理して、1 つごとに結果を 1 行で出す。
//! 書き方は、設計文書(`docs/design.md`)の 10 にある。読めない行があったときは、その行を飛ばして続け、
//! 終わりの値を 1 にする。起動のオプションが誤っていたときは、終わりの値を 2 にする。

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use kieli_cli::Cli;

const USAGE: &str = "使い方: kieli-cli [--max-preedit-bytes N]
標準入力から 1 行ずつ読み、1 つの入力ごとに結果を 1 行で出します(書き方は docs/design.md の 10)。";

fn main() -> ExitCode {
    let mut cli = match parse_args(std::env::args().skip(1)) {
        Ok(Some(cli)) => cli,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("kieli-cli: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
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

/// 起動のオプションを読む。`--help` なら `None`。
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Option<Cli>, String> {
    let mut cli = Cli::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--max-preedit-bytes" => {
                let value = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .ok_or("--max-preedit-bytes には、0 以上の整数を付けてください")?;
                cli = Cli::with_max_preedit_bytes(value);
            }
            _ => return Err(String::from("知らないオプションがあります")),
        }
    }
    Ok(Some(cli))
}
