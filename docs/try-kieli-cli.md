# kieli-cli を試す手順

運用者が、WSL の端末で kieli-cli を自分で試すための手順です。コマンドは、そのままコピーして貼れば動きます。
書き方と出力の形は、`docs/design.md` の 10 にあります。

## 1. ビルドする

```bash
cd ~/workspace/hack-labs/kieli
cargo build -p kieli-cli
```

`target/debug/kieli-cli` ができます。Rust の版は `rust-toolchain.toml` で決まります(1.97.1)。

## 2. 見本を流す

見本の入力は `examples/basic.txt` です。ひらがな・ん・促音・小書き・記号・大文字と数字・未定義の綴り・Backspace・Esc(取消)・Enter(確定)・取っておいたキー・Space・ショートカット・モードの切り替え・直接入力の欄・入力先を失う・上限を越える、を一通り含みます。
各行の前の `#` の行に、何を確かめる行かを書いてあります(`#` の行は、kieli-cli が読み飛ばす)。

上限を越える動きを短い入力で見せるため、見本は上限を 15 バイト(ひらがなで 5 文字)にして流します(ふだんの上限は 1,024 バイト)。

```bash
target/debug/kieli-cli --max-preedit-bytes 15 < examples/basic.txt
```

出力の列はタブで区切ってあります。そろえて見るときは、次のようにします。

```bash
target/debug/kieli-cli --max-preedit-bytes 15 < examples/basic.txt | column -t -s "$(printf '\t')"
```

## 3. 期待する出力と比べる

期待する出力は `examples/basic.expected` です。`# 入力:` の行に、その下の出力を出した入力の行を書いてあります。`#` の行を除けば、kieli-cli の出力とそのまま比べられます。

```bash
diff <(target/debug/kieli-cli --max-preedit-bytes 15 < examples/basic.txt) <(grep -v '^#' examples/basic.expected) && echo 同じです
```

「同じです」と出れば、期待どおりです。違う所があれば、diff がその行を出します。
同じ比べ方を、`cargo test`(`crates/kieli-cli/tests/examples.rs` の `the_basic_example_gives_the_expected_output`)でも行っています。

## 4. 自分で 1 行ずつ打って試す

```bash
target/debug/kieli-cli
```

起動したら、1 行打って Enter を押すたびに、その行の結果が出ます。終えるときは Ctrl+D を押します。

起動したときは直接入力のモードなので、最初に `<Hiragana>` を打ちます。たとえば、次の行を 1 行ずつ打ちます。

```text
<Hiragana>
"nihongo" <Enter>
"kitte" <BS> <BS> <Esc>
"ka" <Space> <F7> <Enter>
```

書き方の要点(くわしくは `docs/design.md` の 10.1):

- 文字は引用符でくくります(`"ka"`)。引用符の中に空白は書けません(空白は `<Space>`)。`"` は `\"`、`\` は `\\` と書きます。`<` は、引用符の中ならただの文字です(`"<"`)。
- キーは `<Space>` `<BS>` `<Enter>` `<Esc>` `<Left>` `<Right>` `<Home>` `<End>` `<Delete>` `<Up>` `<Down>` `<Tab>` `<PageUp>` `<PageDown>` `<F1>`〜`<F24>` `<Ctrl+s>` のように書きます。
- モードは `<Toggle>`(切り替え)、`<Hiragana>`、`<Direct>`。
- 入力先を失うのは `<Leave>`、欄の種類は `<Field:normal>` `<Field:password>` `<Field:pin>` `<Field:hidden>` `<Field:sensitive>`。
- 読めない行は、`誤り` の行だけが出て、その行は処理しません(入力した文字は出しません)。

## 5. CLI で確かめられないこと

CLI で確かめられるのは、変換の規則と、状態の移り方までです。操作感、キーの繰り返し、Wayland での順番(確定とキーの渡し方の順など)は、第 2 段階(Seinas との統合)で確かめます。
