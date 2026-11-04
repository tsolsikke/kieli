# Kieli の第 2 段階の設計

第 2 段階は、Linux の上の Seinas で、Kieli を通して、text-input-v3 に対応したアプリにひらがなを入力する段です。この文書は、その設計です。コードはまだ書いていません。

- 「決定」と書いたものは、運用者が決めたものです(第 1 段階の設計文書 `docs/design.md`、Seinas の ADR 0004、この文書の 9 の決定(2026-11-04。GPT-6 の第二意見を受けて運用者が決めた))。「案」と書いたものは、書き手の案で、運用者が決めたものではありません。
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
| 切り替えのキー(半角/全角など)と、英字配列での組み合わせ | `docs/design.md` の 2.3、Seinas の ADR 0004 の 3(Q7) | 2.3、9 の決定 7 |

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

1. XKB の状態を進め、押したキーなら keysym と文字を得る(2.3)。grab の `key` のキーの番号は evdev の番号で、XKB のキーコードは、それに 8 を足したもの(9 の決定 2 の条件で確かめる)。
2. 押したキーを、kieli-core の `Event` に直す(2.3 の表)。直せないキー(修飾キーだけ、など)は kieli-core へ渡さない。
3. 入力文脈がアクティブなら(3.4)、`Session::handle` を呼ぶ。アクティブでなければ、3.6 の決まりで扱う。
4. `Outcome` に従って、input-method の要求と virtual-keyboard のキーを送る(3.5)。
5. 押したキーの配り先(Kieli が消費した、または virtual-keyboard でアプリへ渡した)を、キーの番号ごとに覚える。離したキーは、押したときに渡したものだけを渡す。押したときに Kieli が消費したキーの「離した」は、どこへも渡さない(繰り返しの途中で消費しなくなったときも同じ。3.7)。この配り先の記録は、2-4(繰り返し)の前に固める(7)。

`modifiers` イベントは、XKB の状態に入れ、そのまま virtual-keyboard の `modifiers` で送り返す(アプリの修飾キーの状態を、物理のキーボードと合わせるため)。Seinas は、IME が grab で受けた物理の modifiers を送り返すのを、ふつうの形としている [ADR4] 1.9「modifiers の限り」。

### 2.3 keymap の扱い(XKB)

- grab の `keymap` イベントで、形式(xkb_v1)・fd・大きさが届く。fd は読むだけで mmap(PROT_READ、MAP_PRIVATE)し、XKB の keymap を作る。Seinas の既定の keymap は、layout `jp`、model `jp106`(設定で `us` などに変えられる)[ADR4] 1.12。
- keymap が替わったら(grab に新しい `keymap` が届いたら)、XKB の状態を作り直し、virtual-keyboard にも新しい keymap を送る。ただし、Smithay 0.7.0 は、keymap が替わっても grab へは送らない(上流の 8a270232 で直った。Seinas は、Kieli の第 2 段階の前に取り込む候補としている)[patches]「取り込む候補」。取り込むまでは、入れ子の構成で親の keymap に置き換わると、Kieli は古い keymap のままキーを読む。
- keysym から `Event` への対応(切り替えのキーは 9 の決定 7。判定は、受け取った keymap の keysym で行う):

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
| `Zenkaku_Hankaku`(jp106 の半角/全角) | `ToggleMode`(既定の割り当て) |
| `Hiragana_Katakana`(jp106 のカタカナ/ひらがな) | `SetMode(Hiragana)`(既定の割り当て) |
| `Eisu_toggle`(jp106 の英数)・`Muhenkan`(無変換) | `SetMode(Direct)`(既定の割り当て) |
| Ctrl・Alt・Super を押したままの文字、ほかの keysym | `Other` |
| 修飾キーだけ(`Shift_L` など) | kieli-core へ渡さない(virtual-keyboard でそのまま戻す) |

- `Char` は、物理のキーの位置ではなく、keymap から得た文字で作る(`docs/design.md` の 1.6 の決定)。
- 切り替えのキーの割り当ては、変えたり無くしたりできるようにする(9 の決定 4・7)。us 配列には既定の割り当てを置かず、設定の例を同梱する。割り当てが無いときは、起動のログに、切り替えのキーが決まっていないことを出す。
- XKB の扱いには、Seinas と同じ `xkbcommon` crate(0.8.0。libxkbcommon を使う)を使う(9 の決定 2)。keymap は自分で作らず、受け取ったものを解釈する。modifiers の反映と、実行中の keymap の変更も確かめる。Kieli の依存で、musl の静的ビルドと実行が通るかを、2-1 の早い所で確かめる。

### 2.4 virtual-keyboard の keymap

- virtual-keyboard の `key` を送る前に、keymap が要る [VK] 54〜68 行。grab で届いた keymap の中身を、Kieli が自分で作った memfd に書き、その fd を送る。
- grab で届いた fd を、そのまま送り返さない。Smithay 0.7.0 は、grab へは一時ファイルで keymap を渡す [zreq] の「一時ファイル」の行。テスト用の IME は届いた fd をそのまま送り返すが [test-ime] 238〜256 行、ZeikOS では、コンポジタが memfd でない fd(一時ファイルなど)を mmap すると EBADF になるおそれがある(推測。出典は [zreq] の「クライアントから受け取った keymap の fd の mmap」の行だが、2026-10-12 の ZeikOS で確かめた古い値で、今の ZeikOS では確かめていない。受け取った memfd の mmap が動くことは、運用者の指示文(2026-11-04)で分かっている)。Kieli は、自分の memfd に書いて送れば、どちらの場合でも当たらない。
- memfd へは、`write()` ではなく、ftruncate で要る長さを取ってから mmap(PROT_WRITE、MAP_SHARED)して書く形を既定にする(Linux でも ZeikOS でも同じ形。8.1)。
- 中身と大きさ:grab の `keymap` で届いた大きさ(`size`)のバイト列を、そのまま写して送る(終わりの NUL を含むかどうかも、届いたものに合わせる)。virtual-keyboard の `keymap` の `size` も同じ値にする。送る前に、XKB で解釈できたことを確かめる(9 の決定 2)。
- fd の寿命:送った要求を flush し終えるまで fd を閉じない。flush の後に閉じる(Seinas の側には、ソケットで複製された fd が届くため)。keymap が替わったら、新しい memfd を作る。
- 2-1 で、この最小の部分(grab の keymap を写して virtual-keyboard へ渡す)を入れる(7)。

### 2.5 イベントループと送りの詰まり(9 の決定 1)

- wayland-client の `prepare_read` と `dispatch_pending` を使い、poll は Kieli が自分で呼ぶ。`blocking_dispatch` など、中で待つものは使わない。
- 待つもの:Wayland のソケットの POLLIN と、キーの繰り返しの締切(poll の timeout。3.7)。
- 送り(flush)が EAGAIN になったら、送り残しがある間だけ、次のように待つ。
  - Linux:ソケットの POLLOUT も待つ。送り残しが無くなったら、POLLOUT を外す。
  - ZeikOS:POLLOUT が使えない(8.3)ので、期限つきの poll(短い timeout)で flush を試し直す。
  - どちらも、送り待ちの量に上限を付ける。上限を越えたら、ログに出して終わる(4.1 と同じ扱い)。これは、ADR 0004 で取り消した「キーへの返事のタイムアウト」[ADR4]「取り消した決定」とは別のもので、Seinas の応答を待つものではない。
- EAGAIN・部分送信・EINTR・切断のどれでも、止まったり空回りしたりしないこと(2-1 と 2-5 のテストで確かめる。7)。
- fd の付いた要求を、ライブラリが 1 回の sendmsg にまとめるかを、採る版(wayland-client 0.31.15 と wayland-backend 0.3.17 の予定)で確かめる。Seinas の側の調べでは、wayland-backend は溜まった fd をまとめて送る。まとめる場合は、fd の付いた要求を送ったら、すぐに flush する(8.2)。

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
- 確定してから渡す形(`consumed` が false で `Outcome::commit` が空でない):input-method の `commit_string`(と、要るなら未確定文字を消す `set_preedit_string("", 0, 0)`)→ input-method の `commit(serial)` → virtual-keyboard の `key` の順に、同じ接続で送る(9 の決定 8)。text-input の commit を送るのではない(text-input の要求はアプリが送るもの)。同じ接続の要求は Seinas で順に処理されるが、それでアプリの側の処理の順まで保たれるとは言えない(`wl_display.sync` も、それを保たない)。Seinas が、アプリへ text-input の変更と done を送ってから `wl_keyboard` の key を送るかは、Seinas の側で確かめる(10 の R3)。アプリの側の結果は、6 の C6 で確かめる。

### 3.6 入力文脈がアクティブでないとき

text-input を使わないアプリ(zeyes など)や、入力欄の外では、入力文脈がアクティブになりません。このときは変換せず、キーを virtual-keyboard でそのまま戻します [ADR4] 1.10。

切り替えのキー(9 の決定 4。条件付きで消費する):

- 消費するのは、設定した切り替えのキーだけ。消費して、seat の入力モードだけを変える。ほかのキーは、全部アプリへ渡す。
- 入力欄の外でも、Seinas が grab でキーを Kieli に届けることが前提。ADR 0004 は、Smithay の grab は text-input が有効でないときもキーを IME へ渡す、とソースで確かめている [ADR4] 1.2 が、Seinas の側の確かめを頼む(10 の R2)。
- 割り当ては、変えたり無くしたりできる(2.3)。
- パスワード・pin の欄での一時的な直接入力(`Session::set_direct_field`)は、利用者が選んだモード(seat の入力モード)とは別に持つ(`docs/design.md` の 6 の決定と同じ)。

### 3.7 キーの繰り返し(9 の決定 3)

- 繰り返しは、IME が消費したキーは IME が、virtual-keyboard で戻したキーはアプリが行う。grab には `repeat_info`(1 秒あたりの回数と、始めるまでの時間)が届く [ADR4] 3 の「キーの繰り返し」、[ime]「押されたままのキー」の繰り返しの項。
- Kieli が消費したキー(押下が `consumed` だったもの)を押し続けたら、`repeat_info` に従って、同じ `Event` を `handle` に渡し直す。条件:
  - 繰り返すのは、XKB で繰り返せるキー(`xkb_keymap_key_repeats`)で、かつ Kieli の操作として繰り返してよいキーだけ。切り替えのキー(`ToggleMode`・`SetMode`)は繰り返さない。
  - `repeat_info` の rate が 0 なら、繰り返さない。`repeat_info` が後から変わったら、次の繰り返しから従う。
  - 締切は単調時計で決める。処理が遅れても、遅れた分をまとめて打たない(次の締切は、今の時刻から決め直す)。
  - 繰り返しの途中で、結果が消費しないもの(`consumed` が false)になったら、その時点で繰り返しを止め、キーはアプリへ渡さない(押し始めの「押した」をアプリに渡していないので、急に渡すと「離した」と組にならないため)。たとえば、Backspace を押し続けて未確定文字が空になったら、そこで止まる。
  - 離す・deactivate・切断・keymap の変更で止める。止めた後も、消費した押下の「離した」だけがアプリへ漏れないこと(2.2 の配り先の記録)。

## 4. 切れたとき

### 4.1 緊急解除

- Seinas の緊急解除(既定は Alt+Shift+F12)は、Kieli の接続を閉じるだけで、`wl_display.error` は送らない。Kieli の側からは、ただの切断に見える(読むと EOF、送ると EPIPE)[ADR4] 1.5、3 の「緊急解除のときの見え方」。
- Kieli は、接続が切れたら、繰り返しを止め、未確定文字を持ち越さずに捨て(送る先が無い)、ログに切れたことだけを出して終わる(9 の決定 5)。アプリには、Seinas が text-input の leave を送り、アプリが未確定文字を消す [ime]「IME の復旧」。virtual-keyboard で押したままのキーも、Seinas が離す [ADR4] 1.9。
- 緊急解除は、利用者が IME を止めたいときの操作なので、Kieli が自分でつなぎ直すと、解除の意味が無くなります。つなぎ直すのは、IME か起動の仕組みの役目とされています [ADR4] 3 の「緊急解除のときの見え方」。Kieli は、緊急解除と、Seinas が落ちたときを見分けられません(どちらも EOF)。そのため、自動のつなぎ直しは入れない。ソケットが現れ直しても、それだけではつなぎ直さない。第 2 段階では、利用者か運用者が手で起こし直す(9 の決定 5)。
- 緊急解除のキーは、IME のためのソケットを開く構成では、IME がつながっていなくても予約される。`--no-ime-socket` の構成では予約されず、アプリへ渡る(この構成では Kieli がつなげない)[ADR4] 1.8。Kieli は、このキーを切り替えのキーなどに使わない(どの構成でも Kieli には届かないか、Kieli がいない)。

### 4.2 Seinas が落ちたとき・再起動したとき

- Seinas が落ちると、接続が切れる(EOF か EPIPE)。4.1 と同じく、未確定文字を捨ててログに出し、終わる。
- Seinas が再起動したら、第 2 段階では、Kieli を手で起こし直す(9 の決定 5)。将来、起動の仕組みが自動で起こし直すときは、「利用者が止めた状態」と「明示して有効にし直す操作」を先に設計する。Kieli は、起動の仕組みが起こす(コンポジタは起こさない [ADR4] 3 の「起動と、IME として認められること」)。
- 起動したときの最初の接続(Seinas がまだ準備できていないとき)を、限られた回数だけ試し直すかは、接続した後のつなぎ直しとは別に決める(第 2 段階では入れない。要るようになったら運用者に諮る)。
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

## 6. 第 2 段階の完了条件

Seinas の側の判定 [ADR4] 4 の「Seinas との統合」は、「テスト用の IME の代わりに Kieli をつなぎ、`crates/seinas-standalone/tests/ime.rs` と同じ流れと、ふつうの入力、文字を出さないキー、フォーカスの切り替え、押しっぱなし、IME の接続断、遅れて届く応答、keymap の変更、text-input を使わないアプリが通ること。入れ子(WSLg)では、親の IME との二重変換が起きないことも見る」です。これに、Kieli の第 1 段階から残したものと、9 の決定を足して、次を完了条件とします。

確かめ方の分け方(9 の決定 6):

- 偽のコンポジタ:Kieli のリポジトリに置き、ふだんの CI で回す(表の「偽」)。
- 本物の Seinas での自動の確かめ:段の完了条件にする。全部の CI で回さなくてよいが、完了の前の結果を、Kieli・Seinas・アプリの版つきで残す(表の「本物」)。
- 手順書:入れ子の Seinas で手で確かめるための補助(表の「手」)。
- 本物の text-input-v3 対応アプリを、少なくとも 1 つ使う(6.1)。

| # | 条件 | 確かめ方 |
| --- | --- | --- |
| C1 | ふつうの入力:Seinas の上の text-input-v3 のアプリ(Seinas のテスト用のアプリと、6.1 の本物のアプリ)に、ローマ字で打ったひらがなが、未確定文字として出て、Enter で確定して入る | 偽・本物・手 |
| C2 | 編集と取消:Backspace(保留のローマ字から)・Esc が、アプリの未確定文字に正しく出る | 偽・本物 |
| C3 | 文字を出さないキー(切り替えのキー、取っておいたキー)で、直接の経路に戻らない | 偽・本物(Seinas の確認項目 6) |
| C4 | 使わないキー(直接入力のモードの英字、矢印など)が、virtual-keyboard でアプリへ届き、押下と解放が組になる。修飾キーの状態がアプリと合う。キーの番号(+8)と modifiers が正しく反映される | 偽・本物(確認項目 4) |
| C5 | フォーカスの切り替え(Alt+Tab・最小化・閉じる)で、未確定文字が取り消され、前のアプリへの応答が新しいアプリへ入らない。入力モードは引き継がれる | 偽・本物(確認項目 1・3) |
| C6 | 確定してから渡す形:input-method の commit_string → commit → virtual-keyboard の key の順に送られ(通信の記録)、本物のアプリで、確定が先に入ってからキーが効く。途中でフォーカスが変わっても、古い確定文字が新しい欄に入らない(9 の決定 8。確かめる形は 9 の決定 8 の「判断の要る所」を見る) | 偽(順の記録)・本物(通信の記録とアプリの結果) |
| C7 | 接続断・緊急解除で、アプリに未確定文字や押されたキーが残らず、次のキーが直接届く。Kieli は繰り返しを止め、未確定文字を持ち越さずに終わる。EAGAIN・部分送信・EINTR で止まったり空回りしたりしない | 偽・本物(確認項目 5) |
| C8 | パスワードの欄(content_type)で、直接入力になる。ふつうの欄に戻ると、元のモードで変換する | 偽・本物。本物には Seinas の側の口が要る(10 の R1。Seinas の課題として、2-3 より前に起票予定) |
| C9 | 押しっぱなし:Kieli が消費したキーは `repeat_info` に従って繰り返し(rate 0 は繰り返さない)、遅れた分をまとめて打たない。途中で消費しなくなっても、キーをアプリへ急に渡さない。戻したキーはアプリが繰り返し、二重にならない。切り替えのキーの長押しは 1 回だけ切り替わる | 偽(時計を差し替えたテスト)・手 |
| C10 | 操作感:キーを押してから、アプリに未確定文字が出るまでの時間を測る(Seinas → Kieli → Seinas の 1 往復の遅れ [ADR4] 1.2 の「費用」) | 本物(時刻の記録)。基準の値は運用者が決める |
| C11 | keymap の変更(入れ子で親の keymap に置き換わる場面)で、Kieli が新しい keymap で読み、virtual-keyboard にも新しい keymap を渡す。繰り返しは止まる | 偽。本物は、Smithay の 8a270232 を Seinas が取り込んだ後 [patches] |
| C12 | text-input を使わないアプリでは変換せず、キーがそのまま届く。入力欄の外でも、設定した切り替えのキーだけは消費してモードを変える | 偽・本物(10 の R2 が前提) |
| C13 | 入れ子(WSLg)で、親の IME との二重変換が起きない | 手 |
| C14 | Kieli のログに、キー・未確定文字・確定した文字が出ない | 偽(ログの中身を確かめるテスト) |
| C15 | Kieli のリポジトリの検査と CI が通る。kieli-core は第 1 段階の決まり(no_std、依存の一覧)を保つ。kieli-wayland は musl の静的ビルドと実行が通る | `tools/check.sh`、CI |

### 6.1 本物の text-input-v3 対応アプリの候補

WSL(Ubuntu 24.04.5)には、どれも入っていません(2026-11-04 に `apt-cache policy` で確かめた。入れるには、運用者の判断が要る。CLAUDE.md の 4)。text-input-v3 に対応しているかは、書き手の知識によるもので、確かめていません(推測)。Seinas の上で起動できるか(使う global が足りるか)も、確かめていません。

| 候補 | パッケージ(入れられる版) | text-input-v3 | 向いている確かめ |
| --- | --- | --- | --- |
| foot(端末) | `foot` 1.16.2-2ubuntu0.1 | 対応している(推測) | 小さく、起動が速い。C1・C2・C5 |
| GTK 4 の見本(gtk4-demo、gtk4-widget-factory) | `gtk-4-examples` 4.14.5+ds-0ubuntu0.10 | GTK 4 は text-input-v3 を使う(推測) | 入力欄の中のカーソルの位置が見えるので、C6 の確定の後の位置の確かめに向く |
| Alacritty(端末) | `alacritty` 0.13.2-1ubuntu1 | winit を通して対応している(推測) | foot が使えないときの代わり |

案:foot と gtk-4-examples の 2 つ。どちらを入れるかは、運用者が決める。

## 7. 段の分け方

1 つずつ課題に起票できる大きさにしてあります(2026-11-04 の指示で直した版)。それぞれの段で、`tools/check.sh` を通してコミットし、止まって報告します。

| 段 | 内容 | 完了条件 |
| --- | --- | --- |
| 2-0 | 9 の論点を運用者が決め、この文書を「決定」に書き換える | 9 に決定と理由と条件が書かれている(この版) |
| 2-1 | kieli-wayland の骨組みと、本物の Seinas への接続:IME のためのソケットへつなぎ、input method・grab・virtual-keyboard を作る。grab の keymap を写して virtual-keyboard へ渡す最小の部分(2.4)を入れる。キーは全部そのまま virtual-keyboard で戻す(変換しない)。`unavailable`・global が無いとき・切断で終わる。入力の中身をログに出さない決まりを入れる。イベントループ(2.5)。偽のコンポジタの最初の形。musl の静的ビルドと実行の確かめ | 本物:入れ子の Seinas とテスト用のアプリで、キーが変わらずに届く(C4 の押下と解放の組)。2 つ目の Kieli は `unavailable` で終わる。緊急解除で Kieli が終わり、次のキーが直接届く。偽:同じ流れと、切断で終わること。C14 の最初の形。musl のビルドと実行の記録 |
| 2-2 | XKB:受け取った keymap の解釈、keysym から `Event` への対応(切り替えのキーの割り当てと、us 配列の設定の例)、キーの番号(+8)と modifiers、実行中の keymap の変更 | keysym から `Event` への対応のユニットテスト(jp106 と us の keymap の文字列から)。偽:keymap の変更で、XKB の状態と virtual-keyboard の keymap が替わる(C11 の偽の部分) |
| 2-3 | kieli-core とつなぐ:activate・deactivate・done・content_type、未確定文字と確定の届け方(3.5)、入力欄の外の切り替えのキー(3.6)。本物のアプリへのひらがなの入力と、確定してから渡す順(9 の決定 8)の確かめ | 偽と本物:C1・C2・C3・C5・C6・C8・C12。本物のアプリ(6.1)での結果を残す。C8 の本物は、Seinas の側の口(10 の R1)ができてから |
| 2-3b | 送りの詰まりの扱いと、押下と離しの配り先の記録を固める(2.2・2.5) | 偽:EAGAIN・部分送信・EINTR・切断で止まらず、空回りしない(C7 の一部)。送り待ちの上限を越えたら終わる。どの順で押して離しても、消費した押下の「離した」がアプリへ漏れない |
| 2-4 | キーの繰り返し(3.7) | C9 |
| 2-5 | 異常の形の仕上げ:切断・緊急解除・Seinas の再起動・送りの詰まり・壊れた keymap などで、決めた形で終わる。ログの決まりの確かめ | C7・C14 |
| 2-6 | 受け入れの確かめ:Kieli・Seinas・アプリの版を固め、本物の Seinas での自動の確かめの結果(版つき)と、手順書での確かめ(C13)と、遅れの測定(C10)を残す。第 2 段階の確かめの表(`docs/stage1-acceptance.md` と同じ形) | C1〜C15 の全部と、全体の表 |

- 2-1 から本物の Seinas につなぐ。2-6 は、初めて統合する段ではなく、版を固めて受け入れの確かめを終える段とする。
- 2-3b は、2-4 の前に置く(繰り返しは、送りの詰まりと配り先の記録の上に作るため)。2-3 と並べて進めてもよい。

## 8. 先の段(ZeikOS の上で動かす段)のための注意

第 2 段階の作りは Linux 向けのままでよい、とされています。今わかっている ZeikOS の制限(運用者の指示文、2026-11-04)に、今の設計が当たるかを書きます。

### 8.1 memfd への write() は EBADF(mmap で書くのは問題ない)

- 当たる所:virtual-keyboard の keymap を、Kieli が自分の memfd に書いて送る所(2.4)。Seinas の [zreq] の表でも、memfd への write は「無い」(EBADF)とされている。
- 避け方:memfd を `memfd_create` で作り、`ftruncate` で大きさを決め、`mmap(PROT_READ|PROT_WRITE, MAP_SHARED)` で書いて `munmap` する形を、Linux でも既定にする。Linux でもこの形で動くので、Linux と ZeikOS で道を分けなくてよい。
- 既定にできるか:できる。今の ZeikOS の memfd の値(運用者の指示文、2026-11-04。Seinas の [zreq] の memfd の行は 2026-10-21 の古い値で、直したコミットはまだ push されていない):
  - memfd は 16 個まで、1 つ 16 MiB、合計 64 MiB。
  - `ftruncate` は、伸ばすのも縮めるのも受ける。
  - SCM_RIGHTS で、memfd とパイプの端を運べる。受け取った memfd の mmap は動く(Seinas のタスクバーの wl_shm が、これで動いている)。
- keymap の大きさ:jp106 の keymap は、libxkbcommon で文字列にすると 65,268 バイト(us は 64,756 バイト。2026-11-04 にこのホストの libxkbcommon で測った。終わりの NUL を含む)。1 つ 16 MiB に収まる。
- keymap が替わったときは、新しい memfd を作って書き、送った後に前の memfd を閉じる(同じ memfd を書き換えると、Seinas が読んでいる途中のものが変わるおそれがあるため)。

### 8.2 1 回の sendmsg で運べる fd は 1 つ。受ける前に 2 回目を送ると EAGAIN

- 当たる所:Kieli から Seinas へ fd を送るのは、virtual-keyboard の keymap だけ(起動のときに 1 回と、keymap が替わったとき)。Seinas から Kieli へは、grab の keymap が届く(向きが違うので、ぶつからない)。
- 当たりうる場面:keymap が続けて 2 回替わり、Seinas が 1 つ目を読む前に 2 つ目を送るとき。wayland-backend が、溜まった要求の fd を 1 回の sendmsg にまとめて送るとき(Seinas の側の調べでは、まとめる。採る版で確かめる。2.5)。
- 避け方:fd を付けた要求を送ったら、すぐに flush し、`wl_display.sync` の返事が届くまで、次の fd 付きの要求を送らない(keymap の変更は、最後のものだけを送る)。

### 8.3 poll は 4 本まで。POLLIN と POLLOUT を同時に待つ形と、ソケットの POLLOUT は EINVAL

- 当たる所:kieli-wayland のイベントループ(2.5)。待つものは、Wayland のソケットと、キーの繰り返しの締切の 2 つ。
- 避け方(9 の決定 1):
  - ソケットはふだん POLLIN だけで待つ。キーの繰り返しは、timerfd を使わず、poll の timeout で待つ(fd は 1 本で済む。ZeikOS の timerfd はシステム全体で 8 個まで [zreq])。
  - 送り残しがあるときだけ、Linux では POLLOUT も待つ。ZeikOS では POLLOUT を使わず、期限つきの poll で flush を試し直し、送り待ちの量に上限を付ける。ZeikOS 向けの切り替えは、ZeikOS の段で入れる(第 2 段階は Linux 向けの作り)。
  - wayland-client の `blocking_dispatch` など、中で poll を呼ぶものは使わない。

### 8.4 そのほか

- Smithay 0.7.0 が grab へ渡す keymap の一時ファイルは、ZeikOS では作れないおそれがある(O_TMPFILE が無い。[zreq] の「一時ファイル」の行で、2026-10-12 の ZeikOS で確かめた古い値。今の ZeikOS では確かめていないので推測)。Seinas の側の課題で、Kieli は届いた fd を読むだけ(mmap で読めなければ、read で読む形にしておく)。

## 9. 決定(2026-11-04)

運用者が、GPT-6 の第二意見を受けて決めた。比べた案(A・B・C)は、321 の報告(2026-11-04 18:28)の版のこの節にある。

### 決定 1:Wayland のライブラリ

A。wayland-client と wayland-protocols-misc を使い、`prepare_read` と `dispatch_pending` で、Kieli が自分で呼ぶ poll に組み込む(2.5)。

- 理由:型の付いた API で、プロトコルの誤りが減る。Seinas と同じ系列の crate で、版をそろえられる。自分で poll を呼べば、ZeikOS の poll の制限に合わせられる。
- 条件:
  - 送りが EAGAIN になったら、送り残しがある間だけ、Linux では POLLOUT も待つ。
  - ZeikOS では POLLOUT が使えないので、期限つきの poll で flush を試し直し、送り待ちの量に上限を付ける。これは、撤回したキーへの返事のタイムアウトとは別のもの。
  - EAGAIN・部分送信・EINTR・切断で、止まったり空回りしたりしないこと。
  - fd の付いた要求をライブラリが 1 回の sendmsg にまとめるかを、採る版で確かめる(Seinas の側の調べでは、wayland-backend は溜まった fd をまとめる)。

### 決定 2:XKB

A。xkbcommon crate を使う(2.3)。

- 理由:標準の実装で、どの keymap でも正しく読める。Seinas と同じで、ZeikOS 向けの静的ビルドにも Seinas の側で入っている。
- 条件:
  - Kieli の依存で、musl の静的ビルドと実行が通るかを、2-1 か 2-2 の早い所で確かめる。
  - keymap は自分で作らない。受け取ったものを解釈し、それと合うものを virtual-keyboard に渡す。
  - キー番号の差(+8)と modifiers の反映、実行中の keymap の変更を確かめる。
  - memfd は、要る長さを取ってから mmap で書く。終わりの NUL・大きさ・fd の寿命も確かめる(2.4)。

### 決定 3:消費したキーの繰り返し

A。grab の `repeat_info` に従って、Kieli が同じ Event を繰り返す(3.7)。

- 理由:ふつうの IME と同じ操作感(Backspace を押し続けて消せる)。Seinas は、消費したキーの繰り返しを IME の役目としている。
- 条件:
  - 繰り返すのは「XKB で繰り返せる」かつ「Kieli の操作として繰り返してよい」キーだけ。切り替えのキーは繰り返さない。
  - `repeat_info` の rate=0 は「繰り返さない」。`repeat_info` が後から変わったら、それに従う。
  - 単調時計を使い、処理が遅れても、遅れた分をまとめて打たない。
  - 途中で消費しなくなっても、それまで渡していなかったキーを、アプリへ急に渡さない。
  - 離す・deactivate・切断・keymap の変更で止める。消費した押下の「離した」だけがアプリへ漏れないこと。

### 決定 4:入力欄の外の切り替えのキー

条件付きで A。消費して、seat の入力モードだけを変える(3.6)。

- 理由:どこで押しても切り替えが同じに働く(ADR 0004 の 1.2 の理由と合う)。
- 条件:
  - 消費するのは、設定した切り替えのキーだけ。ほかはアプリへ渡す。
  - 入力欄の外でも Seinas がキーを Kieli に届けることが前提。これは Seinas の側で確かめる(10 の R2)。
  - 割り当てを変えたり、無くしたりできること。
  - パスワード・pin の欄での一時的な直接入力は、利用者が選んだモードとは別に持つ。

### 決定 5:接続が切れたとき

A。終わる。第 2 段階は手で起こし直す(4)。

- 理由:緊急解除とコンポジタが落ちたときを見分けられず、自動でつなぎ直すと、緊急解除の意味を打ち消すため。
- 条件:
  - 自動のつなぎ直しは入れない。ソケットが現れ直しても、それだけではつなぎ直さない。
  - 将来、自動で起こし直すときは、「利用者が止めた状態」と「明示して有効にし直す操作」を先に設計する。
  - 最初の接続の限られた試し直しは、接続した後のつなぎ直しとは別に決める。
  - 切れたら、繰り返しを止め、未確定文字を持ち越さずに終わる。

### 決定 6:統合の確かめ方

- 偽のコンポジタを Kieli に置き、ふだんの CI で回す。
- 本物の Seinas での自動の確かめを、段の完了条件にする。全部の CI で回さなくてよいが、完了の前の結果(Kieli・Seinas・アプリの版つき)を残す。
- 手順書は補助。
- 本物の text-input-v3 対応アプリを、少なくとも 1 つ入れる。候補と、WSL で入るかは 6.1。
- 理由:偽のコンポジタは速く細かく確かめられるが、本物との違いを見落とすので、本物での結果を完了の前に必ず残す。

### 決定 7:切り替えのキー

A。jp106 では、半角/全角で切り替え、カタカナ/ひらがなでひらがな、英数・無変換で直接入力。us には既定の割り当てを置かない(2.3)。

- 理由:Seinas の既定の keymap は jp106。us の既定のキーは、アプリのショートカットとぶつかりやすい。
- 条件:
  - 判定は、受け取った keymap の keysym で行う。
  - us 配列向けの設定例を同梱する。切り替えのキーが決まっていないときは、そう明示する(起動のログ)。
  - 長押しでは 1 回だけ切り替わり、押下と離しの扱いが合うこと(繰り返さない。消費した押下の「離した」も渡さない)。

### 決定 8:確定してから渡す順

A。同じ接続で、待たずに送る(3.5)。

- 送る順:input-method の `commit_string`(要るなら未確定文字の消去)→ input-method の `commit(serial)` → virtual-keyboard の `key`。text-input の commit を送るのではない。
- 同じ接続の要求は順に処理されるが、それでアプリの側の処理順まで保たれるとは言えない(`wl_display.sync` も、それを保たない)。
- Seinas が、アプリへ text-input の変更と done を送ってから key を送るかを、Seinas の側で確かめる(10 の R3)。
- 確かめる形(指示の形):「か+Tab」(前の欄に確定してから移る)、「か+左矢印」(確定した後の位置から動く)、途中でフォーカスが変わっても、古い確定文字が新しい欄に入らないこと。通信の記録と、本物のアプリでの結果の両方で見る。

判断の要る所(書き手が見つけた食い違い):第 1 段階の決定(`docs/design.md` の 2.3)では、未確定文字があるときの Tab と左右は「取っておくキー」で、消費して未確定文字を残し(「未対応」)、確定してから渡す形にはならない。そのため、上の「か+Tab」「か+左矢印」は、今の決まりのままでは、確定してから渡す順を確かめる形にならない。案:

- 案 A:確かめる形を、Kieli が使わないキー(確定してから渡すもの)に替える。たとえば「か+Ctrl+A」(確定した文字が、すべての選択に入る)、「か+Ctrl+Left」(確定した後の位置から、語の単位で動く)。Tab で欄を移る形は、確かめる形から外す。
- 案 B:第 1 段階の決定を変え、Tab と左右も、確定してから渡すキーにする(第 3 段階の候補の操作や、未確定文字の中の移動の受け口は、そのときに決め直す)。

書き手の推し:案 A(第 1 段階の決定を保ち、確かめたいこと(確定とキーの順)は Ctrl の組み合わせで確かめられるため)。運用者が決めるまで、C6 の確かめる形は決めないでおく。

## 10. Seinas の側に頼むこと

Seinas のリポジトリには書きません。起票はレビューの側が行います。

| # | 頼むこと | なぜ要るか | 確かめる方法の案 |
| --- | --- | --- | --- |
| R1 | テスト用のアプリ(`test-client window --text-input`)に、content_type(hint と purpose)を設定する口 | C8(パスワードの欄で直接入力になること)を、本物の Seinas で自動で確かめるため | 標準入力に `text-input content-type HINT PURPOSE` のような行を足し、送った後の commit で、IME に `content_type` と `done` が届くことを、テスト用の IME の出力で確かめる |
| R2 | text-input が有効でないとき(text-input を使わないアプリ、入力欄の外)も、grab がキーを IME へ届けるか | 決定 4(入力欄の外の切り替えのキー)の前提。ADR 0004 の 1.2 は Smithay のソースで確かめたと書くが、テストで確かめたい | text-input を使わないテスト用のアプリ(`test-client window --keyboard`)にフォーカスがあるときに、テスト用の IME の grab にキーが届き(`ime: key 30 pressed`)、IME が戻したキーがアプリに届くことを、Seinas の回帰のテストにする |
| R3 | アプリへ、text-input の変更(commit_string・preedit)と done を送ってから、`wl_keyboard` の key を送るか | 決定 8(確定してから渡す順)の前提 | テスト用の IME で、`commit_string` と `commit` と virtual-keyboard の `key` を 1 回の flush で続けて送り、テスト用のアプリの出力で、`window: commit …` と `window: applied commit …` の行が、`window: key … pressed` の行より先に出ることを確かめる。`WAYLAND_DEBUG=1` の通信の記録も残す |

## 11. この文書で決めないもの

- 候補ウィンドウ(第 3 段階)。input popup surface を Kieli が描くか。
- タスクバーへの入力モードの表示(K8。Seinas の ADR 0004 の 1.13 と 3 の Q6)。
- 設定のファイル(切り替えのキーの変え方など)。
- ZeikOS の上で動かす段の作り(8 は注意だけ)。
