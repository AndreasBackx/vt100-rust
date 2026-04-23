mod helpers;

#[test]
fn modes() {
    helpers::fixture("modes");
}

#[test]
fn alternate_buffer() {
    helpers::fixture("alternate_buffer");
}

#[test]
fn kitty_keyboard_push_pop() {
    let mut parser = vt100::Parser::default();

    assert_eq!(parser.screen().kitty_keyboard_mode(), 0);
    assert!(parser.screen().kitty_keyboard_stack().is_empty());

    // CSI > 1 u — push flags=1
    parser.process(b"\x1b[>1u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 1);
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1]);

    // Push flags=3
    parser.process(b"\x1b[>3u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 3);
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1, 3]);

    // CSI < u — pop one
    parser.process(b"\x1b[<u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 1);
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1]);

    // Pop last
    parser.process(b"\x1b[<u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 0);
    assert!(parser.screen().kitty_keyboard_stack().is_empty());

    helpers::assert_reproduces_state(b"\x1b[>1u");
    helpers::assert_reproduces_state(b"\x1b[>1u\x1b[>3u");
}

#[test]
fn kitty_keyboard_pop_count() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>1u\x1b[>2u\x1b[>3u");
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1, 2, 3]);

    // CSI < 2 u — pop 2
    parser.process(b"\x1b[<2u");
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1]);
}

#[test]
fn kitty_keyboard_pop_underflow() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>1u");
    // Pop more than stack depth
    parser.process(b"\x1b[<5u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 0);
    assert!(parser.screen().kitty_keyboard_stack().is_empty());

    // Pop on empty stack — no panic
    parser.process(b"\x1b[<u");
    assert!(parser.screen().kitty_keyboard_stack().is_empty());
}

#[test]
fn kitty_keyboard_set_disposition() {
    let mut parser = vt100::Parser::default();

    // CSI = 3 ; 1 u — set to 3 (disposition 1 = set)
    parser.process(b"\x1b[=3;1u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 3);

    // CSI = 4 ; 2 u — OR with 4 (disposition 2 = or)
    parser.process(b"\x1b[=4;2u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 7);

    // CSI = 1 ; 3 u — AND NOT 1 (disposition 3 = not)
    parser.process(b"\x1b[=1;3u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 6);

    helpers::assert_reproduces_state(b"\x1b[=3;1u");
}

#[test]
fn modify_other_keys() {
    let mut parser = vt100::Parser::default();

    assert_eq!(parser.screen().modify_other_keys(), 0);

    // CSI > 4 ; 2 m — enable level 2
    parser.process(b"\x1b[>4;2m");
    assert_eq!(parser.screen().modify_other_keys(), 2);

    // CSI > 4 ; 1 m — level 1
    parser.process(b"\x1b[>4;1m");
    assert_eq!(parser.screen().modify_other_keys(), 1);

    // CSI > 4 ; 0 m — disable
    parser.process(b"\x1b[>4;0m");
    assert_eq!(parser.screen().modify_other_keys(), 0);

    helpers::assert_reproduces_state(b"\x1b[>4;2m");
}

#[test]
fn modify_other_keys_level_3() {
    let mut parser = vt100::Parser::default();

    // CSI > 4 ; 3 m — enable level 3 (unmodified keys as escape sequences)
    parser.process(b"\x1b[>4;3m");
    assert_eq!(parser.screen().modify_other_keys(), 3);

    helpers::assert_reproduces_state(b"\x1b[>4;3m");
}

#[test]
fn ris_resets_keyboard_modes() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>1u");
    parser.process(b"\x1b[>4;2m");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 1);
    assert_eq!(parser.screen().modify_other_keys(), 2);

    // ESC c — full reset
    parser.process(b"\x1bc");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 0);
    assert!(parser.screen().kitty_keyboard_stack().is_empty());
    assert_eq!(parser.screen().modify_other_keys(), 0);
}

#[test]
fn state_formatted_roundtrip() {
    let mut parser = vt100::Parser::default();
    parser.process(b"\x1b[>1u\x1b[>4;2m");

    let state = parser.screen().state_formatted();
    let mut parser2 = vt100::Parser::default();
    parser2.process(&state);

    assert_eq!(parser2.screen().kitty_keyboard_mode(), 1);
    assert_eq!(parser2.screen().kitty_keyboard_stack(), &[1]);
    assert_eq!(parser2.screen().modify_other_keys(), 2);

    assert!(helpers::compare_screens(
        parser2.screen(),
        parser.screen()
    ));
}

#[test]
fn state_diff_keyboard_modes() {
    let mut parser = vt100::Parser::default();
    let prev = parser.screen().clone();

    parser.process(b"\x1b[>1u\x1b[>4;2m");

    let diff = parser.screen().state_diff(&prev);
    let mut parser2 = vt100::Parser::default();
    parser2.process(&prev.state_formatted());
    parser2.process(&diff);

    assert!(helpers::compare_screens(
        parser2.screen(),
        parser.screen()
    ));
}

#[test]
fn kitty_keyboard_push_zero_flags() {
    let mut parser = vt100::Parser::default();

    // CSI > u — push with default flags (0)
    parser.process(b"\x1b[>u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 0);
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[0]);

    helpers::assert_reproduces_state(b"\x1b[>u");
}

#[test]
fn kitty_keyboard_multi_level_roundtrip() {
    let mut parser = vt100::Parser::default();
    parser.process(b"\x1b[>1u\x1b[>3u\x1b[>7u");
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1, 3, 7]);

    helpers::assert_reproduces_state(b"\x1b[>1u\x1b[>3u\x1b[>7u");
}

#[test]
fn kitty_keyboard_diff_stack_to_stack() {
    let mut parser = vt100::Parser::default();
    parser.process(b"\x1b[>1u\x1b[>3u");
    let prev = parser.screen().clone();

    // Pop one, push a different value
    parser.process(b"\x1b[<u\x1b[>5u");
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1, 5]);

    let diff = parser.screen().state_diff(&prev);
    let mut parser2 = vt100::Parser::default();
    parser2.process(&prev.state_formatted());
    parser2.process(&diff);

    assert!(helpers::compare_screens(
        parser2.screen(),
        parser.screen()
    ));
}

#[test]
fn combined_modes_roundtrip() {
    helpers::assert_reproduces_state(
        b"\x1b[?2004h\x1b[>1u\x1b[>4;2m\x1b[?1000h",
    );
}

// --- Kitty keyboard protocol spec compliance ---

#[test]
fn kitty_keyboard_stack_overflow_evicts_oldest() {
    let mut parser = vt100::Parser::default();

    // Push 16 entries (fills the stack)
    for i in 1..=16u16 {
        parser.process(format!("\x1b[>{i}u").as_bytes());
    }
    assert_eq!(parser.screen().kitty_keyboard_stack().len(), 16);
    assert_eq!(parser.screen().kitty_keyboard_stack()[0], 1);
    assert_eq!(parser.screen().kitty_keyboard_mode(), 16);

    // Push 17th — oldest (1) should be evicted
    parser.process(b"\x1b[>99u");
    assert_eq!(parser.screen().kitty_keyboard_stack().len(), 16);
    assert_eq!(parser.screen().kitty_keyboard_stack()[0], 2);
    assert_eq!(parser.screen().kitty_keyboard_mode(), 99);
}

#[test]
fn kitty_keyboard_set_default_mode() {
    let mut parser = vt100::Parser::default();

    // CSI = 5 u — mode parameter omitted, defaults to 1 (set)
    parser.process(b"\x1b[=5u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 5);
}

#[test]
fn kitty_keyboard_set_after_push() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>1u");
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[1]);

    // CSI = 3 ; 2 u — OR 3 into the top entry (1 | 3 = 3)
    parser.process(b"\x1b[=3;2u");
    // Stack depth should NOT change — set modifies in-place
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[3]);
}

#[test]
fn kitty_keyboard_individual_flag_bits() {
    for flag in [1u16, 2, 4, 8, 16] {
        let seq = format!("\x1b[>{flag}u");
        helpers::assert_reproduces_state(seq.as_bytes());
    }
}

#[test]
fn kitty_keyboard_all_flags_combined() {
    // 0x1F = 31 = all 5 flag bits set
    let mut parser = vt100::Parser::default();
    parser.process(b"\x1b[>31u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 31);
    helpers::assert_reproduces_state(b"\x1b[>31u");
}

#[test]
fn kitty_keyboard_push_pop_restores_previous() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>1u");
    parser.process(b"\x1b[>3u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 3);

    parser.process(b"\x1b[<u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 1);
}

#[test]
fn kitty_keyboard_pop_exact_stack_depth() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>1u\x1b[>2u\x1b[>3u");
    assert_eq!(parser.screen().kitty_keyboard_stack().len(), 3);

    // Pop exactly 3
    parser.process(b"\x1b[<3u");
    assert!(parser.screen().kitty_keyboard_stack().is_empty());
    assert_eq!(parser.screen().kitty_keyboard_mode(), 0);
}

#[test]
fn kitty_keyboard_set_on_empty_stack() {
    let mut parser = vt100::Parser::default();

    // CSI = 5 ; 1 u on fresh parser — should create an entry
    parser.process(b"\x1b[=5;1u");
    assert_eq!(parser.screen().kitty_keyboard_stack(), &[5]);
}

#[test]
fn kitty_keyboard_set_invalid_mode() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>7u");

    // Mode 4 — invalid (only 1-3 defined), should not change top
    parser.process(b"\x1b[=99;4u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 7);

    // Mode 5 — invalid, should not change top
    parser.process(b"\x1b[=99;5u");
    assert_eq!(parser.screen().kitty_keyboard_mode(), 7);
}

// --- xterm XTMODKEYS spec compliance ---

#[test]
fn xtmodkeys_disable_via_csi_n() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>4;2m");
    assert_eq!(parser.screen().modify_other_keys(), 2);

    // CSI > 4 n — disable modifyOtherKeys
    parser.process(b"\x1b[>4n");
    assert_eq!(parser.screen().modify_other_keys(), 0);
}

#[test]
fn xtmodkeys_disable_on_fresh_parser() {
    let mut parser = vt100::Parser::default();

    // CSI > 4 n on a fresh parser — should remain 0
    parser.process(b"\x1b[>4n");
    assert_eq!(parser.screen().modify_other_keys(), 0);
}

#[test]
fn xtmodkeys_reset_all_no_params() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>4;2m");
    assert_eq!(parser.screen().modify_other_keys(), 2);

    // CSI > m — no params, resets all resources
    parser.process(b"\x1b[>m");
    assert_eq!(parser.screen().modify_other_keys(), 0);
}

#[test]
fn xtmodkeys_reset_pv_omitted() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>4;2m");
    assert_eq!(parser.screen().modify_other_keys(), 2);

    // CSI > 4 m — Pv omitted, resets to initial value
    parser.process(b"\x1b[>4m");
    assert_eq!(parser.screen().modify_other_keys(), 0);
}

#[test]
fn xtmodkeys_non_4_param_ignored() {
    let mut parser = vt100::Parser::default();

    parser.process(b"\x1b[>4;2m");
    assert_eq!(parser.screen().modify_other_keys(), 2);

    // CSI > 1 ; 2 m — modifyCursorKeys, not modifyOtherKeys
    parser.process(b"\x1b[>1;2m");
    // Our modifyOtherKeys should be unchanged
    assert_eq!(parser.screen().modify_other_keys(), 2);
}
