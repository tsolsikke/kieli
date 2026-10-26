//! セッションの振る舞いのテスト。
//!
//! コメントの「決定」は、設計文書(`docs/design.md`)の、運用者の決定(2026-10-25)の項目を指す。

use alloc::string::String;
use alloc::vec::Vec;

use crate::{Config, Event, Feature, InputMode, Outcome, OutcomeKind, Session};

fn hiragana() -> Session {
    Session::new(Config {
        mode: InputMode::Hiragana,
        ..Config::default()
    })
}

fn direct() -> Session {
    Session::new(Config {
        mode: InputMode::Direct,
        ..Config::default()
    })
}

fn limited(max_preedit_bytes: usize) -> Session {
    Session::new(Config {
        mode: InputMode::Hiragana,
        max_preedit_bytes,
    })
}

fn outcome(kind: OutcomeKind, consumed: bool, commit: &str, preedit: &str) -> Outcome {
    Outcome {
        kind,
        consumed,
        commit: String::from(commit),
        preedit: String::from(preedit),
    }
}

fn passed() -> Outcome {
    outcome(OutcomeKind::Passed, false, "", "")
}

/// 文字を順に入れる(結果は見ない)。
fn type_chars(session: &mut Session, text: &str) {
    for c in text.chars() {
        session.handle(Event::Char(c));
    }
}

const RESERVED_KEYS: [(Event, Feature); 16] = [
    (Event::Space, Feature::Conversion),
    (Event::Left, Feature::CursorMovement),
    (Event::Right, Feature::CursorMovement),
    (Event::Home, Feature::CursorMovement),
    (Event::End, Feature::CursorMovement),
    (Event::Up, Feature::CandidateOperation),
    (Event::Down, Feature::CandidateOperation),
    (Event::Tab, Feature::CandidateOperation),
    (Event::Delete, Feature::CursorMovement),
    (Event::PageUp, Feature::CandidateOperation),
    (Event::PageDown, Feature::CandidateOperation),
    (Event::Function(6), Feature::Conversion),
    (Event::Function(7), Feature::Conversion),
    (Event::Function(8), Feature::Conversion),
    (Event::Function(9), Feature::Conversion),
    (Event::Function(10), Feature::Conversion),
];

/// 取っておかないファンクションキー(Kieli が使わないキーと同じに扱う)。
const UNUSED_FUNCTION_KEYS: [Event; 9] = [
    Event::Function(1),
    Event::Function(2),
    Event::Function(3),
    Event::Function(4),
    Event::Function(5),
    Event::Function(11),
    Event::Function(12),
    Event::Function(13),
    Event::Function(24),
];

// ---- 起動したとき

#[test]
fn a_new_session_starts_in_the_direct_mode() {
    // 決定:起動したときのモードは直接入力(設計文書の 2.3)。
    let session = Session::new(Config::default());
    assert_eq!(session.mode(), InputMode::Direct);
    assert!(!session.is_composing());
}

// ---- 未確定文字が無いとき

#[test]
fn keys_without_a_preedit_go_to_the_application() {
    for mut session in [hiragana(), direct()] {
        for event in [
            Event::Backspace,
            Event::Commit,
            Event::Cancel,
            Event::Left,
            Event::Right,
            Event::Home,
            Event::End,
            Event::Up,
            Event::Down,
            Event::Tab,
            Event::Delete,
            Event::PageUp,
            Event::PageDown,
            Event::Function(1),
            Event::Function(6),
            Event::Other,
            Event::Char('\t'),
        ] {
            assert_eq!(
                session.handle(event),
                passed(),
                "{:?} {event:?}",
                session.mode()
            );
        }
    }
}

#[test]
fn characters_and_space_in_the_direct_mode_go_to_the_application() {
    let mut session = direct();
    assert_eq!(session.handle(Event::Char('A')), passed());
    assert_eq!(session.handle(Event::Space), passed());
    assert!(!session.is_composing());
}

#[test]
fn space_without_a_preedit_commits_a_full_width_space() {
    // 決定:ひらがなのモードの Space は、全角の空白(U+3000)を確定する(設計文書の 2.3)。
    let mut session = hiragana();
    assert_eq!(
        session.handle(Event::Space),
        outcome(OutcomeKind::Committed, true, "\u{3000}", "")
    );
}

#[test]
fn a_character_in_the_hiragana_mode_starts_a_preedit() {
    let mut session = hiragana();
    // テーブルが無いので、文字は未定義の綴りとして、そのまま全角で入る(決定。設計文書の 1.6)。
    assert_eq!(
        session.handle(Event::Char('k')),
        outcome(OutcomeKind::Updated, true, "", "ｋ")
    );
    assert!(session.is_composing());
}

#[test]
fn literal_characters_are_full_width_and_capitals_stay_capitals() {
    // 決定:未確定文字に残る英字・数字・記号は全角。大文字は打ったまま残す(設計文書の 1.6)。
    let mut session = hiragana();
    // 英小文字はローマ字として照らすので、テーブルに無い綴りの文字で確かめる。
    type_chars(&mut session, "qZ0!~");
    assert_eq!(session.preedit().text(), "ｑＺ０！～");
}

#[test]
fn a_non_ascii_character_keeps_its_form() {
    let mut session = hiragana();
    type_chars(&mut session, "あ𠮷");
    assert_eq!(session.preedit().text(), "あ𠮷");
}

// ---- 未確定文字があるとき

#[test]
fn backspace_deletes_one_scalar_value() {
    // 決定:保留のローマ字が無ければ、表示の最後の 1 文字(Unicode のスカラー値 1 つ)を消す。
    let mut session = hiragana();
    type_chars(&mut session, "A𠮷");
    assert_eq!(
        session.handle(Event::Backspace),
        outcome(OutcomeKind::Updated, true, "", "Ａ")
    );
    assert_eq!(
        session.handle(Event::Backspace),
        outcome(OutcomeKind::Updated, true, "", "")
    );
    assert!(!session.is_composing());
    // 未確定文字が無くなった後の Backspace は、アプリへ渡す。
    assert_eq!(session.handle(Event::Backspace), passed());
}

#[test]
fn enter_commits_the_preedit() {
    let mut session = hiragana();
    type_chars(&mut session, "AB");
    assert_eq!(
        session.handle(Event::Commit),
        outcome(OutcomeKind::Committed, true, "ＡＢ", "")
    );
    assert!(!session.is_composing());
}

#[test]
fn escape_cancels_the_preedit() {
    let mut session = hiragana();
    type_chars(&mut session, "AB");
    assert_eq!(
        session.handle(Event::Cancel),
        outcome(OutcomeKind::Canceled, true, "", "")
    );
    assert!(!session.is_composing());
}

#[test]
fn reserved_keys_keep_the_preedit_and_report_unsupported() {
    // 決定:Space と F6〜F10 は変換のキー、左右・Home・End・Delete は未確定文字の中を動く・編集するキー、
    // 上下・Tab・PageUp・PageDown は変換・候補の操作のキーとして取っておき、初版は「未対応」を返して
    // 未確定文字を残す(設計文書の 2.3)。
    for (event, feature) in RESERVED_KEYS {
        let mut session = hiragana();
        type_chars(&mut session, "A");
        assert_eq!(
            session.handle(event),
            outcome(OutcomeKind::Unsupported(feature), true, "", "Ａ"),
            "{event:?}"
        );
    }
}

#[test]
fn an_unused_key_commits_and_goes_to_the_application() {
    // 決定:Kieli が使わないキーは、確定してからアプリへ渡す(consumed が false で、commit が空でない)。
    // アプリでの順番と編集の結果は、第 2 段階で確かめる(設計文書の 2.3)。
    let mut session = hiragana();
    type_chars(&mut session, "A");
    assert_eq!(
        session.handle(Event::Other),
        outcome(OutcomeKind::Committed, false, "Ａ", "")
    );
}

#[test]
fn other_function_keys_are_treated_as_unused_keys() {
    // 決定:F1〜F5・F11・F12 と、それ以外のファンクションキーは、Kieli が使わないキーと同じに扱う
    // (設計文書の 2.3)。
    for event in UNUSED_FUNCTION_KEYS {
        let mut session = hiragana();
        type_chars(&mut session, "A");
        assert_eq!(
            session.handle(event),
            outcome(OutcomeKind::Committed, false, "Ａ", ""),
            "{event:?}"
        );
    }
}

#[test]
fn a_control_character_while_composing_is_treated_as_an_unused_key() {
    let mut session = hiragana();
    type_chars(&mut session, "A");
    assert_eq!(
        session.clone().handle(Event::Char('\t')),
        session.handle(Event::Other)
    );
}

#[test]
fn cancel_and_commit_are_available_without_a_key() {
    // 決定:取り消しと確定は別の操作として持ち、入力先を失ったときに接続層が選ぶ(設計文書の 3)。
    let mut session = hiragana();
    type_chars(&mut session, "AB");
    assert_eq!(session.commit(), "ＡＢ");
    assert_eq!(session.commit(), "");
    type_chars(&mut session, "C");
    session.cancel();
    assert!(!session.is_composing());
}

#[test]
fn the_cursor_is_at_the_end_of_the_preedit() {
    let mut session = hiragana();
    assert_eq!(session.preedit().cursor(), 0..0);
    type_chars(&mut session, "Aあ𠮷");
    let preedit = session.preedit();
    assert_eq!(preedit.text(), "Ａあ𠮷");
    assert_eq!(preedit.cursor(), 10..10);
}

#[test]
fn sessions_do_not_share_state() {
    let mut a = hiragana();
    let b = hiragana();
    type_chars(&mut a, "A");
    a.handle(Event::ToggleMode);
    a.set_sensitive(true);
    a.set_direct_field(true);
    assert_eq!(b.preedit().text(), "");
    assert_eq!(b.mode(), InputMode::Hiragana);
    assert!(!b.is_sensitive());
    assert!(!b.is_direct_field());
}

#[test]
fn the_sensitive_flag_is_kept_and_does_not_change_the_conversion() {
    // 決定:sensitive_data だけの欄は、ふつうに変換する(設計文書の 6)。
    let mut session = hiragana();
    session.set_sensitive(true);
    assert!(session.is_sensitive());
    type_chars(&mut session, "A");
    assert_eq!(session.preedit().text(), "Ａ");
}

// ---- 入力モード

#[test]
fn mode_keys_are_consumed_without_a_preedit() {
    let mut session = hiragana();
    let changed = outcome(OutcomeKind::ModeChanged, true, "", "");
    assert_eq!(session.handle(Event::ToggleMode), changed);
    assert_eq!(session.mode(), InputMode::Direct);
    assert_eq!(session.handle(Event::SetMode(InputMode::Hiragana)), changed);
    assert_eq!(session.mode(), InputMode::Hiragana);
    assert_eq!(session.handle(Event::SetMode(InputMode::Direct)), changed);
    assert_eq!(session.mode(), InputMode::Direct);
}

#[test]
fn a_mode_key_for_the_current_mode_is_consumed_and_changes_nothing() {
    // 決定:今と同じモードを指すモードのキーは、消費して何もしない(確定しない、状態を変えない)。
    // 未確定文字が無いときも同じ(設計文書の 2.3)。
    let mut composing = hiragana();
    type_chars(&mut composing, "A");
    for mut session in [composing, hiragana()] {
        let before = session.clone();
        let preedit = String::from(before.preedit().text());
        assert_eq!(
            session.handle(Event::SetMode(InputMode::Hiragana)),
            outcome(OutcomeKind::Ignored, true, "", &preedit)
        );
        assert_eq!(session, before);
    }
    let mut session = direct();
    assert_eq!(
        session.handle(Event::SetMode(InputMode::Direct)),
        outcome(OutcomeKind::Ignored, true, "", "")
    );
}

#[test]
fn the_mode_switch_commits_the_preedit_first() {
    // 決定:未確定文字があるときは、Enter と同じ処理で確定してから切り替える。キーはアプリへ渡さない
    // (設計文書の 2.3)。
    for event in [Event::ToggleMode, Event::SetMode(InputMode::Direct)] {
        let mut session = hiragana();
        type_chars(&mut session, "A");
        assert_eq!(
            session.handle(event),
            outcome(OutcomeKind::ModeChanged, true, "Ａ", ""),
            "{event:?}"
        );
        assert_eq!(session.mode(), InputMode::Direct);
    }
}

// ---- 直接入力の欄

#[test]
fn a_direct_field_passes_every_key_and_keeps_the_chosen_mode() {
    // 決定:password・pin・hidden_text の欄では、その欄の間だけ実際の動きを直接入力にする。
    // 利用者が選んだモードは上書きしない(設計文書の 6)。
    let mut session = hiragana();
    session.set_direct_field(true);
    assert_eq!(session.mode(), InputMode::Hiragana);
    assert_eq!(session.effective_mode(), InputMode::Direct);
    assert_eq!(session.handle(Event::Char('A')), passed());
    assert_eq!(session.handle(Event::Space), passed());
    assert!(!session.is_composing());

    session.set_direct_field(false);
    assert_eq!(session.effective_mode(), InputMode::Hiragana);
    assert_eq!(
        session.handle(Event::Char('A')),
        outcome(OutcomeKind::Updated, true, "", "Ａ")
    );
}

#[test]
fn entering_a_direct_field_does_not_commit_the_preedit() {
    // 決定:守る欄へ移るとき、前の未確定文字をそこへ確定しない(設計文書の 6)。
    let mut session = hiragana();
    type_chars(&mut session, "AB");
    session.set_direct_field(true);
    assert!(!session.is_composing());
    assert_eq!(session.handle(Event::Commit), passed());
}

#[test]
fn mode_keys_in_a_direct_field_change_only_the_chosen_mode() {
    // 決定:直接入力の欄の中のモードのキーは、消費して mode() だけを変える。effective_mode() は Direct の
    // まま(設計文書の 6)。
    let mut session = hiragana();
    session.set_direct_field(true);
    assert_eq!(
        session.handle(Event::ToggleMode),
        outcome(OutcomeKind::ModeChanged, true, "", "")
    );
    assert_eq!(session.mode(), InputMode::Direct);
    assert_eq!(session.effective_mode(), InputMode::Direct);
    session.handle(Event::ToggleMode);
    session.set_direct_field(false);
    assert_eq!(session.effective_mode(), InputMode::Hiragana);
}

// ---- 長さの上限

#[test]
fn the_default_limit_is_1024_bytes() {
    // 決定:上限は 1,024 バイト(暫定。設計文書の 5)。
    assert_eq!(Config::default().max_preedit_bytes, 1024);
    let mut session = hiragana();
    // 全角の英字は 3 バイトなので、341 文字(1,023 バイト)まで入る。
    for _ in 0..400 {
        session.handle(Event::Char('A'));
    }
    assert_eq!(session.preedit().text().len(), 1023);
}

#[test]
fn a_character_over_the_limit_is_refused_without_any_change() {
    // 決定:越える入力は受け付けず、「上限を越えた」を返し、その入力による変更は丸ごと取り消す。
    // 知らせに、入力した文字を入れない(設計文書の 5)。
    let mut session = limited(7);
    type_chars(&mut session, "AB");
    let before = session.clone();
    assert_eq!(
        session.handle(Event::Char('C')),
        outcome(OutcomeKind::OverLimit, true, "", "ＡＢ")
    );
    assert_eq!(session, before);
    // 1 バイトの文字も、全角にした後の長さで判定する(半角なら 7 バイトに収まる)。
    assert_eq!(
        session.handle(Event::Char('!')).kind,
        OutcomeKind::OverLimit
    );
}

#[test]
fn editing_keys_work_after_the_limit() {
    // 決定:上限に達した後も、Backspace・Enter・Esc・モードの切り替えは使える(設計文書の 5)。
    let full = || {
        let mut session = limited(6);
        type_chars(&mut session, "ABC");
        assert_eq!(session.preedit().text(), "ＡＢ");
        session
    };
    assert_eq!(full().handle(Event::Backspace).kind, OutcomeKind::Updated);
    assert_eq!(full().handle(Event::Commit).kind, OutcomeKind::Committed);
    assert_eq!(full().handle(Event::Cancel).kind, OutcomeKind::Canceled);
    assert_eq!(
        full().handle(Event::ToggleMode).kind,
        OutcomeKind::ModeChanged
    );
}

// ---- would_consume と handle が食い違わないこと

/// 並びの 1 つの手順。キーのイベントのほかに、接続層が呼ぶ操作も混ぜる。
#[derive(Debug, Clone, Copy)]
enum Step {
    Key(Event),
    DirectField(bool),
    Cancel,
    Commit,
}

/// キーの形を全部(ファンクションキーは、取っておく番号と取っておかない番号の代表を 1 つずつ)。
/// 文字は、ひらがなになる英字(a)、保留になる英字(k・n。促音・ん・未定義の綴りもできる)、かな、制御文字を入れる。
const ALL_KEYS: [Event; 25] = [
    Event::Char('a'),
    Event::Char('k'),
    Event::Char('n'),
    Event::Char('あ'),
    Event::Char('\t'),
    Event::Backspace,
    Event::Commit,
    Event::Cancel,
    Event::Space,
    Event::Left,
    Event::Right,
    Event::Home,
    Event::End,
    Event::Delete,
    Event::Up,
    Event::Down,
    Event::Tab,
    Event::PageUp,
    Event::PageDown,
    Event::Function(6),
    Event::Function(1),
    Event::ToggleMode,
    Event::SetMode(InputMode::Hiragana),
    Event::SetMode(InputMode::Direct),
    Event::Other,
];

/// 今は同じ動きをするキーの組と、その代表。食い違いのテストは、組のキーを代表だけで試す。
/// 組のキーが代表と同じ動きをすることは、`grouped_keys_behave_like_their_representative` が見張る。
///
/// 動きが分かれたら(たとえば Delete に本物の動きを入れたら)、この表からキーを外して、食い違いのテストへ戻す。
const KEY_GROUPS: [(Event, &[Event]); 3] = [
    (
        Event::Left,
        &[Event::Right, Event::Home, Event::End, Event::Delete],
    ),
    (
        Event::Up,
        &[Event::Down, Event::Tab, Event::PageUp, Event::PageDown],
    ),
    // 取っておかないファンクションキーと、制御文字の文字は、Kieli が使わないキーと同じに扱う。
    // F6 は、未確定文字が無いときの Space(U+3000 を確定する)と動きが違うので、Space にまとめない。
    (Event::Other, &[Event::Function(1), Event::Char('\t')]),
];

fn is_grouped(event: Event) -> bool {
    KEY_GROUPS
        .iter()
        .any(|(_, members)| members.contains(&event))
}

/// 接続層が呼ぶ操作の手順。
const OPERATIONS: [Step; 4] = [
    Step::DirectField(true),
    Step::DirectField(false),
    Step::Cancel,
    Step::Commit,
];

/// キーを全部使う手順(見張りのテストの、始めの状態を作るため)。
fn all_steps() -> Vec<Step> {
    ALL_KEYS
        .into_iter()
        .map(Step::Key)
        .chain(OPERATIONS)
        .collect()
}

/// 組のキーを代表だけにした手順(食い違いのテストのため)。
fn representative_steps() -> Vec<Step> {
    ALL_KEYS
        .into_iter()
        .filter(|&event| !is_grouped(event))
        .map(Step::Key)
        .chain(OPERATIONS)
        .collect()
}

/// 接続層が呼ぶ操作を行う。
fn apply_operation(session: &Session, step: Step) -> Session {
    let mut next = session.clone();
    match step {
        Step::Key(event) => {
            next.handle(event);
        }
        Step::DirectField(on) => next.set_direct_field(on),
        Step::Cancel => next.cancel(),
        Step::Commit => {
            next.commit();
        }
    }
    next
}

/// キーのイベントを 1 つ処理し、決まりが守られていることを確かめる。
fn check_key(session: &Session, event: Event, max_preedit_bytes: usize) -> Session {
    let before = session.clone();
    let expected = session.would_consume(&event);
    assert_eq!(*session, before, "would_consume が状態を変えた: {event:?}");

    let mut next = session.clone();
    let result = next.handle(event);
    assert_eq!(result.consumed, expected, "{before:?} に {event:?}");
    assert_eq!(result.preedit, next.preedit().text());

    let text = next.preedit().text();
    assert!(text.len() <= max_preedit_bytes);
    let cursor = next.preedit().cursor();
    assert!(text.is_char_boundary(cursor.start) && text.is_char_boundary(cursor.end));
    if next.is_composing() {
        assert_eq!(
            next.effective_mode(),
            InputMode::Hiragana,
            "直接入力で未確定文字がある"
        );
    }

    match result.kind {
        OutcomeKind::Passed
        | OutcomeKind::Ignored
        | OutcomeKind::Unsupported(_)
        | OutcomeKind::OverLimit => {
            assert!(result.commit.is_empty());
            assert_eq!(
                next, before,
                "{:?} なのに状態が変わった: {event:?}",
                result.kind
            );
        }
        _ => {}
    }
    assert_eq!(
        result.kind == OutcomeKind::Passed,
        !result.consumed && result.commit.is_empty()
    );
    next
}

/// 長さ `depth` までの手順の並びを全部試す。確かめたキーのイベントの数を返す。
/// `start` は、上限を `max_preedit_bytes` にして作ったセッション。
fn check_all_sequences(start: &Session, max_preedit_bytes: usize, depth: usize) -> usize {
    let steps = representative_steps();
    let mut checked = 0;
    let mut stack: Vec<(Session, usize)> = Vec::from([(start.clone(), 0)]);
    while let Some((session, len)) = stack.pop() {
        if len == depth {
            continue;
        }
        for &step in &steps {
            let next = match step {
                Step::Key(event) => {
                    checked += 1;
                    check_key(&session, event, max_preedit_bytes)
                }
                _ => apply_operation(&session, step),
            };
            stack.push((next, len + 1));
        }
    }
    checked
}

/// 長さ `depth` までの手順(キーを全部使う)でたどれる状態を、全部集める(同じ状態も重ねて数える)。
fn reachable_states(start: &Session, depth: usize) -> Vec<Session> {
    let steps = all_steps();
    let mut states = Vec::from([start.clone()]);
    let mut frontier = Vec::from([start.clone()]);
    for _ in 0..depth {
        let mut next_frontier = Vec::new();
        for session in &frontier {
            for &step in &steps {
                next_frontier.push(apply_operation(session, step));
            }
        }
        states.extend(next_frontier.iter().cloned());
        frontier = next_frontier;
    }
    states
}

/// 見張りのテストの、始めの状態を作る手順の長さ。
const GROUP_CHECK_DEPTH: usize = 3;

/// 上限は、全角の英字 2 文字(6 バイト)が入り、3 文字目で越える値にする。
const TEST_MAX_PREEDIT_BYTES: usize = 7;

fn starts() -> [Session; 2] {
    [InputMode::Hiragana, InputMode::Direct].map(|mode| {
        Session::new(Config {
            mode,
            max_preedit_bytes: TEST_MAX_PREEDIT_BYTES,
        })
    })
}

#[test]
fn grouped_keys_behave_like_their_representative() {
    // 食い違いのテストが組のキーを代表だけで試してよいことを見張る。動きが分かれたら、
    // KEY_GROUPS からキーを外して、食い違いのテストへ戻す。
    let mut compared = 0;
    for start in starts() {
        let states = reachable_states(&start, GROUP_CHECK_DEPTH);
        // 手順が 29 種類なので、1 + 29 + 29^2 + 29^3 = 25,260 の状態(同じ状態も重ねて数える)。
        assert_eq!(states.len(), 25_260);
        for state in &states {
            for (representative, members) in KEY_GROUPS {
                let mut expected_state = state.clone();
                let expected = expected_state.handle(representative);
                for &member in members {
                    assert_eq!(
                        state.would_consume(&member),
                        state.would_consume(&representative),
                        "{state:?} で {member:?} と {representative:?}"
                    );
                    let mut actual_state = state.clone();
                    let actual = actual_state.handle(member);
                    assert_eq!(
                        actual, expected,
                        "{state:?} で {member:?} と {representative:?}"
                    );
                    // 未確定文字・カーソル・mode()・effective_mode()・直接入力の欄を含め、状態の全体を比べる。
                    assert_eq!(
                        actual_state, expected_state,
                        "{state:?} で {member:?} と {representative:?}"
                    );
                    assert_eq!(actual_state.preedit(), expected_state.preedit());
                    assert_eq!(
                        actual_state.effective_mode(),
                        expected_state.effective_mode()
                    );
                    compared += 1;
                }
            }
        }
    }
    // 組のキーは 10 個なので、25,260 × 10 × 2 = 505,200。
    assert_eq!(compared, 505_200);
}

/// 並びの長さ。
///
/// 長さ 5 あれば、上限まで文字を入れ(2 手)、越える文字を入れ(1 手)、その後にキーを入れる(1 手)並びに、
/// 直接入力の欄の出入りを 1 つ挟む形まで試せる。
///
/// 手順が 27 種類のとき(2026-10-26)は、cargo test の全体で 8.96〜10.40 秒かかったので、今は同じ動きの
/// キーを代表 1 つにまとめている(`KEY_GROUPS`)。
const DEPTH: usize = 5;

#[test]
fn would_consume_and_handle_agree_on_every_short_sequence() {
    for start in starts() {
        // 並びの長さが 1〜5 で、手順が 19 種類(うちキーのイベントが 15 種類)。
        // 長さ k の並びの最後の手順がキーのイベントになる数は 15 × 19^(k-1) なので、
        // 15 × (1 + 19 + 19^2 + 19^3 + 19^4) = 2,063,415。
        assert_eq!(
            check_all_sequences(&start, TEST_MAX_PREEDIT_BYTES, DEPTH),
            2_063_415
        );
    }
}
