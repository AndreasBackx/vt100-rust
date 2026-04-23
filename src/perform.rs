const BASE64: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
const CLIPBOARD_SELECTOR: &[u8] = b"cpqs01234567";

pub struct WrappedScreen<CB: crate::callbacks::Callbacks = ()> {
    pub screen: crate::screen::Screen,
    pub callbacks: CB,
}

impl WrappedScreen<()> {
    pub fn new(rows: u16, cols: u16, scrollback_len: usize) -> Self {
        Self::new_with_callbacks(rows, cols, scrollback_len, ())
    }
}

impl<CB: crate::callbacks::Callbacks> WrappedScreen<CB> {
    pub fn new_with_callbacks(
        rows: u16,
        cols: u16,
        scrollback_len: usize,
        callbacks: CB,
    ) -> Self {
        Self {
            screen: crate::screen::Screen::new(
                crate::grid::Size { rows, cols },
                scrollback_len,
            ),
            callbacks,
        }
    }
}

impl<CB: crate::callbacks::Callbacks> vte::Perform for WrappedScreen<CB> {
    fn print(&mut self, c: char) {
        if c == '\u{fffd}' || ('\u{80}'..'\u{a0}').contains(&c) {
            self.callbacks.unhandled_char(&mut self.screen, c);
        } else {
            self.screen.text(c);
        }
    }

    fn execute(&mut self, b: u8) {
        match b {
            7 => self.callbacks.audible_bell(&mut self.screen),
            8 => self.screen.bs(),
            9 => self.screen.tab(),
            10 => self.screen.lf(),
            11 => self.screen.vt(),
            12 => self.screen.ff(),
            13 => self.screen.cr(),
            // we don't implement shift in/out alternate character sets, but
            // it shouldn't count as an "error"
            14 | 15 => {}
            _ => self.callbacks.unhandled_control(&mut self.screen, b),
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, b: u8) {
        if let Some(i) = intermediates.first() {
            self.callbacks.unhandled_escape(
                &mut self.screen,
                Some(*i),
                intermediates.get(1).copied(),
                b,
            );
        } else {
            match b {
                b'7' => self.screen.decsc(),
                b'8' => self.screen.decrc(),
                b'=' => self.screen.deckpam(),
                b'>' => self.screen.deckpnm(),
                b'M' => self.screen.ri(),
                b'c' => self.screen.ris(),
                b'g' => self.callbacks.visual_bell(&mut self.screen),
                _ => {
                    self.callbacks.unhandled_escape(
                        &mut self.screen,
                        None,
                        None,
                        b,
                    );
                }
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        c: char,
    ) {
        let mut unhandled = |screen: &mut crate::screen::Screen| {
            self.callbacks.unhandled_csi(
                screen,
                intermediates.first().copied(),
                intermediates.get(1).copied(),
                &params.iter().collect::<Vec<_>>(),
                c,
            );
        };
        match intermediates.first() {
            None => match c {
                '@' => self.screen.ich(canonicalize_params_1(params, 1)),
                'A' => self.screen.cuu(canonicalize_params_1(params, 1)),
                'B' => self.screen.cud(canonicalize_params_1(params, 1)),
                'C' => self.screen.cuf(canonicalize_params_1(params, 1)),
                'D' => self.screen.cub(canonicalize_params_1(params, 1)),
                'E' => self.screen.cnl(canonicalize_params_1(params, 1)),
                'F' => self.screen.cpl(canonicalize_params_1(params, 1)),
                'G' => self.screen.cha(canonicalize_params_1(params, 1)),
                'H' => self.screen.cup(canonicalize_params_2(params, 1, 1)),
                'J' => self
                    .screen
                    .ed(canonicalize_params_1(params, 0), unhandled),
                'K' => self
                    .screen
                    .el(canonicalize_params_1(params, 0), unhandled),
                'L' => self.screen.il(canonicalize_params_1(params, 1)),
                'M' => self.screen.dl(canonicalize_params_1(params, 1)),
                'P' => self.screen.dch(canonicalize_params_1(params, 1)),
                'S' => self.screen.su(canonicalize_params_1(params, 1)),
                'T' => self.screen.sd(canonicalize_params_1(params, 1)),
                'X' => self.screen.ech(canonicalize_params_1(params, 1)),
                'd' => self.screen.vpa(canonicalize_params_1(params, 1)),
                'm' => self.screen.sgr(params, unhandled),
                'r' => self.screen.decstbm(canonicalize_params_decstbm(
                    params,
                    self.screen.grid().size(),
                )),
                't' => {
                    let mut params_iter = params.iter();
                    let op =
                        params_iter.next().and_then(|x| x.first().copied());
                    if op == Some(8) {
                        let (screen_rows, screen_cols) = self.screen.size();
                        let rows =
                            params_iter.next().map_or(screen_rows, |x| {
                                *x.first().unwrap_or(&screen_rows)
                            });
                        let cols =
                            params_iter.next().map_or(screen_cols, |x| {
                                *x.first().unwrap_or(&screen_cols)
                            });
                        self.callbacks.resize(&mut self.screen, (rows, cols));
                    } else {
                        self.callbacks.unhandled_csi(
                            &mut self.screen,
                            None,
                            None,
                            &params.iter().collect::<Vec<_>>(),
                            c,
                        );
                    }
                }
                _ => {
                    self.callbacks.unhandled_csi(
                        &mut self.screen,
                        None,
                        None,
                        &params.iter().collect::<Vec<_>>(),
                        c,
                    );
                }
            },
            Some(b'?') => match c {
                'J' => self
                    .screen
                    .decsed(canonicalize_params_1(params, 0), unhandled),
                'K' => self
                    .screen
                    .decsel(canonicalize_params_1(params, 0), unhandled),
                'h' => self.screen.decset(params, unhandled),
                'l' => self.screen.decrst(params, unhandled),
                _ => {
                    self.callbacks.unhandled_csi(
                        &mut self.screen,
                        Some(b'?'),
                        intermediates.get(1).copied(),
                        &params.iter().collect::<Vec<_>>(),
                        c,
                    );
                }
            },
            // Kitty keyboard protocol and xterm modifyOtherKeys.
            //
            // The `>` byte (0x3E) is a "private marker" in the CSI
            // sequence grammar (bytes 0x3C–0x3F), so vte places it in
            // the `intermediates` slice — same treatment as `?` which
            // is used for DECSET/DECRST above.
            //
            // Two different protocols share this intermediate:
            //
            // 1. Kitty keyboard protocol (final byte `u`):
            //    Applications send `CSI > flags u` to push a keyboard
            //    mode requesting enhanced key reporting — e.g. so that
            //    Shift+Enter (CSI 13;2u) can be distinguished from
            //    plain Enter (\r). See:
            //    https://sw.kovidgoyal.net/kitty/keyboard-protocol/
            //
            // 2. xterm modifyOtherKeys (final byte `m`):
            //    `CSI > 4 ; n m` sets the modifyOtherKeys level.
            //    Only first-parameter 4 is handled; other values of
            //    `CSI > n m` are unknown private sequences and fall
            //    through to the unhandled callback. See:
            //    https://invisible-island.net/xterm/manpage/xterm.html#VT100-Widget-Resources:modifyOtherKeys
            Some(b'>') => match c {
                'u' => {
                    let flags = canonicalize_params_1(params, 0);
                    self.screen.kitty_keyboard_push(flags);
                }
                'm' => {
                    // CSI > Pp ; Pv m (XTMODKEYS) — set key modifier
                    // resource. Pp identifies which resource (we only
                    // track Pp=4, modifyOtherKeys). If no params are
                    // given (Pp defaults to 0), all resources reset to
                    // initial values. If Pv is omitted, the addressed
                    // resource resets to its initial value.
                    // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
                    let mut iter = params.iter();
                    let p = iter
                        .next()
                        .map_or(0, |x| *x.first().unwrap_or(&0));
                    if p == 0 {
                        // No params or Pp=0: reset all resources.
                        // We only track modifyOtherKeys.
                        self.screen.set_modify_other_keys(0);
                    } else if p == 4 {
                        let level = iter
                            .next()
                            .map_or(0, |x| *x.first().unwrap_or(&0));
                        self.screen.set_modify_other_keys(
                            u8::try_from(level).unwrap_or(u8::MAX),
                        );
                    } else {
                        unhandled(&mut self.screen);
                    }
                }
                'n' => {
                    // CSI > Ps n — disable key modifier option
                    // (XTMODKEYS). Sets the resource to -1 (fully
                    // disabled). We track this as 0 since we use u8
                    // and the semantic effect is the same: no
                    // modifier encoding.
                    let p = canonicalize_params_1(params, 0);
                    if p == 4 {
                        self.screen.set_modify_other_keys(0);
                    }
                }
                _ => unhandled(&mut self.screen),
            },
            // CSI < [n] u — pop n entries from the kitty keyboard mode
            // stack (default 1). Sent by applications on exit to
            // restore the previous keyboard mode.
            Some(b'<') => match c {
                'u' => {
                    let n = canonicalize_params_1(params, 1);
                    self.screen.kitty_keyboard_pop(n);
                }
                _ => unhandled(&mut self.screen),
            },
            // CSI = flags ; mode u — modify the top entry on the kitty
            // keyboard mode stack. The `mode` parameter is a
            // disposition: 1=set, 2=bitwise-or, 3=bitwise-and-not.
            Some(b'=') => match c {
                'u' => {
                    let (flags, mode) =
                        canonicalize_params_2(params, 0, 1);
                    self.screen.kitty_keyboard_set(flags, mode);
                }
                _ => unhandled(&mut self.screen),
            },
            Some(i) => {
                self.callbacks.unhandled_csi(
                    &mut self.screen,
                    Some(*i),
                    intermediates.get(1).copied(),
                    &params.iter().collect::<Vec<_>>(),
                    c,
                );
            }
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bel_terminated: bool) {
        match params {
            [b"0", s] => {
                self.callbacks.set_window_icon_name(&mut self.screen, s);
                self.callbacks.set_window_title(&mut self.screen, s);
            }
            [b"1", s] => {
                self.callbacks.set_window_icon_name(&mut self.screen, s);
            }
            [b"2", s] => {
                self.callbacks.set_window_title(&mut self.screen, s);
            }
            [b"52", ty, data] => {
                match (
                    ty.iter().all(|c| CLIPBOARD_SELECTOR.contains(c)),
                    *data,
                ) {
                    (true, b"?") => {
                        self.callbacks
                            .paste_from_clipboard(&mut self.screen, ty);
                    }
                    (true, data)
                        if data.iter().all(|c| BASE64.contains(c)) =>
                    {
                        self.callbacks.copy_to_clipboard(
                            &mut self.screen,
                            ty,
                            data,
                        );
                    }
                    _ => {
                        self.callbacks
                            .unhandled_osc(&mut self.screen, params);
                    }
                }
            }
            _ => {
                self.callbacks.unhandled_osc(&mut self.screen, params);
            }
        }
    }
}

fn canonicalize_params_1(params: &vte::Params, default: u16) -> u16 {
    let first = params.iter().next().map_or(0, |x| *x.first().unwrap_or(&0));
    if first == 0 {
        default
    } else {
        first
    }
}

fn canonicalize_params_2(
    params: &vte::Params,
    default1: u16,
    default2: u16,
) -> (u16, u16) {
    let mut iter = params.iter();
    let first = iter.next().map_or(0, |x| *x.first().unwrap_or(&0));
    let first = if first == 0 { default1 } else { first };

    let second = iter.next().map_or(0, |x| *x.first().unwrap_or(&0));
    let second = if second == 0 { default2 } else { second };

    (first, second)
}

fn canonicalize_params_decstbm(
    params: &vte::Params,
    size: crate::grid::Size,
) -> (u16, u16) {
    let mut iter = params.iter();
    let top = iter.next().map_or(0, |x| *x.first().unwrap_or(&0));
    let top = if top == 0 { 1 } else { top };

    let bottom = iter.next().map_or(0, |x| *x.first().unwrap_or(&0));
    let bottom = if bottom == 0 { size.rows } else { bottom };

    (top, bottom)
}
