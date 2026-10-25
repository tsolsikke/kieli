#!/bin/sh
# Kieli の検査を、1 つのコマンドでまとめて行う。CI(.github/workflows/ci.yml)も、これを呼ぶ。
#
# 使い方:
#     tools/check.sh
#
# 行う検査:
#     fmt         cargo fmt --check
#     clippy      cargo clippy(-D warnings)
#     test        cargo test
#     rustdoc     cargo doc(RUSTDOCFLAGS="-D warnings")
#     no_std      kieli-core だけを、std の無い target(x86_64-unknown-none)でビルドする
#     core-deps   kieli-core の依存が、tools/core-deps.allow の一覧の中だけであること
#
# 終わりの値:0 は全部通った、1 は落ちた検査がある。
# 落ちても残りの検査は続け、最後に落ちたものをまとめて出す。
set -eu
export LC_ALL=C.UTF-8

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

# 試しに別の target を指すときだけ、KIELI_NO_STD_TARGET で変える(tools/check-sabotage.sh が使う)。
no_std_target=${KIELI_NO_STD_TARGET:-x86_64-unknown-none}

failed=""
log=$(mktemp)
trap 'rm -f "$log"' EXIT
# 1 つの検査を行う。出力は、落ちたときだけ見せる(終わりの 40 行)。
step() {
    name=$1
    shift
    if "$@" > "$log" 2>&1; then
        printf -- '-- %s:通った\n' "$name"
    else
        printf -- '-- %s:落ちた\n' "$name"
        tail -n 40 "$log" | sed 's/^/   /'
        failed="$failed
  $name"
    fi
}

# std の無い target は、入っているものを使う。入っていなければ、入れずに落とす(入れるかは運用者が決める)。
check_no_std() {
    if ! rustup target list --installed | grep -qx "$no_std_target"; then
        echo "$no_std_target が入っていません(rustup target list --installed)。このスクリプトは入れません。"
        return 1
    fi
    cargo build --locked -p kieli-core --target "$no_std_target"
}

# kieli-core の依存(通常とビルドの依存。どの target のものも含める)を、一覧と比べる。
# テストだけの依存(dev-dependencies)は、ビルドの成果物に入らないので見ない。
check_core_deps() {
    deps=$(cargo tree --locked -p kieli-core -e normal,build --target all --prefix none \
        | sed 1d | awk '{print $1}' | sort -u)
    allowed=$(grep -v '^#' tools/core-deps.allow | grep . | sort -u || true)
    status=0
    for dep in $deps; do
        # Wayland・OS・描画の crate は、一覧に書いてあっても落とす。
        if echo "$dep" | grep -qiE 'wayland|smithay|xkb|x11|windows|winapi|libc|nix|rustix|mio|calloop|tiny-skia|pixman'; then
            echo "kieli-core の依存に、Wayland・OS・描画の crate があります: $dep"
            status=1
        elif ! echo "$allowed" | grep -qx "$dep"; then
            echo "kieli-core の依存に、tools/core-deps.allow に無い crate があります: $dep"
            status=1
        fi
    done
    [ $status -eq 0 ] && echo "kieli-core の依存: ${deps:-なし}"
    return $status
}

step "fmt" cargo fmt --all -- --check
step "clippy" cargo clippy --workspace --all-targets --locked -- -D warnings
step "test" cargo test --workspace --locked
step "rustdoc" env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
step "no_std(kieli-core を $no_std_target で)" check_no_std
step "kieli-core の依存" check_core_deps

echo
if [ -n "$failed" ]; then
    echo "落ちた検査:$failed"
    exit 1
fi
echo "全部通りました。"
