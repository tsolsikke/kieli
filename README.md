# Kieli

Kieli は、Rust で作る日本語 IME です。既存の IME(Anthy など)の移植ではなく、一から実装します。

## 今の段階

第 1 段階の途中です。kieli-core で、ローマ字からひらがなへの変換と、編集・確定・取消が動きます(かな漢字変換はまだ無い)。kieli-cli で、1 行ずつ読む形で試せます。設計は `docs/design.md` にあります。

| 段階 | 内容 | 判定 |
| --- | --- | --- |
| 第 1 段階 | kieli-core と kieli-cli で、ローマ字からひらがなへの変換と、編集・確定・取消 | Kieli の初版の完成 |
| 第 2 段階 | kieli-wayland を足し、Linux の上の Seinas とつなぐ | Seinas との統合の完了(初版とは別に判定する) |
| 第 3 段階以降 | 辞書、かな漢字変換、候補ウィンドウ。Windows の TSF の接続層は、さらに後 | — |

最初に動作を保証する範囲は、Linux と ZeikOS の上の [Seinas](https://github.com/tsolsikke/seinas)(Wayland のコンポジタ)です。

## 構成

| crate | 役割 |
| --- | --- |
| `crates/kieli-core` | 変換の規則と、入力セッションの状態。`no_std` + `alloc`。Wayland・fd・serial・COM・描画・OS 固有のキーコードを持ち込まない |
| `crates/kieli-cli` | kieli-core の最初の利用者。ホストの端末で、1 行ずつ読む形で試す |
| `kieli-wayland`(第 2 段階で足す) | 別のプロセスの IME として、標準の input-method-v2 と virtual-keyboard-v1 でコンポジタにつなぐ。Kieli 専用のプロトコルは作らない |

ほかの OS には、kieli-core の上に OS ごとの接続層を作ります(将来の Windows 11 の TSF など)。

## ビルドとテスト

Rust の版は `rust-toolchain.toml` で固定しています(Seinas と同じ 1.97.1)。

```bash
cargo build
```

kieli-cli は、標準入力から 1 行ずつ読み、1 つの入力ごとに結果を 1 行で出します(書き方は `docs/design.md` の 10)。見本の入力と、試す手順は `docs/try-kieli-cli.md` にあります。第 1 段階の完了条件と、それぞれを何で確かめたかは `docs/stage1-acceptance.md` にあります。

```bash
echo '<Hiragana> "nihongo" <Enter>' | cargo run -q -p kieli-cli
```

検査は、次の 1 つのコマンドでまとめて回せます。CI も同じものを使います。

```bash
tools/check.sh
```

`tools/check.sh` が行うこと:

- `cargo fmt --check`
- `cargo clippy`(警告を誤りとして扱う)
- `cargo test`
- `cargo doc`(`RUSTDOCFLAGS="-D warnings"`)
- kieli-core だけを、std の無い target(`x86_64-unknown-none`)でビルドする。target が入っていなければ、入れずに落ちる
- kieli-core の依存が、`tools/core-deps.allow` の一覧の中だけであること。Wayland・OS・描画の crate が入ったら落ちる

`tools/check-sabotage.sh` は、上の検査を 1 つずつわざと壊し、それぞれが落ちることを確かめます(作業ツリーには触らず、コピーの中で行う)。

## ライセンス

MIT ライセンスです(`LICENSE`)。
