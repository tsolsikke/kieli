# Kieli の第 2 段階の設計(案)

第 2 段階は、Linux の上の Seinas で、Kieli を通して、text-input-v3 に対応したアプリにひらがなを入力する段です。この文書は、その設計の案です。コードはまだ書いていません。

- 「決定」と書いたものは、運用者が決めたものです(第 1 段階の設計文書 `docs/design.md` と、Seinas の ADR 0004 にあるもの)。「案」と「推し」は、書き手の案で、運用者が決めたものではありません。運用者の判断が要る論点は、9 にまとめてあります(GPT-6 にそのまま貼れる形)。
- 推測と事実を分けて書きます。出典を付けたものは、その出典で確かめた事実です。「推測」と書いたものは、確かめていません。

## 0. 前提

### 0.1 決まっていること

- Kieli は、別のプロセスの IME として、標準の input-method-v2 と virtual-keyboard-v1 でコンポジタにつなぐ。Kieli 専用のプロトコルは作らない(運用者の決定「案 D」、2026-10-24。`docs/design.md` の 0.1)。
- Wayland の接続層は、このリポジトリの別の crate の kieli-wayland にする。kieli-core は no_std + alloc のままで、Wayland の型・fd・serial を持ち込まない(同じ)。
- 第 2 段階の完了は、「Seinas との統合の完了」として、初版の完成とは別に判定する(同じ)。
- 最初に保証する範囲は、Linux と ZeikOS の上の Seinas。第 2 段階は Linux の上の Seinas で、ZeikOS の上で動かすのは先の段(同じ。この文書の 8)。

### 0.2 第 1 段階から第 2 段階に残したもの

`docs/stage1-acceptance.md` の 3 と `docs/design.md` で、第 2 段階に残したもの:

| 残したもの | 出典 | この文書での扱い |
| --- | --- | --- |
| Kieli が使わないキーを、確定してからアプリへ渡すときの、アプリでの順番と編集の結果 | `docs/design.md` の 2.3(決定の条件) | 3.5、6 の C6 |
| 入力先を失ったとき(Wayland の deactivate)に取り消す | `docs/design.md` の 3 | 3.4、6 の C5 |
| content_type から、直接入力の欄・残さない入力を決める | `docs/design.md` の 6 | 3.4 |
| 入力モードを seat ごとに 1 つ持ち、セッションに渡す | `docs/design.md` の 7 | 3.3 |
| 操作感、キーの繰り返し、Wayland での順番 | `docs/design.md` の 10.3 | 3.7、6 |
| 切り替えのキー(半角/全角など)と、英字配列での組み合わせ | `docs/design.md` の 2.3、Seinas の ADR 0004 の 3(Q7) | 3.2、9 の論点 7 |

### 0.3 出典

| 記号 | 出典 |
| --- | --- |
| [ADR4] | Seinas の `docs/adr/0004-ime-keyboard-path.md`(origin/main の 56fd606。2026-11-04 に読んだ) |
| [ime] | Seinas の `docs/ime.md`(同じ版) |
| [zreq] | Seinas の `docs/zeikos-requirements.md`(同じ版) |
| [patches] | Seinas の `docs/smithay-patches.md`(同じ版) |
| [test-ime] | Seinas の `crates/seinas-test-clients/src/ime.rs`(同じ版。テスト用の IME) |
| [IM] | input-method-unstable-v2.xml(wayland-protocols-misc 0.3.12。Seinas の Cargo.lock の版。行の番号もこの XML のもの) |
| [TI] | text-input-unstable-v3.xml(wayland-protocols 0.32.13。同じ) |
| [VK] | virtual-keyboard-unstable-v1.xml(wayland-protocols-misc 0.3.12) |

Seinas の回帰のテストは、Seinas の `crates/seinas-standalone/tests/ime.rs` にあり、ADR 0004 の 4 の表「統合の前の確認項目」(6 項目)と対応しています(Seinas のコミット d590cb7 と f419ef5)。

## 1. 全体の形

```text
キーボード ─→ Seinas ─(grab: keymap・key・modifiers・repeat_info)─→ kieli-wayland
                                                                       │ keysym → Event
                                                                       ▼
                                                                    kieli-core(Session)
                                                                       │ Outcome
                                                                       ▼
            アプリ ←─(text-input-v3: preedit・commit)── Seinas ←─(input-method-v2: set_preedit_string・commit_string・commit)─┤
            アプリ ←─(wl_keyboard: key・modifiers)──── Seinas ←─(virtual-keyboard: keymap・key・modifiers)─────────────────┘
```

- kieli-wayland は、1 つのスレッドで動く Wayland のクライアントです。IME のためのソケット(`WAYLAND_DISPLAY=<名前>-ime`。入れ子の Seinas では `seinas-0-ime`)へつなぎます [ADR4] 1.6、[ime]「見せる global」。
- Seinas が Kieli へ渡すのは、grab で届くキーだけです。Seinas のショートカット(Alt+Tab、緊急解除の Alt+Shift+F12 など)は、Kieli に届きません [ADR4] 1.8。
- kieli-core は、第 1 段階のまま使います(3.3)。kieli-core に Wayland のための口は足さない案です。

## 2. kieli-wayland の作り

### 2.1 つなぐ順

1. `wl_display` に接続し、`wl_registry` で `wl_seat`・`zwp_input_method_manager_v2`・`zwp_virtual_keyboard_manager_v1` を探す。input-method と virtual-keyboard の global が無ければ、理由(ふつうのソケットにつないだ、`--no-ime-socket` の構成、キーボードの無い seat)をログに出して終わる。Seinas は、IME のソケットにだけこの 2 つを見せ、キーボードの無い seat では作らない [ime]「見せる global」。
2. `get_input_method(seat)` で `zwp_input_method_v2` を作る。`unavailable` が届いたら(ほかの IME がつないでいる)、そのオブジェクトを捨てて、ログに出して終わる [IM] 340〜355 行、[ADR4] 1.7・2.4・3 の「unavailable と、activate・deactivate」。
3. `grab_keyboard()` で `zwp_input_method_keyboard_grab_v2` を作る。届く keymap で、XKB の状態を作る(2.3)。
4. `create_virtual_keyboard(seat)` で `zwp_virtual_keyboard_v1` を 1 つ作り、grab で届いた keymap と同じ中身の keymap を送る(2.4)。virtual-keyboard は作り直さず、1 つを使い続ける(Seinas は、要求を送らずに待つものを 1 クライアントで 8 個まで、生きているものを 16 個までに限る [ime]「見せる global」の限界)。

virtual-keyboard を作れるのは、`get_input_method` を受け入れられたクライアントだけです。順を逆にすると、`unauthorized` のエラーで切られます [ime]「見せる global」。

### 2.2 キーの受け方

grab の `key` イベント(serial、時刻、evdev のキーの番号、押した・離した)を受けるたびに:

1. XKB の状態を進め、押したキーなら keysym と文字を得る(2.3)。
2. 押したキーを、kieli-core の `Event` に直す(2.3 の表)。直せないキー(修飾キーだけ、など)は kieli-core へ渡さない。
3. 入力文脈がアクティブなら(3.4)、`Session::handle` を呼ぶ。アクティブでなければ、3.6 の決まりで扱う。
4. `Outcome` に従って、input-method の要求と virtual-keyboard のキーを送る(3.5)。
5. 押したキーを virtual-keyboard でアプリへ渡したかを、キーの番号ごとに覚える。離したキーは、押したときに渡したものだけを渡す。押したときに Kieli が消費したキーの「離した」は、どこへも渡さない。

`modifiers` イベントは、XKB の状態に入れ、そのまま virtual-keyboard の `modifiers` で送り返す(アプリの修飾キーの状態を、物理のキーボードと合わせるため)。Seinas は、IME が grab で受けた物理の modifiers を送り返すのを、ふつうの形としている [ADR4] 1.9「modifiers の限り」。

### 2.3 keymap の扱い(XKB)

- grab の `keymap` イベントで、形式(xkb_v1)・fd・大きさが届く。fd は読むだけで mmap(PROT_READ、MAP_PRIVATE)し、XKB の keymap を作る。Seinas の既定の keymap は、layout `jp`、model `jp106`(設定で `us` などに変えられる)[ADR4] 1.12。
- keymap が替わったら(grab に新しい `keymap` が届いたら)、XKB の状態を作り直し、virtual-keyboard にも新しい keymap を送る。ただし、Smithay 0.7.0 は、keymap が替わっても grab へは送らない(上流の 8a270232 で直った。Seinas は、Kieli の第 2 段階の前に取り込む候補としている)[patches]「取り込む候補」。取り込むまでは、入れ子の構成で親の keymap に置き換わると、Kieli は古い keymap のままキーを読む。
- keysym から `Event` への対応(案。切り替えのキーは 9 の論点 7):

| keysym(XKB の名前) | Event |
| --- | --- |
| 文字になるキー(Ctrl・Alt・Super を押していない) | `Char(c)`(XKB で得た UTF-8 の 1 文字。Shift を反映したもの) |
| `BackSpace` | `Backspace` |
| `Return`、`KP_Enter` | `Commit` |
| `Escape` | `Cancel` |
| `space`(修飾なし) | `Space` |
| `Left`・`Right`・`Home`・`End`・`Delete` | 同じ名前の Event |
| `Up`・`Down`・`Tab`・`Prior`(PageUp)・`Next`(PageDown) | `Up`・`Down`・`Tab`・`PageUp`・`PageDown` |
| `F1`〜`F24` | `Function(1)`〜`Function(24)` |
| `Zenkaku_Hankaku`(jp106 の半角/全角) | `ToggleMode` |
| `Hiragana_Katakana`(jp106 のカタカナ/ひらがな) | `SetMode(Hiragana)`(案) |
| `Eisu_toggle`(jp106 の英数)・`Muhenkan`(無変換) | `SetMode(Direct)`(案) |
| Ctrl・Alt・Super を押したままの文字、ほかの keysym | `Other` |
| 修飾キーだけ(`Shift_L` など) | kieli-core へ渡さない(virtual-keyboard でそのまま戻す) |

- `Char` は、物理のキーの位置ではなく、keymap から得た文字で作る(`docs/design.md` の 1.6 の決定)。
- XKB の扱いには、Seinas と同じ `xkbcommon` crate(0.8.0。libxkbcommon を使う)を使う案です(9 の論点 2)。

### 2.4 virtual-keyboard の keymap

- virtual-keyboard の `key` を送る前に、keymap が要る [VK] 54〜68 行。grab で届いた keymap の中身を、Kieli が自分で作った memfd に書き、その fd を送る。
- grab で届いた fd を、そのまま送り返さない。Smithay 0.7.0 は、grab へは一時ファイルで keymap を渡す [zreq] の「一時ファイル」の行。テスト用の IME は届いた fd をそのまま送り返すが [test-ime] 238〜256 行、ZeikOS では、コンポジタが一時ファイルの fd を mmap すると EBADF になる [zreq] の「クライアントから受け取った keymap の fd の mmap」の行。
- memfd へは、`write()` ではなく、ftruncate で大きさを決めてから mmap(PROT_WRITE、MAP_SHARED)して書く形を既定にする(Linux でも ZeikOS でも同じ形。8.1)。

## 3. kieli-core との境目

### 3.1 kieli-core は変えない

第 1 段階の `Session`・`Event`・`Outcome`・`OutcomeKind` を、そのまま使います。kieli-wayland が持つものと、kieli-core が持つものを、次のように分けます。

| 持つもの | 持ち主 |
| --- | --- |
| ローマ字の規則、未確定文字、保留、結果の種類 | kieli-core(`Session`) |
| seat ごとの入力モード(全体で 1 つ) | kieli-wayland(`docs/design.md` の 7 の決定) |
| XKB の状態、keysym から `Event` への対応 | kieli-wayland |
| 受けた done の数(commit の serial) | kieli-wayland |
| virtual-keyboard で渡したキーの記録 | kieli-wayland |
| キーの繰り返しのタイマー | kieli-wayland(`docs/design.md` の 2.1) |
| content_type から欄の種類を決めること | kieli-wayland(`docs/design.md` の 6) |

`Session::would_consume` は、Wayland では使いません(調べるだけの問い合わせが無いため。TSF の接続層のためのもの)。

### 3.2 キーと Event

2.3 の表のとおりです。第 1 段階の `Event` で足りない形は、今のところありません(推測。2.3 の表で、全部の keysym を `Event` か「渡さない」に分けられた)。

### 3.3 セッションと入力モード

- 入力文脈がアクティブになるたびに(3.4)、新しい `Session` を作る。`Config` の `mode` には、kieli-wayland が持っている seat の入力モードを渡す。起動したときのモードは直接入力(`docs/design.md` の 2.3 の決定)。
- `OutcomeKind::ModeChanged` が返ったら、`Session::mode` を seat の入力モードとして覚え直す。次の `Session` に引き継がれる。

### 3.4 activate・deactivate と content_type

input-method-v2 の状態は、`done` で効きます [IM] 91〜93 行。kieli-wayland は、`done` を受けたときに、たまった状態をまとめて効かせます。

- `activate` の後の `done`:新しい `Session` を作る(3.3)。同じ `done` までに届いた `content_type` で、欄の種類を決める。
  - purpose が `password`・`pin`、または hint に `hidden_text` がある:`Session::set_direct_field(true)`。
  - hint に `sensitive_data` がある、または purpose が `password`・`pin`:`Session::set_sensitive(true)`。
  - `docs/design.md` の 6 の決定と同じ分け方です。text-input-v3 の値は [TI] 209〜210 行、240〜241 行。
- 同じアクティブの間に `content_type` が変わったら(`done` で)、同じ分け方で設定し直す。直接入力の欄に移るときは、`set_direct_field(true)` が未確定文字を捨てる(守る欄へ確定しない)。
- `deactivate` の後の `done`:`Session::cancel` を呼んでセッションを捨て、何も送らない(`docs/design.md` の 3 の決定)。アプリは、text-input の leave で未確定文字を消す [TI] 349〜351 行。deactivate の後に送った要求は、新しい入力先には効かない [IM] 66〜68 行、[ADR4] 1.16。
- deactivate の後は、前の入力欄のために残っているキーを virtual-keyboard で送らない [ADR4] 1.16 の末尾「IME の側でできること」。
- `done` の数を数え、commit の serial にする(3.5)。

### 3.5 未確定文字の届け方と、確定の届け方

`Outcome` ごとに、次のように送ります(`serial` は、それまでに受けた `done` の数 [IM] 279〜306 行、[ADR4] 1.16)。

| OutcomeKind | consumed | 送るもの |
| --- | --- | --- |
| `Updated` | true | `set_preedit_string(preedit, len, len)`、`commit(serial)` |
| `Committed` | true | `commit_string(commit)`、`set_preedit_string(preedit, len, len)`、`commit(serial)` |
| `Committed` | false | 上と同じものを送ってから、virtual-keyboard でキーを渡す(確定してから渡す) |
| `Canceled` | true | `set_preedit_string("", 0, 0)`、`commit(serial)` |
| `ModeChanged` | true | 確定した文字があれば `commit_string` と空の `set_preedit_string` と `commit(serial)`。無ければ何も送らない |
| `Ignored`・`Unsupported`・`OverLimit` | true | 何も送らない(状態が変わっていない) |
| `Passed` | false | virtual-keyboard でキーを渡すだけ |

- `len` は未確定文字の UTF-8 のバイト数で、カーソルは未確定文字の終わり(`docs/design.md` の 4)。上限は 1,024 バイトなので、4000 バイトの上限 [IM] 216・237 行と、`i32` に収まる。
- text-input-v3 のアプリは、`done` のたびに未確定文字を初期値(空)に戻す [TI] 380 行。そのため、未確定文字が変わらなくても、`commit` を送るときは今の未確定文字を毎回入れる。
- キーごとの返事として、空の `commit` を送らない。文字を出さずにキーを使うのは、ふつうの IME の振る舞いで、Seinas も返事を求めない [ADR4] 1.3・2.2、3 の「キーへの返事」。
- 確定してから渡す形(`consumed` が false で `commit` が空でない):`commit` を送った後、同じ接続で virtual-keyboard の `key` を送る。Kieli から Seinas への要求は、1 つの接続の中で順に処理されるので、Seinas はアプリへ、確定(text-input の commit_string と done)を先に、キーを後に送る、と考えている(推測。Seinas の fork の Smithay は、IME の commit でアプリへ送り [ADR4] 1.16、virtual-keyboard の key は届いた時にフォーカスのクライアントへ直に送る [ADR4] 1.16 の「防げない場面」)。アプリの側で、確定の後にキーが効くかは、第 2 段階で確かめる(6 の C6。9 の論点 8)。

### 3.6 入力文脈がアクティブでないとき

text-input を使わないアプリ(zeyes など)や、入力欄の外では、入力文脈がアクティブになりません。このときは変換せず、キーを virtual-keyboard でそのまま戻します [ADR4] 1.10。

切り替えのキー(`ToggleMode`・`SetMode`)をどうするかは、9 の論点 4 です(案:消費して seat の入力モードだけを変える)。

### 3.7 キーの繰り返し

- 繰り返しは、IME が消費したキーは IME が、virtual-keyboard で戻したキーはアプリが行う。grab には `repeat_info`(1 秒あたりの回数と、始めるまでの時間)が届く [ADR4] 3 の「キーの繰り返し」、[ime]「押されたままのキー」の繰り返しの項。
- 案:Kieli が消費したキー(押下が `consumed` だったもの)を押し続けたら、`repeat_info` に従って同じ `Event` を `handle` に渡し直す。繰り返しの途中で、結果が消費しないもの(`consumed` が false)になったら、その時点で繰り返しを止め、キーは渡さない(押し始めに消費したキーの「押した」をアプリに渡していないので、途中から渡すと「離した」と組にならないため)。たとえば、Backspace を押し続けて未確定文字が空になったら、そこで止まる。
- 繰り返しの扱いは、9 の論点 3 です。

## 4. 切れたとき

### 4.1 緊急解除

- Seinas の緊急解除(既定は Alt+Shift+F12)は、Kieli の接続を閉じるだけで、`wl_display.error` は送らない。Kieli の側からは、ただの切断に見える(読むと EOF、送ると EPIPE)[ADR4] 1.5、3 の「緊急解除のときの見え方」。
- Kieli は、接続が切れたら、未確定文字を捨て(送る先が無い)、ログに切れたことだけを出して終わる案です。アプリには、Seinas が text-input の leave を送り、アプリが未確定文字を消す [ime]「IME の復旧」。virtual-keyboard で押したままのキーも、Seinas が離す [ADR4] 1.9。
- 緊急解除は、利用者が IME を止めたいときの操作なので、Kieli が自分でつなぎ直すと、解除の意味が無くなります。つなぎ直すのは、IME か起動の仕組みの役目とされています [ADR4] 3 の「緊急解除のときの見え方」。Kieli は、緊急解除と、Seinas が落ちたときを見分けられません(どちらも EOF)。扱いは 9 の論点 5 です(案:終わりの値で知らせて終わり、つなぎ直さない)。
- 緊急解除のキーは、IME のためのソケットを開く構成では、IME がつながっていなくても予約される。`--no-ime-socket` の構成では予約されず、アプリへ渡る(この構成では Kieli がつなげない)[ADR4] 1.8。Kieli は、このキーを切り替えのキーなどに使わない(どの構成でも Kieli には届かないか、Kieli がいない)。

### 4.2 Seinas が落ちたとき・再起動したとき

- Seinas が落ちると、接続が切れる(EOF か EPIPE)。4.1 と同じく、未確定文字を捨ててログに出し、終わる案です。
- Seinas が再起動したら、新しい Kieli を起こすのは起動の仕組みの役目とする案です(Kieli は、起動の仕組みが起こす。コンポジタは起こさない [ADR4] 3 の「起動と、IME として認められること」)。
- Seinas は、古い接続のクリーンアップが済むまで、新しい `get_input_method` を断る [ime]「IME の復旧」。つなぎ直した直後に `unavailable` が届くことがあるので、起動の仕組みがすぐに起こし直すときは、少し待って起こし直す必要がある(推測)。
- 新しくつながった IME には、フォーカスのあるアプリの同じ欄で、text-input の enter が届く(クリックし直さなくてよい)[ime]「IME の復旧」。

### 4.3 Kieli が自分で input method を destroy しない

Seinas は、IME が `zwp_input_method_v2` だけを destroy したときも、IME が去ったものとして片付ける [ADR4] 1.5。Kieli は、動き続けるつもりなら input method を destroy しない。終わるときは、接続を閉じる。

## 5. Seinas の側の決まりのうち、Kieli が守ること

| # | 決まり | 出典 |
| --- | --- | --- |
| S1 | IME のためのソケット(`<名前>-ime`)へつなぐ。ほかのソケットには input-method と virtual-keyboard が無い | [ADR4] 1.6、[ime]「見せる global」 |
| S2 | IME は seat ごとに同時に 1 つ。`unavailable` を受けたら、そのオブジェクトを捨てる | [ADR4] 1.7・2.4、3 の「unavailable と、activate・deactivate」 |
| S3 | virtual-keyboard は、`get_input_method` を受け入れられた後に作る。作り直さずに 1 つを使う(待つもの 8 個、生きているもの 16 個の上限がある) | [ADR4] 1.6、[ime]「見せる global」 |
| S4 | キーごとの返事を送らない。空の `commit` を返事の代わりに送らない | [ADR4] 1.3・2.2、3 の「キーへの返事」 |
| S5 | 使わなかったキーは virtual-keyboard でアプリへ戻す。押したキーは、離したことも戻す | [ADR4] 3 の「使わなかったキー」、1.9 |
| S6 | commit には、受けた done の数をそのまま serial として付ける | [ADR4] 1.16、3 の「古い入力文脈への応答」 |
| S7 | deactivate の後は、前の入力欄のためのキーを virtual-keyboard で送らない | [ADR4] 1.16 の「IME の側でできること」 |
| S8 | IME へ渡したキーは、Seinas がアプリへ送り直さない。Kieli が受けたキーは、Kieli が戻さない限りアプリに届かない | [ADR4] 1.3 |
| S9 | Seinas のショートカット(Alt+Tab・緊急解除など)は Kieli に届かない。切り替えのキーに使わない | [ADR4] 1.8 |
| S10 | 緊急解除は、ただの切断に見える。つなぎ直すのは IME か起動の仕組みの役目 | [ADR4] 1.5、3 の「緊急解除のときの見え方」 |
| S11 | virtual-keyboard の keymap は、key の前に送る | [VK] 54〜68 行、[ADR4] 2.5 |
| S12 | 未確定文字は、アプリが text-input-v3 の preedit で描く。text-input を使わないアプリでは変換しない | [ADR4] 1.10 |
| S13 | 消費したキーの繰り返しは IME が行う(`repeat_info`) | [ADR4] 3 の「キーの繰り返し」 |
| S14 | `sensitive_data` の入力を、ログや学習に残さない | [ADR4] 2.6、3 の「パスワードの文脈」 |
| S15 | input method だけを destroy すると、IME が去ったものとして扱われる | [ADR4] 1.5 |
| S16 | ふだんのログに、生のキー・keysym・未確定文字・確定した文字・周りの文字を出さない(Seinas の側のログの決まり。Kieli も同じにする) | [ADR4] 1.6 の「ログ」、`docs/design.md` の 6 |

Seinas の回帰のテスト(ADR 0004 の 4 の表)は、テスト用の IME を使って、標準の input-method-v2 と text-input-v3 の振る舞いを見るものです。Kieli は、その 6 項目と同じ流れを、自分をつないで通す必要があります(6 の C1〜C7)。

| # | Seinas の確認項目 | Seinas のテスト(抜粋) | Kieli の側で守ること |
| --- | --- | --- | --- |
| 1 | activate・deactivate・done と、アプリの enable・disable・commit が対応する | `the_activation_follows_the_application_enabling_and_disabling_the_text_input` | 3.4 |
| 2 | 変換中の文字の更新・消去・確定が正しい順で届く | `the_preedit_is_updated_cleared_and_committed_in_order` | 3.5 |
| 3 | フォーカスを変える前の応答が、新しいアプリへ入らない | `an_answer_sent_before_a_focus_change_does_not_reach_the_new_window` ほか | S6・S7 |
| 4 | keymap・修飾キー・押下と解放が整合する | `keys_go_through_the_ime_and_come_back_to_the_window` ほか | 2.2、2.4 |
| 5 | IME の切断・destroy・緊急解除で、変換中の文字や押されたキーが残らない | `a_disconnected_ime_returns_the_keys_and_the_window_resets_its_preedit` ほか | 4 |
| 6 | 文字を出さずにキーを使っても、異常と判定されない | `an_ime_that_swallows_a_key_is_not_bypassed` | S4 |

## 6. 第 2 段階の完了条件(案)

Seinas の側の判定 [ADR4] 4 の「Seinas との統合」は、「テスト用の IME の代わりに Kieli をつなぎ、`crates/seinas-standalone/tests/ime.rs` と同じ流れと、ふつうの入力、文字を出さないキー、フォーカスの切り替え、押しっぱなし、IME の接続断、遅れて届く応答、keymap の変更、text-input を使わないアプリが通ること。入れ子(WSLg)では、親の IME との二重変換が起きないことも見る」です。これに、Kieli の第 1 段階から残したものを足して、次を完了条件の案とします。

| # | 条件 | 確かめ方(案) |
| --- | --- | --- |
| C1 | ふつうの入力:Seinas の上の text-input-v3 のアプリ(Seinas のテスト用のアプリ `test-client window --text-input`)に、ローマ字で打ったひらがなが、未確定文字として出て、Enter で確定して入る | 自動(9 の論点 6 の形)と、入れ子での手の確かめ |
| C2 | 編集と取消:Backspace(保留のローマ字から)・Esc が、アプリの未確定文字に正しく出る | 同上 |
| C3 | 文字を出さないキー(切り替えのキー、取っておいたキー)で、直接の経路に戻らない | 同上(Seinas の確認項目 6) |
| C4 | 使わないキー(直接入力のモードの英字、矢印など)が、virtual-keyboard でアプリへ届き、押下と解放が組になる。修飾キーの状態がアプリと合う | 同上(確認項目 4) |
| C5 | フォーカスの切り替え(Alt+Tab・最小化・閉じる)で、未確定文字が取り消され、前のアプリへの応答が新しいアプリへ入らない。入力モードは引き継がれる | 同上(確認項目 1・3) |
| C6 | 確定してから渡す形(未確定文字があるときの Ctrl の組み合わせ)で、アプリに確定が先に入り、その後にキーが効く | 同上。`docs/design.md` の 2.3 の決定の条件 |
| C7 | 接続断・緊急解除で、アプリに未確定文字や押されたキーが残らず、次のキーが直接届く。Kieli は決めた形(9 の論点 5)で終わる | 同上(確認項目 5) |
| C8 | パスワードの欄(content_type)で、直接入力になる。ふつうの欄に戻ると、元のモードで変換する | 自動(テスト用のアプリに content_type を設定する口が要る。Seinas の側の変更になるので、運用者と相談) |
| C9 | 押しっぱなし:Kieli が消費したキーの繰り返し(9 の論点 3 で決めた形)と、戻したキーのアプリでの繰り返しが、それぞれ二重にならない | 入れ子での手の確かめ |
| C10 | 操作感:キーを押してから、アプリに未確定文字が出るまでの時間を測る(Seinas → Kieli → Seinas の 1 往復の遅れ [ADR4] 1.2 の「費用」) | ログの時刻か、テストで測る。基準の値は運用者が決める |
| C11 | keymap の変更(入れ子で親の keymap に置き換わる場面)で、Kieli が新しい keymap で読む | Smithay の 8a270232 を Seinas が取り込んだ後に確かめる [patches] |
| C12 | text-input を使わないアプリでは変換せず、キーがそのまま届く | 自動 |
| C13 | 入れ子(WSLg)で、親の IME との二重変換が起きない | 入れ子での手の確かめ |
| C14 | Kieli のログに、キー・未確定文字・確定した文字が出ない | ログの中身を確かめるテスト |
| C15 | Kieli のリポジトリの検査と CI が通る。kieli-core は第 1 段階の決まり(no_std、依存の一覧)を保つ | `tools/check.sh`、CI |

## 7. 段の分け方(案)

1 つずつ課題に起票できる大きさにしてあります。それぞれの段で、`tools/check.sh` を通してコミットし、止まって報告します。

| 段 | 内容 | 完了条件 |
| --- | --- | --- |
| 2-0 | 9 の論点を運用者が決め、この文書を「決定」に書き換える | 論点 1〜8 に決定と理由が書かれている |
| 2-1 | kieli-wayland の骨組み:IME のためのソケットへつなぎ、input method・grab・virtual-keyboard を作る。キーは全部そのまま virtual-keyboard で戻す(変換しない)。`unavailable`・global が無いとき・接続断で、決めた形で終わる。依存の見張り(kieli-core に Wayland の crate が入らないこと)は今のまま | 入れ子の Seinas とテスト用のアプリで、キーが変わらずに届く。2 つ目の Kieli は `unavailable` で終わる。緊急解除で Kieli が終わり、次のキーが直接届く |
| 2-2 | keymap(XKB):grab の keymap から XKB の状態を作り、keysym を `Event` に直す。virtual-keyboard の keymap を、自分の memfd に mmap で書いて送る | keysym から `Event` への対応のユニットテスト(jp106 と us の keymap の文字列から)。virtual-keyboard の keymap を、write を使わずに送ること |
| 2-3 | kieli-core とつなぐ:activate・deactivate・done・content_type、未確定文字と確定の届け方(3.5)、渡したキーの記録 | C1〜C6・C8・C12 |
| 2-4 | キーの繰り返し(論点 3 で決めた形) | C9 |
| 2-5 | 切れたとき(論点 5 で決めた形)と、ログの決まり | C7・C14 |
| 2-6 | 統合の確かめ:論点 6 で決めた形の自動の確かめと、入れ子での手の確かめの手順書、遅れの測定、第 2 段階の確かめの表(`docs/stage1-acceptance.md` と同じ形) | C10・C11・C13・C15 と、全体の表 |

2-2 は、2-1 と並べて進められます(XKB の部分は、接続が無くてもユニットテストで確かめられるため)。

## 8. 先の段(ZeikOS の上で動かす段)のための注意

第 2 段階の作りは Linux 向けのままでよい、とされています。今わかっている ZeikOS の制限(運用者の指示文、2026-11-04)に、今の設計が当たるかを書きます。

### 8.1 memfd への write() は EBADF(mmap で書くのは問題ない)

- 当たる所:virtual-keyboard の keymap を、Kieli が自分の memfd に書いて送る所(2.4)。Seinas の [zreq] の表でも、memfd への write は「無い」(EBADF)とされている。
- 避け方:memfd を `memfd_create` で作り、`ftruncate` で大きさを決め、`mmap(PROT_READ|PROT_WRITE, MAP_SHARED)` で書いて `munmap` する形を、Linux でも既定にする。Linux でもこの形で動くので、Linux と ZeikOS で道を分けなくてよい。
- 既定にできるか:できる。ただし、ZeikOS の memfd には、次の制限もある [zreq] の「共有メモリー」の表。
  - `ftruncate` は 1 回だけで、32 KiB まで。jp106 の keymap は、libxkbcommon で文字列にすると 65,268 バイト(us は 64,756 バイト。2026-11-04 にこのホストの libxkbcommon で測った。終わりの NUL を含む)で、32 KiB を越える。コンポジタが配る keymap も同じ大きさなので、Kieli だけの問題ではない(ZeikOS の側の上限を上げるか、keymap を小さくする必要がある。Seinas の [zreq] の「新しい課題の候補」の 1)。
  - memfd は、システム全体で 2 つまで。コンポジタの keymap、クライアントの pool、Kieli の virtual-keyboard の keymap で足りなくなる(同じ)。
  - keymap を替えるたびに memfd を作り直す(ftruncate を 2 回できないため)。前の memfd は、送った後に閉じる。

### 8.2 1 回の sendmsg で運べる fd は 1 つ。受ける前に 2 回目を送ると EAGAIN

- 当たる所:Kieli から Seinas へ fd を送るのは、virtual-keyboard の keymap だけ(起動のときに 1 回と、keymap が替わったとき)。Seinas から Kieli へは、grab の keymap が届く(向きが違うので、ぶつからない)。
- 当たりうる場面:keymap が続けて 2 回替わり、Seinas が 1 つ目を読む前に 2 つ目を送るとき。wayland-client が、fd 付きの要求を 2 つ、1 回の sendmsg にまとめて送るとき(wayland-backend は、たまった要求の fd をまとめて送ることがある。推測。確かめていない)。
- 避け方:fd を付けた要求を送ったら、すぐに flush し、`wl_display.sync` の返事が届くまで、次の fd 付きの要求を送らない(keymap の変更は、最後のものだけを送る)。

### 8.3 poll は 4 本まで。POLLIN と POLLOUT を同時に待つ形と、ソケットの POLLOUT は EINVAL

- 当たる所:kieli-wayland のイベントループ。待つものは、Wayland のソケットと、キーの繰り返しのタイマーの 2 つ。
- 避け方:
  - ソケットは POLLIN だけで待つ。キーの繰り返しは、timerfd を使わず、poll の timeout で待つ(fd は 1 本で済む。ZeikOS の timerfd はシステム全体で 8 個まで [zreq])。
  - 送る側:flush が「今は送れない」(EAGAIN)を返したら、POLLOUT を待たずに、短い timeout(たとえば 10 ms)の後に flush し直す。
  - wayland-client の `blocking_dispatch` など、中で poll を呼ぶものは使わず、Kieli が自分で poll を呼ぶ(中で POLLOUT を待つかもしれないため。推測。確かめていない)。
- この形は Linux でも動くので、第 2 段階からこの形にしておく案です。

### 8.4 そのほか

- Smithay 0.7.0 が grab へ渡す keymap の一時ファイルは、ZeikOS では作れない(O_TMPFILE が無い)[zreq] の「一時ファイル」の行。Seinas の側の課題で、Kieli は届いた fd を読むだけ(mmap で読めなければ、read で読む形にしておく)。

## 9. 運用者が GPT-6 に相談する論点

この節は、そのまま GPT-6 に貼れる形にしてあります。

---

**前提**:Kieli は Rust で一から作る日本語 IME。kieli-core(no_std + alloc。ローマ字からひらがなへの変換と、未確定文字の状態。第 1 段階で完成)の上に、Wayland の接続層 kieli-wayland を作る。kieli-wayland は別のプロセスの IME として、標準の input-method-unstable-v2 と virtual-keyboard-unstable-v1 で、Smithay 0.7.0 で作ったコンポジタ Seinas につなぐ。最初に動かすのは Linux(WSL2 の WSLg の中の入れ子の Seinas)で、後で自作 OS の ZeikOS の上の Seinas でも動かす。ZeikOS には、memfd への write が使えない、1 回の sendmsg で fd は 1 つ、poll は 4 本までで POLLOUT が使えない、などの制限がある。Seinas は、IME にキーごとの返事を求めず、緊急解除のキーで IME の接続を閉じる(IME からは、ただの切断に見える)。

### 論点 1:Wayland のライブラリ

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. wayland-client と wayland-protocols-misc(生成したプロトコルの定義) | 型が付いた API で、プロトコルの誤りが減る。Seinas と同じ系列の crate(wayland-client 0.31.15、wayland-protocols-misc 0.3.12)で、版をそろえられる | 依存が増える。中の送り方(fd のまとめ方、poll の呼び方)が見えにくく、ZeikOS の制限に当たる所を確かめる手間がある |
| B. ソケットへ直に書く(Seinas のテスト用の IME と同じ形) | 依存が無い。送り方と fd の扱いを全部自分で決められ、ZeikOS の制限を避けやすい | プロトコルの組み立てと読み取りを全部書く(数百行)。誤りを型で防げない |
| C. smithay-client-toolkit | イベントループや keymap の扱いまで揃っている | input-method-v2 の扱いは自分で足す。依存がいちばん大きい。calloop のイベントループが ZeikOS の poll の制限に当たるかもしれない |

Kieli の推し:A。ただし、自分で poll を呼ぶ形(`prepare_read` と `read`)にして、`blocking_dispatch` は使わない。ZeikOS の段で送り方が問題になったら、B に移れるよう、Wayland の部分を 1 つのモジュールに閉じる。

### 論点 2:XKB(keymap と keysym)の扱い

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. xkbcommon crate(0.8.0。C の libxkbcommon を使う) | 標準の実装で、どの keymap でも正しく読める。Seinas と同じ。ZeikOS 向けの静的ビルドにも、Seinas の側で libxkbcommon が入っている | C のライブラリに依存する。いずれ自前に置き換える前提(運用者の決定 K8)と合わない部分がある |
| B. 純 Rust の XKB の実装(既存の crate か自作) | C に依存しない | 既存の crate の完成度は確かめていない(推測)。自作は大きい |
| C. keymap を読まず、evdev のキーの番号から文字への表を持つ(jp106 と us だけ) | 小さく、依存が無い | Seinas が配る keymap と食い違うおそれがある。配列の設定を変えると合わなくなる |

Kieli の推し:A。第 2 段階は Seinas と同じ形で始め、置き換えは後の課題にする。

### 論点 3:Kieli が消費したキーの繰り返し

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. grab の `repeat_info` に従って、Kieli がタイマーで同じ Event を繰り返す。途中で消費しない結果になったら止める | ふつうの IME と同じ操作感(Backspace を押し続けて消せる) | タイマーが要る。止めどころの決まりが要る |
| B. 繰り返さない(第 2 段階では入れない) | 簡単 | Backspace を押し続けても 1 文字しか消えない |
| C. 押し始めに消費したキーも、繰り返しの分は virtual-keyboard でアプリへ渡す | Kieli にタイマーが要らない | 未確定文字があるのにアプリにキーが入り、状態がずれる |

Kieli の推し:A。

### 論点 4:入力文脈がアクティブでないときの切り替えのキー

text-input を使わないアプリや入力欄の外で、半角/全角などを押したとき。

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. 消費して、seat の入力モードだけを変える | どこで押しても切り替えが同じに働く(ADR 0004 の 1.2 の理由と合う)。タスクバーの表示(後の段)とも合う | アプリはそのキーを受け取れない |
| B. virtual-keyboard でアプリへ渡す | アプリがそのキーを使える | 入力欄に入ってからでないと切り替えられない |

Kieli の推し:A。

### 論点 5:接続が切れたとき(緊急解除と、コンポジタが落ちたとき)

Kieli は、緊急解除とコンポジタが落ちたときを見分けられない(どちらも EOF か EPIPE)。

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. ログに出して、決めた終わりの値で終わる。つなぎ直さない。起こし直すのは起動の仕組み | 緊急解除の意味(IME を止める)を保てる。簡単 | コンポジタが再起動したとき、起動の仕組みが Kieli を起こし直す必要がある |
| B. 少し待ってからつなぎ直す(回数に上限) | コンポジタの再起動に自分で追いつく | 緊急解除の後に、Kieli が勝手に戻ってくる。不具合のある Kieli だと、解除しても戻る |
| C. ソケットが一度消えてから現れたとき(コンポジタの再起動)だけ、つなぎ直す | 緊急解除では戻らず、再起動には追いつく | ソケットのファイルを見張る仕組みが要る。ZeikOS では、ソケットの名前がファイルにならない(Seinas の zeikos-requirements)ので、同じ形が使えない |

Kieli の推し:A。

### 論点 6:統合の自動の確かめ方

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. Kieli の CI で、Seinas のリポジトリを決まった版で取り、seinas-standalone(fake screen とテスト用の入力インターフェース)とテスト用のアプリを動かして、Kieli をつなぐ | 本物の Seinas で確かめられる | Kieli の CI が Seinas のビルド(C のライブラリ、Smithay の fork)に依存し、重く、壊れやすい |
| B. Kieli のリポジトリに、input-method-v2 と virtual-keyboard だけを持つ小さな偽のコンポジタ(wayland-server)を置き、テストで使う | 速く、Kieli だけで閉じる。順番や切断を細かく作れる | 本物の Seinas との違いを見落とす。偽物の保守が要る |
| C. 自動にせず、入れ子の Seinas で手で確かめる手順書だけにする | 作る物が無い | 毎回の確かめが手作業で、戻りに気づきにくい |

Kieli の推し:B を CI に置き、A は手元で動かすスクリプト(運用者か実装の担当が、決まった時に回す)にする。C の手順書も、入れ子での操作感と二重変換の確かめのために作る。

### 論点 7:切り替えのキー(jp106 と us)

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. jp106:半角/全角で切り替え、カタカナ/ひらがなでひらがな、英数・無変換で直接入力。us:決まったキーを置かない(設定で決める) | jp106 で一般的な日本語 IME と近い(推測。ほかの IME の既定は確かめていない) | us の利用者は、設定するまで切り替えられない |
| B. A に加えて、us では Ctrl+Space で切り替え | us でもすぐ使える | Ctrl+Space を使うアプリ(エディタの補完など)とぶつかる |
| C. jp106 では A、us では右 Alt の単独押しで切り替え | ぶつかるアプリが少ない(推測) | 単独押しの見分け(押して離すまでにほかのキーが無い)が要る。Seinas の Alt+Tab と組み合わせの扱いが要る |

Kieli の推し:第 2 段階は A(Seinas の既定の keymap は jp106 なので)。us の既定は、設定の仕組みを作る段で決める。

### 論点 8:確定してから渡す形の順番

未確定文字があるときに、Kieli が使わないキー(Ctrl+S など)を押すと、Kieli は確定してからキーをアプリへ渡す(第 1 段階の決定)。確定は input-method-v2 の commit で、キーは virtual-keyboard で送るので、別のオブジェクトになる。

| 案 | 長所 | 短所 |
| --- | --- | --- |
| A. 同じ接続で、commit の後に virtual-keyboard の key を送る(待たない) | 遅れが無い。Kieli からの要求は 1 つの接続の中で順に処理される | アプリの側で、text-input の done(確定)とキーの順が保たれるかは、コンポジタの作りしだい(Seinas では保たれると考えているが、確かめていない) |
| B. commit を送った後に `wl_display.sync` の返事を待ってから、key を送る | コンポジタが確定を処理し終えたことを確かめてから渡せる | 1 往復の遅れが足される。アプリの側の順は、これでも保証されない |
| C. 確定せずにキーだけを渡す(第 1 段階の決定を変える) | 順番の問題が無い | アプリの状態と未確定文字がずれる |

Kieli の推し:A。第 2 段階の完了条件(C6)で、アプリの側の結果を確かめる。

---

## 10. この文書で決めないもの

- 候補ウィンドウ(第 3 段階)。input popup surface を Kieli が描くか。
- タスクバーへの入力モードの表示(K8。Seinas の ADR 0004 の 1.13 と 3 の Q6)。
- 設定のファイル(切り替えのキーの変え方など)。
- ZeikOS の上で動かす段の作り(8 は注意だけ)。
