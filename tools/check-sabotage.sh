#!/bin/sh
# tools/check.sh の各検査が、壊したときに本当に落ちることを確かめる。
#
# 使い方:
#     tools/check-sabotage.sh
#
# 作業ツリーには触らない。追跡しているファイルを作業用のディレクトリへコピーし、そこで 1 つずつ壊して
# tools/check.sh を回す。壊し方ごとに、決めた検査が「落ちた」になり、ほかの検査が通ることを見る。
# ネットワークは使わない(依存の壊し方は、作業用のディレクトリの中の path の crate を使う)。
#
# 終わりの値:0 は全部の壊し方で期待どおりに落ちた、1 は期待と違うものがある。
set -eu
export LC_ALL=C.UTF-8

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# ビルドの成果物は、壊し方の間で使い回す。
export CARGO_TARGET_DIR="$work/target"

fresh() {
    rm -rf "$work/tree"
    mkdir "$work/tree"
    (cd "$root" && git ls-files -z --cached --others --exclude-standard | xargs -0 cp --parents -t "$work/tree")
}

lib=crates/kieli-core/src/lib.rs
status=0
# 壊し方を 1 つ試す。$1 は名前、$2 は落ちるはずの検査の名前(check.sh の出力の「-- 」の後の頭の部分)。
# $2 が空なら、全部の検査が通ることを期待する(壊さないときの確かめ)。
# 壊す手順は、関数 sabotage_<名前> に書く。
try() {
    name=$1
    expect=$2
    fresh
    (cd "$work/tree" && "sabotage_$name")
    out=$(cd "$work/tree" && tools/check.sh 2>&1) && code=0 || code=$?
    failed=$(echo "$out" | grep -E '^-- .*:落ちた$' | sed 's/^-- //; s/:落ちた$//')
    if [ -z "$expect" ]; then
        if [ $code -eq 0 ] && [ -z "$failed" ]; then
            echo "ok  $name:全部通った"
            return
        fi
    elif [ $code -ne 0 ] && [ "$(echo "$failed" | grep -c .)" -eq 1 ] && echo "$failed" | grep -q "^$expect"; then
        echo "ok  $name:「$expect」だけが落ちた"
    else
        echo "NG  $name:落ちた検査が期待と違う(期待は「${expect:-なし}」、終わりの値 $code)"
        echo "$out" | sed 's/^/    /'
        status=1
    fi
}

sabotage_none() { :; }
# テストのモジュールの前に、ファイルの中身を差し込む(後ろに足すと、clippy の items_after_test_module にも当たるため)。
insert_before_tests() {
    awk -v snippet="$1" '/^#\[cfg\(test\)\]$/ { printf "%s\n", snippet } { print }' "$lib" > "$lib.new"
    mv "$lib.new" "$lib"
}
sabotage_fmt() { insert_before_tests 'pub fn badly_formatted( ) {}
'; }
sabotage_clippy() { insert_before_tests '/// clippy の ptr_arg に当たる。
pub fn ptr_arg(s: &String) -> usize {
    s.len()
}
'; }
sabotage_test() { printf '\n#[test]\nfn fails() {\n    panic!("sabotage");\n}\n' >> crates/kieli-cli/src/main.rs; }
sabotage_rustdoc() { insert_before_tests '/// [`NoSuchItem`] への壊れたリンク。
pub fn broken_link() {}
'; }
sabotage_no_std() { sed -i 's|^#!\[no_std\]$|// no_std を外した。|' "$lib"; }
fake_dep() {
    mkdir -p "crates/$1/src"
    printf '#![no_std]\n' > "crates/$1/src/lib.rs"
    printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n' "$1" > "crates/$1/Cargo.toml"
    sed -i "s|^members = \[|members = [\n    \"crates/$1\",|" Cargo.toml
    printf '%s = { path = "../%s" }\n' "$1" "$1" >> crates/kieli-core/Cargo.toml
    cargo metadata --offline --format-version 1 > /dev/null
}
sabotage_dep_unlisted() { fake_dep some-helper; }
sabotage_dep_wayland() { fake_dep wayland-client; echo wayland-client >> tools/core-deps.allow; }
sabotage_target_missing() { :; }

try none ""
try fmt "fmt"
try clippy "clippy"
try test "test"
try rustdoc "rustdoc"
try no_std "no_std"
try dep_unlisted "kieli-core の依存"
try dep_wayland "kieli-core の依存"
export KIELI_NO_STD_TARGET=no-such-target-none
try target_missing "no_std"
unset KIELI_NO_STD_TARGET
exit $status
