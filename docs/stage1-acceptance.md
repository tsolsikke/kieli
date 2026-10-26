# 第 1 段階の確かめ(初版の判定の材料)

第 1 段階(kieli-core と kieli-cli)の完了条件と、それぞれを何で確かめたかの表です。テストの名前は、`cargo test` で動くもの(`crates/kieli-core/src/` と `crates/kieli-cli/` の中)です。「見本」は `examples/basic.txt` の節の番号で、出力は `examples/basic.expected` と比べています(`docs/try-kieli-cli.md`)。

この表は 2026-10-26 の時点のものです。

## 1. 作業の課題の受け入れ条件

課題の受け入れ条件は、課題の本文を読んでいないので、運用者の指示文(2026-10-25・2026-10-26)に書かれた作業の範囲から並べました。課題の本文と違えば、直します。

| 条件 | 確かめたもの | 状態 |
| --- | --- | --- |
| kieli-core の型(イベント・結果・セッション)と、Backspace・確定・取消・モードの切り替え・上限の動き(設計文書の 9 の順 2) | `enter_commits_the_preedit`、`escape_cancels_the_preedit`、`backspace_deletes_one_scalar_value`、`mode_keys_are_consumed_without_a_preedit`、`the_mode_switch_commits_the_preedit_first`、`a_character_over_the_limit_is_refused_without_any_change`、`editing_keys_work_after_the_limit` | 確かめた |
| `would_consume` と `handle` が食い違わない | `would_consume_and_handle_agree_on_every_short_sequence`(長さ 5 までの手順の並びを全部)、`grouped_keys_behave_like_their_representative`(代表にまとめたキーの見張り) | 確かめた |
| 判断を待っていた振る舞いを、運用者の決定のとおりにする | 「決定:」のコメントを付けたテスト(下の 2 の表) | 確かめた |
| ローマ字のテーブル(設計文書の 9 の順 3) | `every_rule_converts_alone`、`inputs_are_unique`、`every_rule_is_well_formed`、`basic_words`、`n_rules`、`sokuon`、`small_kana_with_x_and_l`、`undefined_spellings_keep_the_first_letter` | 確かめた |
| ひらがなのモードの記号が、全角にする処理より先に効く | `the_symbols_are_in_the_table`、`the_symbols_win_over_the_full_width_conversion`、見本の 7 | 確かめた |
| 保留のローマ字の Backspace | `backspace_deletes_a_pending_letter_first`、見本の 10 | 確かめた |
| kieli-cli を 1 行ずつ読む形(設計文書の 9 の順 4) | `quoted_text_becomes_characters`、`special_keys`、`errors`、`an_error_line_does_nothing_and_does_not_show_the_input`、`a_session_from_typing_to_commit`、`every_kind_of_result_is_shown`、`leave_and_fields`、`the_shortcut_commits_and_passes`、`comment_lines_are_skipped`、`an_error_line_makes_the_exit_status_fail_and_the_rest_runs`、`a_bad_option_is_refused` | 確かめた |
| 運用者が試す手順と、見本の出力の確かめ(設計文書の 9 の順 6) | `docs/try-kieli-cli.md`、`the_basic_example_gives_the_expected_output` | 確かめた |
| `tools/check.sh` と CI が通る | `tools/check.sh`、`tools/check-sabotage.sh`、GitHub Actions の CI | push の後に CI を確かめる |

## 2. 設計文書の 9 の、第 1 段階の完了条件

### 2.1 1〜7 で決めた規則が、それぞれテストになっていること

| 設計文書の項目 | 確かめたもの | 状態 |
| --- | --- | --- |
| 1.2 変換のしかた(照らす順、保留の表示、テーブルの範囲) | `basic_words`、`undefined_spellings_keep_the_first_letter`(決まった規則が先)、`every_rule_converts_alone`、`a_character_in_the_hiragana_mode_starts_a_preedit`(保留は全角) | 確かめた |
| 1.3 ん(n・nn・n'・xn、確定のときの n) | `n_rules`、見本の 4 | 確かめた |
| 1.4 促音(子音の重ね、tch。ll・xx はならない) | `sokuon`、見本の 5 | 確かめた |
| 1.5 小書き文字(x と l、ヵ・ヶ) | `small_kana_with_x_and_l`、見本の 6 | 確かめた |
| 1.6 未定義の綴り | `undefined_spellings_keep_the_first_letter`、見本の 9 | 確かめた |
| 1.6 英字・数字・記号の幅(全角)、大文字は打ったまま | `literal_characters_are_full_width_and_capitals_stay_capitals`、`capitals_digits_and_kana_are_kept_as_typed`、`ascii_becomes_full_width`、見本の 8 | 確かめた |
| 1.6 記号 5 種類 | `the_symbols_are_in_the_table`、`the_symbols_win_over_the_full_width_conversion`、見本の 7 | 確かめた |
| 1.6 保留の後にローマ字でない文字が来たとき | `capitals_digits_and_kana_are_kept_as_typed`(kanA・k1) | 確かめた |
| 2.2 未確定文字が無いときのキー(渡す) | `keys_without_a_preedit_go_to_the_application`、`characters_and_space_in_the_direct_mode_go_to_the_application`、見本の 1 | 確かめた |
| 2.2 Backspace の単位 | `backspace_deletes_one_scalar_value`、`backspace_deletes_a_pending_letter_first`、見本の 10 | 確かめた |
| 2.2 Enter(確定)・Esc(取消) | `enter_commits_the_preedit`、`escape_cancels_the_preedit`、見本の 3・11 | 確かめた |
| 2.3 モードの切り替え(確定してから) | `the_mode_switch_commits_the_preedit_first`、見本の 15 | 確かめた |
| 2.3 今と同じモードを指すモードのキー | `a_mode_key_for_the_current_mode_is_consumed_and_changes_nothing`、見本の 2 | 確かめた |
| 2.3 取っておいたキー(Space・F6〜F10・左右・Home・End・Delete・上下・Tab・PageUp・PageDown) | `reserved_keys_keep_the_preedit_and_report_unsupported`、見本の 12 | 確かめた |
| 2.3 Kieli が使わないキー(確定してから渡す) | `an_unused_key_commits_and_goes_to_the_application`、`other_function_keys_are_treated_as_unused_keys`、`a_control_character_while_composing_is_treated_as_an_unused_key`、見本の 14 | 確かめた。アプリでの順番と編集の結果は、第 2 段階で確かめる(決定の条件) |
| 2.3 未確定文字が無いときの Space(全角の空白) | `space_without_a_preedit_commits_a_full_width_space`、見本の 13 | 確かめた |
| 2.3 起動したときのモード(直接入力) | `a_new_session_starts_in_the_direct_mode`、見本の 1 | 確かめた |
| 3 入力先を失ったとき(取り消しと確定を別の操作として持つ) | `cancel_and_commit_are_available_without_a_key`、`leave_and_fields`、見本の 17 | kieli-core と kieli-cli では確かめた。Wayland の deactivate で取り消すことは、第 2 段階(kieli-wayland)で確かめる |
| 4 文字位置(UTF-8 のバイト、文字境界) | `the_cursor_is_at_the_end_of_the_preedit`、食い違いのテストの中の文字境界の確かめ | 確かめた。UTF-16 への変換は、TSF の接続層を作るときに作る(初版に入れない決定) |
| 5 未確定文字の上限 | `the_default_limit_is_1024_bytes`、`a_character_over_the_limit_is_refused_without_any_change`、`editing_keys_work_after_the_limit`、`every_kind_of_result_is_shown`、見本の 18 | 確かめた |
| 6 直接入力の欄・残さない入力 | `a_direct_field_passes_every_key_and_keeps_the_chosen_mode`、`entering_a_direct_field_does_not_commit_the_preedit`、`mode_keys_in_a_direct_field_change_only_the_chosen_mode`、`the_sensitive_flag_is_kept_and_does_not_change_the_conversion`、見本の 16 | 確かめた。content_type から欄の種類を決めるのは、第 2 段階(kieli-wayland)で確かめる。初版には履歴・学習・ログが無いので、「残さない」を確かめる所はまだ無い |
| 7 入力モードは接続層が持ち、セッションに渡す | `sessions_do_not_share_state`、`leave_and_fields`(入力先を失ってもモードを保つ)、見本の 17 | 確かめた |
| 8.3 状態はインスタンスごと | `sessions_do_not_share_state` | 確かめた |

### 2.2 そのほかの完了条件

| 条件 | 確かめたもの | 状態 |
| --- | --- | --- |
| `tools/check.sh` と CI が通る | `tools/check.sh`(fmt・clippy・test・rustdoc・no_std・依存)、`tools/check-sabotage.sh`(各検査が壊したときに落ちる)、GitHub Actions の CI | push の後に CI を確かめる |
| kieli-cli で、ローマ字からひらがなへの変換と、編集(Backspace)・確定・取消が動く(実行の記録) | 見本の全体と `examples/basic.expected`、`the_basic_example_gives_the_expected_output` | 確かめた |
| kieli-core が std の無い target でビルドでき、依存が一覧の中だけ | `tools/check.sh` の「no_std」と「kieli-core の依存」 | 確かめた |

## 3. 足りない所

- 課題の本文の受け入れ条件は、読んでいないので、1 の表は指示文から並べたものです。
- 第 2 段階に残すもの(上の表の「第 2 段階で確かめる」):Kieli が使わないキーを確定してから渡すときの、アプリでの順番と編集の結果、Wayland の deactivate での取り消し、content_type から欄の種類を決めること。操作感・キーの繰り返しも、第 2 段階で確かめる(設計文書の 10.3)。
- 運用者が自分で kieli-cli を試し、見本の出力を見ること(`docs/try-kieli-cli.md`)は、運用者の側で行う。
