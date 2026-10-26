//! 見本のファイル(`examples/`)を kieli-cli に流し、期待する出力と同じことを確かめる。
//!
//! 期待する出力のファイルの `#` で始まる行は、読む人のための注釈なので、比べる前に除く。
//! 手順は `docs/try-kieli-cli.md` にある。

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// kieli-cli に `input` を流して、標準出力と、終わりの値が成功かを返す。
fn run(args: &[&str], input: &str) -> (String, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kieli-cli"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("kieli-cli を起動できない");
    child
        .stdin
        .take()
        .expect("標準入力")
        .write_all(input.as_bytes())
        .expect("標準入力へ書けない");
    let output = child.wait_with_output().expect("kieli-cli が終わらない");
    (
        String::from_utf8(output.stdout).expect("出力が UTF-8 でない"),
        output.status.success(),
    )
}

#[test]
fn the_basic_example_gives_the_expected_output() {
    let input = std::fs::read_to_string(examples().join("basic.txt")).unwrap();
    let expected: String = std::fs::read_to_string(examples().join("basic.expected"))
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| format!("{line}\n"))
        .collect();
    let (actual, success) = run(&["--max-preedit-bytes", "15"], &input);
    assert!(success, "終わりの値が 0 でない");
    // 違ったときに、どの行かが分かるように、行ごとに比べる。
    for (number, (actual, expected)) in actual.lines().zip(expected.lines()).enumerate() {
        assert_eq!(actual, expected, "出力の {} 行目", number + 1);
    }
    assert_eq!(
        actual.lines().count(),
        expected.lines().count(),
        "出力の行の数"
    );
}

#[test]
fn an_error_line_makes_the_exit_status_fail_and_the_rest_runs() {
    let (output, success) = run(&[], "<Hiragana> \"a\n\"a\"\n");
    assert!(!success);
    assert_eq!(
        output,
        "誤り\t1 行目: 引用符が閉じていない\n渡した\tconsumed=no\tcommit=\"\"\tpreedit=\"\"\tmode=直接入力\tfield=normal\n"
    );
}

#[test]
fn a_bad_option_is_refused() {
    let output = Command::new(env!("CARGO_BIN_EXE_kieli-cli"))
        .arg("--bogus")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
