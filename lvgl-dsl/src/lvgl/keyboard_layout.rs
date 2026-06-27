use core::ffi::c_char;

// ---------------------------------------------------------------------------
// KeyMapEntry — Sync-safe C string pointer
// ---------------------------------------------------------------------------

/// A thin wrapper around `*const c_char` that is safe to place in `static`
/// context.
///
/// LVGL key-map arrays are composed of pointers to `'static` C string
/// literals, which are by definition immutable and never deallocated.  The
/// `Sync` bound on shared statics is satisfied here because:
/// - every pointer stored in a `KeyMap` static must come from a `c"..."`
///   literal (enforced by the crate's `pub static` keymaps), and
/// - `'static` C string literals have constant addresses that never change.
///
/// Construct entries with [`KeyMapEntry::new`]:
/// ```rust
/// KeyMapEntry::new(c"A")
/// ```
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct KeyMapEntry(*const c_char);

impl KeyMapEntry {
    /// Creates a `KeyMapEntry` from a `'static` C string literal.
    ///
    /// The only safe way to construct a `KeyMapEntry` for use in a `static`
    /// [`KeyMap`].  Accepts `c"..."` literals directly.
    ///
    /// ```rust
    /// use jetbeep_lvgl_dsl::lvgl::prelude::*;
    /// static MY_MAP: &KeyMap = &[
    ///     KeyMapEntry::new(c"A"), KeyMapEntry::new(c"B"),
    ///     KeyMapEntry::new(c"\n"),
    ///     KeyMapEntry::new(c""),  // terminator
    /// ];
    /// ```
    pub const fn new(s: &'static core::ffi::CStr) -> Self {
        KeyMapEntry(s.as_ptr())
    }
}

// SAFETY: All `KeyMapEntry` values in this crate are derived from `c"..."`
// literals, which are `'static` and immutable.  The raw pointer itself is
// never written through, so aliasing across threads is safe.
unsafe impl Sync for KeyMapEntry {}
unsafe impl Send for KeyMapEntry {}

// ---------------------------------------------------------------------------
// LvKeyboardMode — raw LVGL enum passthrough
// ---------------------------------------------------------------------------

/// Direct LVGL keyboard mode passthrough.
///
/// Maps to the `lv_keyboard_mode_t` C enum values used by
/// `lv_keyboard_set_mode` and `lv_keyboard_set_map`.
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LvKeyboardMode {
    /// Lower-case QWERTY (LVGL built-in).
    TextLower = 0,
    /// Upper-case QWERTY (LVGL built-in).
    TextUpper = 1,
    /// Special character layout (LVGL built-in).
    Special = 2,
    /// Numeric pad (LVGL built-in).
    Number = 3,
    /// User-defined layout slot 1.
    User1 = 4,
    /// User-defined layout slot 2.
    User2 = 5,
    /// User-defined layout slot 3.
    User3 = 6,
    /// User-defined layout slot 4.
    User4 = 7,
}

// ---------------------------------------------------------------------------
// KeyMap — flat slice type alias
// ---------------------------------------------------------------------------

/// A flat, `'\0'`-terminated array of [`KeyMapEntry`] describing one keyboard layout.
///
/// Each element is a [`KeyMapEntry`] pointing to a key label literal.
/// A `KeyMapEntry(c"\n".as_ptr())` element starts a new row; a
/// `KeyMapEntry(c"".as_ptr())` element terminates the entire map — this is
/// the LVGL `lv_keyboard_set_map` convention.
///
/// Use `c"..."` literals (Rust 2024 edition) to construct entries.  See
/// [`KEYMAP_QWERTY_EN`] and [`KEYMAP_NUMPAD`] for examples.
pub type KeyMap = [KeyMapEntry];

// ---------------------------------------------------------------------------
// CtrlMap — parallel button-width and flag array
// ---------------------------------------------------------------------------

/// Parallel companion to [`KeyMap`]: one `u16` entry per actual key button
/// (excluding `\n` row-separators and the `""` terminator).
///
/// Each entry encodes two fields packed into a single `u16` matching LVGL's
/// `lv_buttonmatrix_ctrl_t` type:
/// - **Bits 0–2** — relative button width (1–7 units).  Widths are
///   distributed proportionally within each row so the row fills the full
///   keyboard width.  A value of `0` is treated by LVGL as width `1`.
/// - **Bit 4** — [`CTRL_HIDDEN`] (`0x0010`): render the button as an
///   invisible spacer that never sends a key event.  Use this to create
///   left/right indent on shorter rows without changing the row's logical
///   key count.
/// - **Bits 5+** — additional `lv_buttonmatrix_ctrl_t` flags (NO_REPEAT,
///   DISABLED, CHECKABLE, …).
///
/// The array length **must** equal the number of non-`\n`, non-`""` entries
/// in the paired [`KeyMap`]; a mismatched length causes undefined LVGL
/// behaviour.
///
/// See [`CTRLMAP_QWERTY_EN`] for a worked example.
pub type CtrlMap = [u32];

/// `LV_BUTTONMATRIX_CTRL_HIDDEN` flag — button rendered as invisible spacer.
///
/// Bit 4 (`0x0010`) matches `LV_BUTTONMATRIX_CTRL_HIDDEN` in LVGL v9.x.
/// Combine with a width constant: `CTRL_HIDDEN | CTRL_W1`.
pub const CTRL_HIDDEN: u32 = 0x0010;

/// Width-1 key (1 relative unit) — standard letter keys.
pub const CTRL_W1: u32 = 1;
/// Width-2 key (2 relative units) — action keys such as `ABC` and `Del`.
pub const CTRL_W2: u32 = 2;
/// Width-3 key (3 relative units) — bottom-row keys such as `Ok` and `#@!`.
pub const CTRL_W3: u32 = 3;
/// Width-4 key (4 relative units) — wider bottom-row keys (Back, Continue).
pub const CTRL_W4: u32 = 4;
/// Width-5 key (5 relative units).
pub const CTRL_W5: u32 = 5;
/// Width-6 key (6 relative units) — extended space bar.
pub const CTRL_W6: u32 = 6;
/// Width-7 key — comfortable space-bar width.
pub const CTRL_SPACE_W: u32 = 7;
/// `LV_BUTTONMATRIX_CTRL_CHECKED` — marks action keys (ABC, Del, Back, etc.)
/// so LVGL applies `LV_STATE_CHECKED` during drawing, enabling separate
/// styling via `SELECTOR_KEY_ACTION`.
pub const CTRL_CHECKED: u32 = 0x0100;
/// `LV_BUTTONMATRIX_CTRL_DISABLED` — disables a button (no events, renders
/// in `LV_STATE_DISABLED`).  Matches LVGL v9.x.
pub const CTRL_DISABLED: u32 = 0x0040;
/// Hidden 1-unit spacer — creates left/right row indent without a visible key.
pub const CTRL_SPACER: u32 = CTRL_HIDDEN | CTRL_W1;

// ---------------------------------------------------------------------------
// KeyboardLocale — language shorthand
// ---------------------------------------------------------------------------

/// Language/locale shorthand for selecting a keyboard layout.
///
/// `Numeric` uses LVGL's built-in Number mode. Text locales install custom
/// maps; the keyboard widget assigns those maps to LVGL's real mode range
/// (`TEXT_LOWER`/`TEXT_UPPER` for `EnUs`, `USER_1..USER_4` for other locales)
/// at runtime.
///
/// This enum is `#[non_exhaustive]` so downstream crates can extend it via
/// their own `impl From<MyLocale> for KeyboardLayout` without requiring
/// changes here.
///
/// # Default
///
/// Use [`LocaleSwitcher`] to track and cycle through locales at runtime
/// without heap allocation.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KeyboardLocale {
    /// English (US) — standard QWERTY lower-case.
    /// Installed using custom QWERTY English lower/upper maps into [`LvKeyboardMode::TextLower`].
    EnUs,
    /// Numeric-only input.  Uses LVGL built-in `Number`.
    Numeric,
    /// German QWERTZ — `y`↔`z` transposed, `ü`/`ö`/`ä`/`ß` included.
    /// Installed into a runtime `USER_1..USER_4` slot.
    De,
    /// French AZERTY — `a`/`z` row 1, `q`/`s` row 2, `é` row 3.
    /// Installed into a runtime `USER_1..USER_4` slot.
    Fr,
    /// Italian QWERTY — standard rows with `à`/`è`/`ì`/`ò`/`ù` accent row.
    /// Installed into a runtime `USER_1..USER_4` slot.
    It,
    /// Swiss French QWERTZ — `z`↔`y` transposed, `é`/`è`/`à`/`ç` accent row.
    /// Installed into a runtime `USER_1..USER_4` slot.
    FrCh,
    /// Ukrainian ЙЦУКЕН — standard Cyrillic layout.
    /// Installed into a runtime `USER_1..USER_4` slot.
    Ua,
}

impl KeyboardLocale {
    /// Returns a safe fallback LVGL mode integer for this locale.
    ///
    /// The keyboard widget keeps the authoritative runtime locale→user-slot
    /// assignment. This fallback is used only by layout-level tests and by
    /// defensive paths where no keyboard state is available.
    #[inline]
    pub(crate) fn native_mode(self) -> u32 {
        match self {
            KeyboardLocale::EnUs => LvKeyboardMode::TextLower as u32,
            KeyboardLocale::Numeric => LvKeyboardMode::Number as u32,
            KeyboardLocale::De => LvKeyboardMode::User2 as u32,
            KeyboardLocale::Fr => LvKeyboardMode::User3 as u32,
            KeyboardLocale::It => LvKeyboardMode::User4 as u32,
            KeyboardLocale::FrCh => LvKeyboardMode::User1 as u32,
            KeyboardLocale::Ua => LvKeyboardMode::User1 as u32,
        }
    }

    /// Returns the static key map and companion ctrl map for language locales,
    /// or `None` for locales that use LVGL built-in maps (`Numeric`).
    ///
    /// The returned `&'static CtrlMap` has exactly as many entries as the
    /// non-separator, non-terminator buttons in the paired `KeyMap`.
    #[inline]
    pub(crate) fn maps(self) -> Option<(&'static KeyMap, &'static CtrlMap)> {
        match self {
            KeyboardLocale::EnUs => Some((KEYMAP_QWERTY_EN_LC, CTRLMAP_QWERTY_EN_LC)),
            KeyboardLocale::De => Some((KEYMAP_QWERTY_DE_LC, CTRLMAP_QWERTY_DE_LC)),
            KeyboardLocale::Fr => Some((KEYMAP_QWERTY_FR_LC, CTRLMAP_QWERTY_FR_LC)),
            KeyboardLocale::It => Some((KEYMAP_QWERTY_IT_LC, CTRLMAP_QWERTY_IT_LC)),
            KeyboardLocale::FrCh => Some((KEYMAP_QWERTY_FRCH_LC, CTRLMAP_QWERTY_FRCH_LC)),
            KeyboardLocale::Ua => Some((KEYMAP_UA_LC, CTRLMAP_UA_LC)),
            _ => None,
        }
    }

    /// Returns lowercase and uppercase map pairs for locales that use custom
    /// layouts, or `None` for `Numeric`.
    ///
    /// Returns `(lc_map, lc_ctrl, uc_map, uc_ctrl)`.
    #[inline]
    pub(crate) fn map_pair(
        self,
    ) -> Option<(
        &'static KeyMap,
        &'static CtrlMap,
        &'static KeyMap,
        &'static CtrlMap,
    )> {
        match self {
            KeyboardLocale::EnUs => Some((
                KEYMAP_QWERTY_EN_LC,
                CTRLMAP_QWERTY_EN_LC,
                KEYMAP_QWERTY_EN_UC,
                CTRLMAP_QWERTY_EN_UC,
            )),
            KeyboardLocale::De => Some((
                KEYMAP_QWERTY_DE_LC,
                CTRLMAP_QWERTY_DE_LC,
                KEYMAP_QWERTY_DE_UC,
                CTRLMAP_QWERTY_DE_UC,
            )),
            KeyboardLocale::Fr => Some((
                KEYMAP_QWERTY_FR_LC,
                CTRLMAP_QWERTY_FR_LC,
                KEYMAP_QWERTY_FR_UC,
                CTRLMAP_QWERTY_FR_UC,
            )),
            KeyboardLocale::It => Some((
                KEYMAP_QWERTY_IT_LC,
                CTRLMAP_QWERTY_IT_LC,
                KEYMAP_QWERTY_IT_UC,
                CTRLMAP_QWERTY_IT_UC,
            )),
            KeyboardLocale::FrCh => Some((
                KEYMAP_QWERTY_FRCH_LC,
                CTRLMAP_QWERTY_FRCH_LC,
                KEYMAP_QWERTY_FRCH_UC,
                CTRLMAP_QWERTY_FRCH_UC,
            )),
            KeyboardLocale::Ua => Some((KEYMAP_UA_LC, CTRLMAP_UA_LC, KEYMAP_UA_UC, CTRLMAP_UA_UC)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// KeyboardLayout — DSL layout selector
// ---------------------------------------------------------------------------

/// DSL-friendly alias selecting a keyboard layout.
///
/// | Variant | LVGL slot | Map installed? |
/// |---|---|---|
/// | `Qwerty` | `TextLower` (built-in) | no |
/// | `QwertyUpper` | `TextUpper` (built-in) | no |
/// | `NumberPad` | `Number` (built-in) | no |
/// | `SpecialChars` | `Special` (built-in) | no |
/// | `Locale(EnUs)` | `TextLower` | yes — `KEYMAP_QWERTY_EN_*` |
/// | `Locale(Numeric)` | `Number` (built-in) | no |
/// | `Locale(De/Fr/It/FrCh/Ua)` | runtime `User1..User4` slot | yes — locale keymap pair |
/// | `Custom(map)` | `User1` | yes — caller-supplied |
#[derive(Copy, Clone)]
pub enum KeyboardLayout {
    /// Standard lower-case QWERTY layout (LVGL built-in `TextLower`).
    Qwerty,
    /// Upper-case QWERTY layout (LVGL built-in `TextUpper`).
    QwertyUpper,
    /// Numeric keypad layout (LVGL built-in `Number`).
    NumberPad,
    /// Special-character layout (LVGL built-in `Special`).
    SpecialChars,
    /// A language locale — selects the layout and LVGL slot automatically.
    ///
    /// Prefer this over raw [`Qwerty`](Self::Qwerty) /
    /// [`NumberPad`](Self::NumberPad) when targeting a specific language, as
    /// language locales install their own custom map into a dedicated slot.
    Locale(KeyboardLocale),
    /// Fully custom layout loaded into `User1` slot.
    ///
    /// The map must be a `&'static KeyMap` — a statically allocated flat
    /// array of [`KeyMapEntry`] terminated by `KeyMapEntry(c"".as_ptr())`.
    Custom(&'static KeyMap),
}

impl KeyboardLayout {
    /// Returns the `lv_keyboard_mode_t` integer for `lv_keyboard_set_mode`.
    pub(crate) fn lv_mode(&self) -> u32 {
        match self {
            KeyboardLayout::Qwerty => LvKeyboardMode::TextLower as u32,
            KeyboardLayout::QwertyUpper => LvKeyboardMode::TextUpper as u32,
            KeyboardLayout::NumberPad => LvKeyboardMode::Number as u32,
            KeyboardLayout::SpecialChars => LvKeyboardMode::Special as u32,
            KeyboardLayout::Locale(loc) => loc.native_mode(),
            KeyboardLayout::Custom(_) => LvKeyboardMode::User1 as u32,
        }
    }

    /// Returns the key map and optional ctrl map to install, if any.
    ///
    /// Returns `None` for built-in LVGL layouts (no map installation needed).
    /// For [`Locale`](Self::Locale) variants the ctrl map is always `Some`;
    /// for [`Custom`](Self::Custom) the ctrl map is `None` and LVGL uses
    /// equal-width buttons.
    pub(crate) fn maps(&self) -> Option<(&'static KeyMap, Option<&'static CtrlMap>)> {
        match self {
            KeyboardLayout::Locale(loc) => loc.maps().map(|(km, ctrl)| (km, Some(ctrl))),
            KeyboardLayout::Custom(map) => Some((map, None)),
            _ => None,
        }
    }
}

impl From<KeyboardLocale> for KeyboardLayout {
    fn from(locale: KeyboardLocale) -> KeyboardLayout {
        KeyboardLayout::Locale(locale)
    }
}

// ---------------------------------------------------------------------------
// LocaleSwitcher<N> — zero-alloc locale cycler for embedded use
// ---------------------------------------------------------------------------

/// A zero-allocation, `no_std`-safe cursor that cycles through a fixed set of
/// [`KeyboardLocale`] values.
///
/// `LocaleSwitcher` is const-constructible and suitable for `static` storage
/// in embedded firmware.  The locale list is stored inline in a
/// `[KeyboardLocale; N]` array — no heap allocation is ever made.
///
/// # Example
///
/// ```rust,ignore
/// use jetbeep_lvgl_dsl::lvgl::prelude::*;
///
/// static LOCALES: LocaleSwitcher<3> = LocaleSwitcher::new([
///     KeyboardLocale::EnUs,
///     KeyboardLocale::De,
///     KeyboardLocale::Fr,
/// ]);
///
/// // At runtime — cycle on button press:
/// fn on_lang_button(kb: &Keyboard, sw: &mut LocaleSwitcher<3>) {
///     let next = sw.next();
///     kb.locale(next);
/// }
/// ```
///
/// # Panics
///
/// `new()` panics at compile time if `N == 0`.
#[derive(Copy, Clone, Debug)]
pub struct LocaleSwitcher<const N: usize> {
    locales: [KeyboardLocale; N],
    index: usize,
}

impl<const N: usize> LocaleSwitcher<N> {
    /// Creates a new `LocaleSwitcher` with the given locale array.
    ///
    /// # Panics
    ///
    /// Panics (at compile time when used in a `const` context) if `N == 0`.
    pub const fn new(locales: [KeyboardLocale; N]) -> Self {
        assert!(N > 0, "LocaleSwitcher must contain at least one locale");
        LocaleSwitcher { locales, index: 0 }
    }

    /// Returns the currently active locale without advancing the cursor.
    #[inline]
    pub fn current(&self) -> KeyboardLocale {
        self.locales[self.index]
    }

    /// Advances the cursor to the next locale, wrapping after the last, and
    /// returns the new active locale.
    #[inline]
    pub fn next(&mut self) -> KeyboardLocale {
        self.index = (self.index + 1) % N;
        self.locales[self.index]
    }

    /// Sets the active locale to `locale` if it is present in the list.
    ///
    /// Returns `true` and updates the cursor on success.  Returns `false`
    /// without changing the cursor if `locale` is not in the list.
    #[inline]
    pub fn set(&mut self, locale: KeyboardLocale) -> bool {
        for (i, &l) in self.locales.iter().enumerate() {
            if l == locale {
                self.index = i;
                return true;
            }
        }
        false
    }

    /// Returns the current cursor position (zero-based index into the list).
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the number of locales in this switcher (`N`).
    #[inline]
    pub const fn len(&self) -> usize {
        N
    }

    /// Returns `false` — a `LocaleSwitcher<N>` with `N > 0` is never empty.
    ///
    /// (Satisfies `clippy::len_without_is_empty`.)
    #[inline]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Convenience macro for constructing KeyMapEntry arrays
// ---------------------------------------------------------------------------

macro_rules! k {
    ($s:expr) => {
        KeyMapEntry($s.as_ptr())
    };
}

// ---------------------------------------------------------------------------
// Named action-key label constants
// ---------------------------------------------------------------------------

/// Key label that LVGL interprets as the ABC (lower→upper case toggle) action.
pub const KEY_ABC: &core::ffi::CStr = c"ABC";
/// Key label that LVGL interprets as the abc (upper→lower case toggle) action.
pub const KEY_ABC_LOWER: &core::ffi::CStr = c"abc";
/// Key label that LVGL interprets as a Backspace / Delete action.
pub const KEY_DEL: &core::ffi::CStr = c"Del";
/// Key label that LVGL interprets as the Ok / Accept action (`LV_EVENT_READY`).
pub const KEY_OK: &core::ffi::CStr = c"Ok";
/// Key label that LVGL interprets as the special-character mode toggle.
pub const KEY_SPECIAL: &core::ffi::CStr = c"#@!";
/// Key label for the space bar.
pub const KEY_SPACE: &core::ffi::CStr = c" ";

// ── New action key labels (custom handler required) ───────────────────────

/// Backspace key displayed as `⌫` symbol — handled by the custom event handler.
pub const KEY_BACKSPACE: &core::ffi::CStr = c"⌫";
/// Back navigation key — fires `LV_EVENT_CANCEL` via the custom event handler.
pub const KEY_BACK: &core::ffi::CStr = c"Back";
/// Continue / accept key — fires `LV_EVENT_READY` via the custom event handler.
pub const KEY_CONTINUE: &core::ffi::CStr = c"Continue";
/// Language / locale switching key (globe emoji) — fires the `on_lang` callback.
pub const KEY_LANG: &core::ffi::CStr = c"\xF0\x9F\x8C\x90";
/// English locale label — fires the `on_lang` callback.
pub const KEY_LANG_EN: &core::ffi::CStr = c"EN";
/// German locale label — fires the `on_lang` callback.
pub const KEY_LANG_DE: &core::ffi::CStr = c"DE";
/// French locale label — fires the `on_lang` callback.
pub const KEY_LANG_FR: &core::ffi::CStr = c"FR";
/// Italian locale label — fires the `on_lang` callback.
pub const KEY_LANG_IT: &core::ffi::CStr = c"IT";
/// Swiss French locale label — fires the `on_lang` callback.
pub const KEY_LANG_CH: &core::ffi::CStr = c"CH";
/// Ukrainian locale label — fires the `on_lang` callback.
pub const KEY_LANG_UA: &core::ffi::CStr = c"UA";
/// Numeric / special mode toggle — label shown as "123".
pub const KEY_123: &core::ffi::CStr = c"123";

// Convenience aliases so key-map arrays can mix `k!(c"q")` literals and named
// action keys uniformly.
macro_rules! ka {
    ($c:expr) => {
        KeyMapEntry($c.as_ptr())
    };
}

// ---------------------------------------------------------------------------
// Predefined static key maps
// ---------------------------------------------------------------------------

/// Standard English (US) QWERTY layout map.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  y  u  i  o  p
/// Row 2  (9):  a  s  d  f  g  h  j  k  l
/// Row 3  (9): ABC  z  x  c  v  b  n  m  Del
/// Row 4  (3): #@!  [space]  Ok
/// ```
///
/// Pair with [`CTRLMAP_QWERTY_EN`] for proportional action-key widths.
/// The natural stagger (row 2 slightly indented) is created automatically by
/// LVGL distributing the 9 row-2 keys across the same width as the 10 row-1
/// keys — no hidden spacer buttons are needed.
///
/// `c"\n"` separates rows; `c""` terminates the map.
pub static KEYMAP_QWERTY_EN: &KeyMap = &[
    // Row 1 — 10 keys
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"y"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2 — 9 keys (LVGL auto-staggers vs row 1)
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"\n"),
    // Row 3 — 9 keys (ABC/Del are CTRL_W2 in ctrl map)
    ka!(KEY_ABC),
    k!(c"z"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    k!(c"m"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — 3 keys
    ka!(KEY_SPECIAL),
    ka!(KEY_SPACE),
    ka!(KEY_OK),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_EN`].
///
/// One entry per actual key (31 total = 10 + 9 + 9 + 3), matching the
/// non-separator, non-terminator count in [`KEYMAP_QWERTY_EN`].
///
/// Width legend: [`CTRL_W1`] = letter, [`CTRL_W2`] = action (ABC/Del),
/// [`CTRL_W3`] = bottom-row, [`CTRL_SPACE_W`] = space bar.
pub static CTRLMAP_QWERTY_EN: &CtrlMap = &[
    // Row 1 — 10 letter keys × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2 — 9 letter keys × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — ABC(2) + 7 letters(1) + Del(2)  [action keys checked]
    CTRL_W2 | CTRL_CHECKED,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2 | CTRL_CHECKED,
    // Row 4 — #@!(3) + space(7) + Ok(3)  [action keys checked]
    CTRL_W3 | CTRL_CHECKED,
    CTRL_SPACE_W,
    CTRL_W3 | CTRL_CHECKED,
];

/// German QWERTZ layout map.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  z  u  i  o  p
/// Row 2  (9):  a  s  d  f  g  h  j  k  l
/// Row 3  (9): ABC  y  x  c  v  b  n  m  Del
/// Row 4  (6): ä  ö  ü  ß  [space]  Ok
/// ```
///
/// The classic QWERTZ transposition (`y`↔`z`) is in rows 1 and 3.
/// Umlauts (ä, ö, ü) and ß are placed on a dedicated accent row 4 so rows
/// 1–3 keep the same proportions as the EN layout (10 / 9 / 9 keys).
///
/// > **Font note**: `ä`, `ö`, `ü`, `ß` require a font compiled with Latin
/// > Extended-A support (`CONFIG_LV_FONT_MONTSERRAT_XX` with Unicode range
/// > U+00C0–U+017E).  Without it, LVGL renders them as □.
///
/// Installed into [`LvKeyboardMode::User2`].
pub static KEYMAP_QWERTY_DE: &KeyMap = &[
    // Row 1 — 10 keys (QWERTZ: z in position 6)
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"z"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2 — 9 keys
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"\n"),
    // Row 3 — y in QWERTZ position
    ka!(KEY_ABC),
    k!(c"y"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    k!(c"m"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — German accent keys + space + Ok
    k!(c"ä"),
    k!(c"ö"),
    k!(c"ü"),
    k!(c"ß"),
    ka!(KEY_SPACE),
    ka!(KEY_OK),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_DE`].
///
/// 34 entries = 10 + 9 + 9 + 6.
pub static CTRLMAP_QWERTY_DE: &CtrlMap = &[
    // Row 1 — 10 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2 — 9 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — ABC(2) + 7 letters(1) + Del(2)  [action keys checked]
    CTRL_W2 | CTRL_CHECKED,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2 | CTRL_CHECKED,
    // Row 4 — 4 accent keys(1) + space(5) + Ok(3)  [action keys checked]
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W5,
    CTRL_W3 | CTRL_CHECKED,
];

/// French AZERTY layout map.
///
/// ```text
/// Row 1 (10): a  z  e  r  t  y  u  i  o  p
/// Row 2 (10): q  s  d  f  g  h  j  k  l  m
/// Row 3  (8): ABC  w  x  c  v  b  n  Del
/// Row 4  (6): é  è  à  ç  [space]  Ok
/// ```
///
/// Classic AZERTY transpositions: `a`↔`q` (rows 1/2), `z` in row 1 position
/// 2, `w` relegated to row 3, `m` at end of row 2.  Common French accents
/// (é, è, à, ç) are on the dedicated row 4.
///
/// > **Font note**: accented characters require Latin Extended-A font support.
///
/// Installed into [`LvKeyboardMode::User3`].
pub static KEYMAP_QWERTY_FR: &KeyMap = &[
    // Row 1 — AZERTY top row (10 keys)
    k!(c"a"),
    k!(c"z"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"y"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2 — AZERTY home row (10 keys; m at end)
    k!(c"q"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"m"),
    k!(c"\n"),
    // Row 3 — 8 keys (w here per AZERTY; del is wider)
    ka!(KEY_ABC),
    k!(c"w"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — French accents + space + Ok
    k!(c"é"),
    k!(c"è"),
    k!(c"à"),
    k!(c"ç"),
    ka!(KEY_SPACE),
    ka!(KEY_OK),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_FR`].
///
/// 34 entries = 10 + 10 + 8 + 6.
pub static CTRLMAP_QWERTY_FR: &CtrlMap = &[
    // Row 1 — 10 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2 — 10 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — ABC(2) + 6 letters(1) + Del(2)  [action keys checked]
    CTRL_W2 | CTRL_CHECKED,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2 | CTRL_CHECKED,
    // Row 4 — 4 accent keys(1) + space(5) + Ok(3)  [action keys checked]
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W5,
    CTRL_W3 | CTRL_CHECKED,
];

/// Italian QWERTY layout map.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  y  u  i  o  p
/// Row 2  (9):  a  s  d  f  g  h  j  k  l
/// Row 3  (9): ABC  z  x  c  v  b  n  m  Del
/// Row 4  (4): à  è  [space]  Ok
/// ```
///
/// Rows 1–3 are identical to the EN layout.  Row 4 provides the two most
/// common Italian accented vowels (à, è) alongside the space bar and Ok key.
///
/// > **Font note**: `à` and `è` require Latin Extended-A font support.
///
/// Installed into [`LvKeyboardMode::User4`].
pub static KEYMAP_QWERTY_IT: &KeyMap = &[
    // Row 1 — 10 keys
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"y"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2 — 9 keys
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"\n"),
    // Row 3 — 9 keys
    ka!(KEY_ABC),
    k!(c"z"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    k!(c"m"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — Italian accents + space + Ok
    k!(c"à"),
    k!(c"è"),
    ka!(KEY_SPACE),
    ka!(KEY_OK),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_IT`].
///
/// 32 entries = 10 + 9 + 9 + 4.
pub static CTRLMAP_QWERTY_IT: &CtrlMap = &[
    // Row 1 — 10 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2 — 9 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — ABC(2) + 7 letters(1) + Del(2)  [action keys checked]
    CTRL_W2 | CTRL_CHECKED,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2 | CTRL_CHECKED,
    // Row 4 — 2 accent keys(1) + space(7) + Ok(3)  [action keys checked]
    CTRL_W1,
    CTRL_W1,
    CTRL_SPACE_W,
    CTRL_W3 | CTRL_CHECKED,
];

/// Numeric keypad layout map (1–9, Del, 0, Ok).
///
/// Three digit rows plus a function row.
/// Row separator: `KeyMapEntry(c"\n".as_ptr())`.
/// Terminator:    `KeyMapEntry(c"".as_ptr())`.
pub static KEYMAP_NUMPAD: &KeyMap = &[
    k!(c"1"),
    k!(c"2"),
    k!(c"3"),
    k!(c"\n"),
    k!(c"4"),
    k!(c"5"),
    k!(c"6"),
    k!(c"\n"),
    k!(c"7"),
    k!(c"8"),
    k!(c"9"),
    k!(c"\n"),
    k!(c"Del"),
    k!(c"0"),
    k!(c"Ok"),
    k!(c""),
];

// ── Special characters (numbers + symbols) ────────────────────────────────

/// Special-characters layout — digits + punctuation, design-aligned.
///
/// Replaces LVGL's built-in Special map (which uses Font-Awesome glyphs
/// `⌫`, `🌐`, `←`, `→`, `✓` for the action row) with plain-text labels
/// (`Del`, `abc`, `Back`, `Continue`) that don't require the symbol font.
/// Without this map, action keys render as tofu (missing glyph) when the
/// keyboard's text font lacks the `LV_SYMBOL_*` icon range.
///
/// Installed into [`LvKeyboardMode::Special`] so the `123` key on the
/// alphabetic layouts switches into it.
///
/// ```text
/// Row 1 (11): 1  2  3  4  5  6  7  8  9  0  Del
/// Row 2 (12): abc  +  &  /  *  =  %  !  ?  #  <  >
/// Row 3 (12): \  @  $  (  )  {  }  [  ]  ;  "  '
/// Row 4  (3): Back  [space]  Continue
/// ```
pub static KEYMAP_SPECIAL: &KeyMap = &[
    // Row 1 — digits + Del
    k!(c"1"),
    k!(c"2"),
    k!(c"3"),
    k!(c"4"),
    k!(c"5"),
    k!(c"6"),
    k!(c"7"),
    k!(c"8"),
    k!(c"9"),
    k!(c"0"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 2 — abc toggle + symbols
    ka!(KEY_ABC_LOWER),
    k!(c"+"),
    k!(c"&"),
    k!(c"/"),
    k!(c"*"),
    k!(c"="),
    k!(c"%"),
    k!(c"!"),
    k!(c"?"),
    k!(c"#"),
    k!(c"<"),
    k!(c">"),
    k!(c"\n"),
    // Row 3 — more symbols
    k!(c"\\"),
    k!(c"@"),
    k!(c"$"),
    k!(c"("),
    k!(c")"),
    k!(c"{"),
    k!(c"}"),
    k!(c"["),
    k!(c"]"),
    k!(c";"),
    k!(c"\""),
    k!(c"'"),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_SPECIAL`].
/// 38 entries = 11 + 12 + 12 + 3.
pub static CTRLMAP_SPECIAL: &CtrlMap = &[
    // Row 1 — 10 digits(1) + Del(2)
    // Only the Continue CTA keeps CTRL_CHECKED (orange fill); every other
    // action key renders as a normal gray-border key.
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2,
    // Row 2 — abc(2) + 11 symbols(1)
    CTRL_W2,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — 12 symbols(1)
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 4 — Back(4) + space(7) + Continue(4)
    CTRL_W4,
    CTRL_SPACE_W,
    CTRL_W4 | CTRL_CHECKED,
];

// ===========================================================================
// New design-aligned layout maps — lowercase / uppercase pairs
//
// Target layout (from design mockup):
//   Row 1 (10): letter keys
//   Row 2  (9): letter keys
//   Row 3  (9): ABC/abc + 7 letters + Del
//   Row 4  (5): Back  EN  123  [space]  Continue
//
// For De/Fr/It, an additional accent row sits between row 3 and the nav row.
// ===========================================================================

// ── English (US) lowercase ────────────────────────────────────────────────

/// English (US) QWERTY layout — **lowercase** variant.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  y  u  i  o  p
/// Row 2 (10): a  s  d  f  g  h  j  k  l  '
/// Row 3  (9): ABC  z  x  c  v  b  n  m  Del
/// Row 4  (5): Back  EN  123  [space]  Continue
/// ```
pub static KEYMAP_QWERTY_EN_LC: &KeyMap = &[
    // Row 1
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"y"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"'"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC),
    k!(c"z"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    k!(c"m"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_EN),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_EN_LC`].
/// 34 entries = 10 + 10 + 9 + 5.
pub static CTRLMAP_QWERTY_EN_LC: &CtrlMap = &[
    // Row 1 — 10 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2 — 10 × width 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — ABC(2) + 7 letters(1) + Del(2)
    // Only the Continue CTA keeps CTRL_CHECKED (orange fill); every other
    // action key renders as a normal gray-border key.
    CTRL_W2,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2,
    // Row 4 — Back(3) + EN(1) + 123(1) + space(6) + Continue(3)
    CTRL_W3,
    CTRL_W1,
    CTRL_W1,
    CTRL_W6,
    CTRL_W3 | CTRL_CHECKED,
];

// ── English (US) uppercase ────────────────────────────────────────────────

/// English (US) QWERTY layout — **uppercase** variant.
///
/// ```text
/// Row 1 (10): Q  W  E  R  T  Y  U  I  O  P
/// Row 2 (10): A  S  D  F  G  H  J  K  L  '
/// Row 3  (9): abc  Z  X  C  V  B  N  M  Del
/// Row 4  (5): Back  EN  123  [space]  Continue
/// ```
pub static KEYMAP_QWERTY_EN_UC: &KeyMap = &[
    // Row 1
    k!(c"Q"),
    k!(c"W"),
    k!(c"E"),
    k!(c"R"),
    k!(c"T"),
    k!(c"Y"),
    k!(c"U"),
    k!(c"I"),
    k!(c"O"),
    k!(c"P"),
    k!(c"\n"),
    // Row 2
    k!(c"A"),
    k!(c"S"),
    k!(c"D"),
    k!(c"F"),
    k!(c"G"),
    k!(c"H"),
    k!(c"J"),
    k!(c"K"),
    k!(c"L"),
    k!(c"'"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC_LOWER),
    k!(c"Z"),
    k!(c"X"),
    k!(c"C"),
    k!(c"V"),
    k!(c"B"),
    k!(c"N"),
    k!(c"M"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_EN),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_EN_UC`] — same widths as lowercase.
pub static CTRLMAP_QWERTY_EN_UC: &CtrlMap = CTRLMAP_QWERTY_EN_LC;

// ── German QWERTZ lowercase ──────────────────────────────────────────────

/// German QWERTZ layout — **lowercase** variant.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  z  u  i  o  p
/// Row 2 (10): a  s  d  f  g  h  j  k  l  '
/// Row 3  (9): ABC  y  x  c  v  b  n  m  Del
/// Row 4  (5): Back  DE  123  [space]  Continue
/// ```
pub static KEYMAP_QWERTY_DE_LC: &KeyMap = &[
    // Row 1 (QWERTZ)
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"z"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"'"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC),
    k!(c"y"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    k!(c"m"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_DE),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_DE_LC`].
/// 34 entries = 10 + 10 + 9 + 5.
pub static CTRLMAP_QWERTY_DE_LC: &CtrlMap = CTRLMAP_QWERTY_EN_LC;

// ── German QWERTZ uppercase ──────────────────────────────────────────────

/// German QWERTZ layout — **uppercase** variant.
pub static KEYMAP_QWERTY_DE_UC: &KeyMap = &[
    // Row 1
    k!(c"Q"),
    k!(c"W"),
    k!(c"E"),
    k!(c"R"),
    k!(c"T"),
    k!(c"Z"),
    k!(c"U"),
    k!(c"I"),
    k!(c"O"),
    k!(c"P"),
    k!(c"\n"),
    // Row 2
    k!(c"A"),
    k!(c"S"),
    k!(c"D"),
    k!(c"F"),
    k!(c"G"),
    k!(c"H"),
    k!(c"J"),
    k!(c"K"),
    k!(c"L"),
    k!(c"'"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC_LOWER),
    k!(c"Y"),
    k!(c"X"),
    k!(c"C"),
    k!(c"V"),
    k!(c"B"),
    k!(c"N"),
    k!(c"M"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_DE),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_DE_UC`] — same widths as lowercase.
pub static CTRLMAP_QWERTY_DE_UC: &CtrlMap = CTRLMAP_QWERTY_DE_LC;

// ── French AZERTY lowercase ──────────────────────────────────────────────

/// French AZERTY layout — **lowercase** variant.
///
/// ```text
/// Row 1 (10): a  z  e  r  t  y  u  i  o  p
/// Row 2 (10): q  s  d  f  g  h  j  k  l  m
/// Row 3  (8): ABC  w  x  c  v  b  n  Del
/// Row 4  (5): Back  FR  123  [space]  Continue
/// ```
pub static KEYMAP_QWERTY_FR_LC: &KeyMap = &[
    // Row 1 — AZERTY
    k!(c"a"),
    k!(c"z"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"y"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2 — AZERTY home row
    k!(c"q"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"m"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC),
    k!(c"w"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_FR),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_FR_LC`].
/// 33 entries = 10 + 10 + 8 + 5.
pub static CTRLMAP_QWERTY_FR_LC: &CtrlMap = &[
    // Row 1
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3
    // Only the Continue CTA keeps CTRL_CHECKED (orange fill); every other
    // action key renders as a normal gray-border key.
    CTRL_W2,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2,
    // Row 4 — navigation
    CTRL_W3,
    CTRL_W1,
    CTRL_W1,
    CTRL_W6,
    CTRL_W3 | CTRL_CHECKED,
];

// ── French AZERTY uppercase ──────────────────────────────────────────────

/// French AZERTY layout — **uppercase** variant.
pub static KEYMAP_QWERTY_FR_UC: &KeyMap = &[
    // Row 1
    k!(c"A"),
    k!(c"Z"),
    k!(c"E"),
    k!(c"R"),
    k!(c"T"),
    k!(c"Y"),
    k!(c"U"),
    k!(c"I"),
    k!(c"O"),
    k!(c"P"),
    k!(c"\n"),
    // Row 2
    k!(c"Q"),
    k!(c"S"),
    k!(c"D"),
    k!(c"F"),
    k!(c"G"),
    k!(c"H"),
    k!(c"J"),
    k!(c"K"),
    k!(c"L"),
    k!(c"M"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC_LOWER),
    k!(c"W"),
    k!(c"X"),
    k!(c"C"),
    k!(c"V"),
    k!(c"B"),
    k!(c"N"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_FR),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_FR_UC`] — same widths as lowercase.
pub static CTRLMAP_QWERTY_FR_UC: &CtrlMap = CTRLMAP_QWERTY_FR_LC;

// ── Italian QWERTY lowercase ─────────────────────────────────────────────

/// Italian QWERTY layout — **lowercase** variant.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  y  u  i  o  p
/// Row 2 (10): a  s  d  f  g  h  j  k  l  '
/// Row 3  (9): ABC  z  x  c  v  b  n  m  Del
/// Row 4  (5): Back  IT  123  [space]  Continue
/// ```
pub static KEYMAP_QWERTY_IT_LC: &KeyMap = &[
    // Row 1
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"y"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"'"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC),
    k!(c"z"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    k!(c"m"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_IT),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_IT_LC`].
/// 34 entries = 10 + 10 + 9 + 5.
pub static CTRLMAP_QWERTY_IT_LC: &CtrlMap = CTRLMAP_QWERTY_EN_LC;

// ── Italian QWERTY uppercase ─────────────────────────────────────────────

/// Italian QWERTY layout — **uppercase** variant.
pub static KEYMAP_QWERTY_IT_UC: &KeyMap = &[
    // Row 1
    k!(c"Q"),
    k!(c"W"),
    k!(c"E"),
    k!(c"R"),
    k!(c"T"),
    k!(c"Y"),
    k!(c"U"),
    k!(c"I"),
    k!(c"O"),
    k!(c"P"),
    k!(c"\n"),
    // Row 2
    k!(c"A"),
    k!(c"S"),
    k!(c"D"),
    k!(c"F"),
    k!(c"G"),
    k!(c"H"),
    k!(c"J"),
    k!(c"K"),
    k!(c"L"),
    k!(c"'"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC_LOWER),
    k!(c"Z"),
    k!(c"X"),
    k!(c"C"),
    k!(c"V"),
    k!(c"B"),
    k!(c"N"),
    k!(c"M"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_IT),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_IT_UC`] — same widths as lowercase.
pub static CTRLMAP_QWERTY_IT_UC: &CtrlMap = CTRLMAP_QWERTY_IT_LC;

// ── Swiss French QWERTZ lowercase ────────────────────────────────────────

/// Swiss French QWERTZ layout — **lowercase** variant.
///
/// ```text
/// Row 1 (10): q  w  e  r  t  z  u  i  o  p
/// Row 2 (10): a  s  d  f  g  h  j  k  l  m
/// Row 3  (8): ABC  y  x  c  v  b  n  Del
/// Row 4  (5): Back  CH  123  [space]  Continue
/// ```
///
/// Classic QWERTZ transposition: `z`↔`y`.  Home row has `m` at end (like
/// French AZERTY).
///
/// Installed into a runtime `USER_1..USER_4` slot.
pub static KEYMAP_QWERTY_FRCH_LC: &KeyMap = &[
    // Row 1 — QWERTZ
    k!(c"q"),
    k!(c"w"),
    k!(c"e"),
    k!(c"r"),
    k!(c"t"),
    k!(c"z"),
    k!(c"u"),
    k!(c"i"),
    k!(c"o"),
    k!(c"p"),
    k!(c"\n"),
    // Row 2 — home row
    k!(c"a"),
    k!(c"s"),
    k!(c"d"),
    k!(c"f"),
    k!(c"g"),
    k!(c"h"),
    k!(c"j"),
    k!(c"k"),
    k!(c"l"),
    k!(c"m"),
    k!(c"\n"),
    // Row 3 — y in QWERTZ position
    ka!(KEY_ABC),
    k!(c"y"),
    k!(c"x"),
    k!(c"c"),
    k!(c"v"),
    k!(c"b"),
    k!(c"n"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_CH),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_FRCH_LC`].
/// 33 entries = 10 + 10 + 8 + 5.
pub static CTRLMAP_QWERTY_FRCH_LC: &CtrlMap = CTRLMAP_QWERTY_FR_LC;

// ── Swiss French QWERTZ uppercase ────────────────────────────────────────

/// Swiss French QWERTZ layout — **uppercase** variant.
pub static KEYMAP_QWERTY_FRCH_UC: &KeyMap = &[
    // Row 1
    k!(c"Q"),
    k!(c"W"),
    k!(c"E"),
    k!(c"R"),
    k!(c"T"),
    k!(c"Z"),
    k!(c"U"),
    k!(c"I"),
    k!(c"O"),
    k!(c"P"),
    k!(c"\n"),
    // Row 2
    k!(c"A"),
    k!(c"S"),
    k!(c"D"),
    k!(c"F"),
    k!(c"G"),
    k!(c"H"),
    k!(c"J"),
    k!(c"K"),
    k!(c"L"),
    k!(c"M"),
    k!(c"\n"),
    // Row 3
    ka!(KEY_ABC_LOWER),
    k!(c"Y"),
    k!(c"X"),
    k!(c"C"),
    k!(c"V"),
    k!(c"B"),
    k!(c"N"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_CH),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_QWERTY_FRCH_UC`] — same widths as lowercase.
pub static CTRLMAP_QWERTY_FRCH_UC: &CtrlMap = CTRLMAP_QWERTY_FRCH_LC;

// ---------------------------------------------------------------------------
// Ukrainian ЙЦУКЕН layout (lowercase + uppercase)
// ---------------------------------------------------------------------------

/// Ukrainian ЙЦУКЕН layout — lowercase (39 keys: 12 + 11 + 11 + 5).
pub static KEYMAP_UA_LC: &KeyMap = &[
    // Row 1 (12)
    k!(c"й"),
    k!(c"ц"),
    k!(c"у"),
    k!(c"к"),
    k!(c"е"),
    k!(c"н"),
    k!(c"г"),
    k!(c"ш"),
    k!(c"щ"),
    k!(c"з"),
    k!(c"х"),
    k!(c"ї"),
    k!(c"\n"),
    // Row 2 (11)
    k!(c"ф"),
    k!(c"і"),
    k!(c"в"),
    k!(c"а"),
    k!(c"п"),
    k!(c"р"),
    k!(c"о"),
    k!(c"л"),
    k!(c"д"),
    k!(c"ж"),
    k!(c"є"),
    k!(c"\n"),
    // Row 3 (11) — ABC + 8 letters + ь + Del
    ka!(KEY_ABC),
    k!(c"я"),
    k!(c"ч"),
    k!(c"с"),
    k!(c"м"),
    k!(c"и"),
    k!(c"т"),
    k!(c"ь"),
    k!(c"б"),
    k!(c"ю"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 (5) — navigation
    ka!(KEY_BACK),
    ka!(KEY_LANG_UA),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_UA_LC`] (39 entries: 12 + 11 + 11 + 5).
pub static CTRLMAP_UA_LC: &CtrlMap = &[
    // Row 1 — 12 keys
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 2 — 11 keys
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    // Row 3 — ABC(2) + 8×W1 + ь(1) + Del(2)  [action keys checked]
    CTRL_W2 | CTRL_CHECKED,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W1,
    CTRL_W2 | CTRL_CHECKED,
    // Row 4 — navigation  [action keys checked]
    CTRL_W3 | CTRL_CHECKED,
    CTRL_W1 | CTRL_CHECKED,
    CTRL_W1 | CTRL_CHECKED,
    CTRL_W6,
    CTRL_W3 | CTRL_CHECKED,
];

/// Ukrainian ЙЦУКЕН layout — uppercase (39 keys: 12 + 11 + 11 + 5).
pub static KEYMAP_UA_UC: &KeyMap = &[
    // Row 1 (12)
    k!(c"Й"),
    k!(c"Ц"),
    k!(c"У"),
    k!(c"К"),
    k!(c"Е"),
    k!(c"Н"),
    k!(c"Г"),
    k!(c"Ш"),
    k!(c"Щ"),
    k!(c"З"),
    k!(c"Х"),
    k!(c"Ї"),
    k!(c"\n"),
    // Row 2 (11)
    k!(c"Ф"),
    k!(c"І"),
    k!(c"В"),
    k!(c"А"),
    k!(c"П"),
    k!(c"Р"),
    k!(c"О"),
    k!(c"Л"),
    k!(c"Д"),
    k!(c"Ж"),
    k!(c"Є"),
    k!(c"\n"),
    // Row 3 (11)
    ka!(KEY_ABC_LOWER),
    k!(c"Я"),
    k!(c"Ч"),
    k!(c"С"),
    k!(c"М"),
    k!(c"И"),
    k!(c"Т"),
    k!(c"Ь"),
    k!(c"Б"),
    k!(c"Ю"),
    ka!(KEY_DEL),
    k!(c"\n"),
    // Row 4 (5)
    ka!(KEY_BACK),
    ka!(KEY_LANG_UA),
    ka!(KEY_123),
    ka!(KEY_SPACE),
    ka!(KEY_CONTINUE),
    k!(c""),
];

/// Ctrl map for [`KEYMAP_UA_UC`] — same widths as lowercase.
pub static CTRLMAP_UA_UC: &CtrlMap = CTRLMAP_UA_LC;

// ---------------------------------------------------------------------------
// Accent variant maps for long-press selection
// ---------------------------------------------------------------------------

// Lowercase accent variants
pub static ACCENT_MAP_A_LC: &KeyMap = &[k!(c"à"), k!(c"â"), k!(c"ä"), k!(c"æ"), k!(c"á"), k!(c"")];
pub static ACCENT_MAP_C_LC: &KeyMap = &[k!(c"ç"), k!(c"")];
pub static ACCENT_MAP_E_LC: &KeyMap = &[k!(c"é"), k!(c"è"), k!(c"ê"), k!(c"ë"), k!(c"")];
pub static ACCENT_MAP_I_LC: &KeyMap = &[k!(c"î"), k!(c"ï"), k!(c"í"), k!(c"")];
pub static ACCENT_MAP_N_LC: &KeyMap = &[k!(c"ñ"), k!(c"")];
pub static ACCENT_MAP_O_LC: &KeyMap = &[k!(c"ô"), k!(c"ö"), k!(c"œ"), k!(c"ó"), k!(c"")];
pub static ACCENT_MAP_S_LC: &KeyMap = &[k!(c"ß"), k!(c"")];
pub static ACCENT_MAP_U_LC: &KeyMap = &[k!(c"ù"), k!(c"û"), k!(c"ü"), k!(c"ú"), k!(c"")];
pub static ACCENT_MAP_Y_LC: &KeyMap = &[k!(c"ÿ"), k!(c"")];

// Uppercase accent variants
pub static ACCENT_MAP_A_UC: &KeyMap = &[k!(c"À"), k!(c"Â"), k!(c"Ä"), k!(c"Æ"), k!(c"Á"), k!(c"")];
pub static ACCENT_MAP_C_UC: &KeyMap = &[k!(c"Ç"), k!(c"")];
pub static ACCENT_MAP_E_UC: &KeyMap = &[k!(c"É"), k!(c"È"), k!(c"Ê"), k!(c"Ë"), k!(c"")];
pub static ACCENT_MAP_I_UC: &KeyMap = &[k!(c"Î"), k!(c"Ï"), k!(c"Í"), k!(c"")];
pub static ACCENT_MAP_N_UC: &KeyMap = &[k!(c"Ñ"), k!(c"")];
pub static ACCENT_MAP_O_UC: &KeyMap = &[k!(c"Ô"), k!(c"Ö"), k!(c"Œ"), k!(c"Ó"), k!(c"")];
pub static ACCENT_MAP_S_UC: &KeyMap = &[k!(c"ẞ"), k!(c"")];
pub static ACCENT_MAP_U_UC: &KeyMap = &[k!(c"Ù"), k!(c"Û"), k!(c"Ü"), k!(c"Ú"), k!(c"")];
pub static ACCENT_MAP_Y_UC: &KeyMap = &[k!(c"Ÿ"), k!(c"")];

// Cyrillic accent variants
pub static ACCENT_MAP_G_CYR_LC: &KeyMap = &[k!(c"ґ"), k!(c"")];
pub static ACCENT_MAP_G_CYR_UC: &KeyMap = &[k!(c"Ґ"), k!(c"")];

/// Returns the accent variant map for a base letter, or `None` if no accents exist.
///
/// The returned `KeyMap` is a flat null-terminated array suitable for passing
/// to `lv_buttonmatrix_set_map` (single row, no `\n` separators).
pub fn accent_variants(ch: &core::ffi::CStr) -> Option<&'static KeyMap> {
    match ch.to_bytes() {
        b"a" => Some(ACCENT_MAP_A_LC),
        b"A" => Some(ACCENT_MAP_A_UC),
        b"c" => Some(ACCENT_MAP_C_LC),
        b"C" => Some(ACCENT_MAP_C_UC),
        b"e" => Some(ACCENT_MAP_E_LC),
        b"E" => Some(ACCENT_MAP_E_UC),
        b"i" => Some(ACCENT_MAP_I_LC),
        b"I" => Some(ACCENT_MAP_I_UC),
        b"n" => Some(ACCENT_MAP_N_LC),
        b"N" => Some(ACCENT_MAP_N_UC),
        b"o" => Some(ACCENT_MAP_O_LC),
        b"O" => Some(ACCENT_MAP_O_UC),
        b"s" => Some(ACCENT_MAP_S_LC),
        b"S" => Some(ACCENT_MAP_S_UC),
        b"u" => Some(ACCENT_MAP_U_LC),
        b"U" => Some(ACCENT_MAP_U_UC),
        b"y" => Some(ACCENT_MAP_Y_LC),
        b"Y" => Some(ACCENT_MAP_Y_UC),
        b"\xd0\xb3" => Some(ACCENT_MAP_G_CYR_LC), // г → ґ
        b"\xd0\x93" => Some(ACCENT_MAP_G_CYR_UC), // Г → Ґ
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        CTRLMAP_QWERTY_DE, CTRLMAP_QWERTY_DE_LC, CTRLMAP_QWERTY_DE_UC, CTRLMAP_QWERTY_EN,
        CTRLMAP_QWERTY_EN_LC, CTRLMAP_QWERTY_EN_UC, CTRLMAP_QWERTY_FR, CTRLMAP_QWERTY_FR_LC,
        CTRLMAP_QWERTY_FR_UC, CTRLMAP_QWERTY_FRCH_LC, CTRLMAP_QWERTY_FRCH_UC, CTRLMAP_QWERTY_IT,
        CTRLMAP_QWERTY_IT_LC, CTRLMAP_QWERTY_IT_UC, CTRLMAP_SPECIAL, CTRLMAP_UA_LC, CTRLMAP_UA_UC,
        KEYMAP_NUMPAD, KEYMAP_QWERTY_DE, KEYMAP_QWERTY_DE_LC, KEYMAP_QWERTY_DE_UC,
        KEYMAP_QWERTY_EN, KEYMAP_QWERTY_EN_LC, KEYMAP_QWERTY_EN_UC, KEYMAP_QWERTY_FR,
        KEYMAP_QWERTY_FR_LC, KEYMAP_QWERTY_FR_UC, KEYMAP_QWERTY_FRCH_LC, KEYMAP_QWERTY_FRCH_UC,
        KEYMAP_QWERTY_IT, KEYMAP_QWERTY_IT_LC, KEYMAP_QWERTY_IT_UC, KEYMAP_SPECIAL, KEYMAP_UA_LC,
        KEYMAP_UA_UC, KeyMap, KeyboardLayout, KeyboardLocale, LocaleSwitcher, LvKeyboardMode,
    };

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn assert_terminated(name: &str, map: &KeyMap) {
        let last = map.last().expect("empty key map").0;
        // SAFETY: static C string literal is always valid NUL-terminated.
        let s = unsafe { core::ffi::CStr::from_ptr(last) };
        assert_eq!(s.to_bytes(), b"", "{name} must end with empty string");
    }

    /// Counts non-separator (`\n`), non-terminator (`""`) entries —
    /// i.e. the expected length of the companion [`CtrlMap`].
    fn count_keys(map: &KeyMap) -> usize {
        map.iter()
            .filter(|e| {
                let s = unsafe { core::ffi::CStr::from_ptr(e.0) };
                let b = s.to_bytes();
                b != b"\n" && !b.is_empty()
            })
            .count()
    }

    // ------------------------------------------------------------------
    // KeyboardLayout — built-in mode mapping
    // ------------------------------------------------------------------

    #[test]
    fn qwerty_maps_to_text_lower() {
        assert_eq!(
            KeyboardLayout::Qwerty.lv_mode(),
            LvKeyboardMode::TextLower as u32
        );
    }

    #[test]
    fn qwerty_upper_maps_to_text_upper() {
        assert_eq!(
            KeyboardLayout::QwertyUpper.lv_mode(),
            LvKeyboardMode::TextUpper as u32
        );
    }

    #[test]
    fn number_pad_maps_to_number() {
        assert_eq!(
            KeyboardLayout::NumberPad.lv_mode(),
            LvKeyboardMode::Number as u32
        );
    }

    #[test]
    fn special_chars_maps_to_special() {
        assert_eq!(
            KeyboardLayout::SpecialChars.lv_mode(),
            LvKeyboardMode::Special as u32
        );
    }

    #[test]
    fn custom_maps_to_user1_and_returns_map() {
        let layout = KeyboardLayout::Custom(KEYMAP_QWERTY_EN);
        assert_eq!(layout.lv_mode(), LvKeyboardMode::User1 as u32);
        assert!(layout.maps().is_some());
    }

    #[test]
    fn non_custom_returns_no_map() {
        assert!(KeyboardLayout::Qwerty.maps().is_none());
    }

    // ------------------------------------------------------------------
    // KeyboardLayout::Locale — slot and map routing
    // ------------------------------------------------------------------

    #[test]
    fn locale_en_us_maps_to_text_lower_with_custom_map() {
        let layout = KeyboardLayout::Locale(KeyboardLocale::EnUs);
        assert_eq!(layout.lv_mode(), LvKeyboardMode::TextLower as u32);
        assert!(layout.maps().is_some(), "EnUs now provides a custom map");
    }

    #[test]
    fn locale_numeric_maps_to_number_no_custom_map() {
        let layout = KeyboardLayout::Locale(KeyboardLocale::Numeric);
        assert_eq!(layout.lv_mode(), LvKeyboardMode::Number as u32);
        assert!(
            layout.maps().is_none(),
            "Numeric must not install a custom map"
        );
    }

    #[test]
    fn locale_de_maps_to_user2_with_map() {
        let layout = KeyboardLayout::Locale(KeyboardLocale::De);
        assert_eq!(layout.lv_mode(), LvKeyboardMode::User2 as u32);
        assert!(layout.maps().is_some(), "De must provide a custom map");
    }

    #[test]
    fn locale_fr_maps_to_user3_with_map() {
        let layout = KeyboardLayout::Locale(KeyboardLocale::Fr);
        assert_eq!(layout.lv_mode(), LvKeyboardMode::User3 as u32);
        assert!(layout.maps().is_some(), "Fr must provide a custom map");
    }

    #[test]
    fn locale_it_maps_to_user4_with_map() {
        let layout = KeyboardLayout::Locale(KeyboardLocale::It);
        assert_eq!(layout.lv_mode(), LvKeyboardMode::User4 as u32);
        assert!(layout.maps().is_some(), "It must provide a custom map");
    }

    #[test]
    fn locale_modes_stay_within_lvgl_v93_range() {
        for locale in [
            KeyboardLocale::EnUs,
            KeyboardLocale::Numeric,
            KeyboardLocale::De,
            KeyboardLocale::Fr,
            KeyboardLocale::It,
            KeyboardLocale::FrCh,
            KeyboardLocale::Ua,
        ] {
            assert!(
                locale.native_mode() < 8,
                "{locale:?} must not map outside LVGL v9.3 keyboard modes 0..=7"
            );
        }
    }

    // ------------------------------------------------------------------
    // From<KeyboardLocale> for KeyboardLayout (backward compat)
    // ------------------------------------------------------------------

    #[test]
    fn from_locale_en_us_routes_to_text_lower() {
        let layout: KeyboardLayout = KeyboardLocale::EnUs.into();
        assert_eq!(layout.lv_mode(), LvKeyboardMode::TextLower as u32);
    }

    #[test]
    fn from_locale_numeric_routes_to_number() {
        let layout: KeyboardLayout = KeyboardLocale::Numeric.into();
        assert_eq!(layout.lv_mode(), LvKeyboardMode::Number as u32);
    }

    #[test]
    fn from_locale_de_routes_to_user2() {
        let layout: KeyboardLayout = KeyboardLocale::De.into();
        assert_eq!(layout.lv_mode(), LvKeyboardMode::User2 as u32);
    }

    // ------------------------------------------------------------------
    // CtrlMap length correctness
    // ------------------------------------------------------------------

    #[test]
    fn ctrlmap_en_length_matches_keymap() {
        assert_eq!(
            CTRLMAP_QWERTY_EN.len(),
            count_keys(KEYMAP_QWERTY_EN),
            "CTRLMAP_QWERTY_EN length must equal non-separator key count"
        );
    }

    #[test]
    fn ctrlmap_de_length_matches_keymap() {
        assert_eq!(
            CTRLMAP_QWERTY_DE.len(),
            count_keys(KEYMAP_QWERTY_DE),
            "CTRLMAP_QWERTY_DE length must equal non-separator key count"
        );
    }

    #[test]
    fn ctrlmap_fr_length_matches_keymap() {
        assert_eq!(
            CTRLMAP_QWERTY_FR.len(),
            count_keys(KEYMAP_QWERTY_FR),
            "CTRLMAP_QWERTY_FR length must equal non-separator key count"
        );
    }

    #[test]
    fn ctrlmap_it_length_matches_keymap() {
        assert_eq!(
            CTRLMAP_QWERTY_IT.len(),
            count_keys(KEYMAP_QWERTY_IT),
            "CTRLMAP_QWERTY_IT length must equal non-separator key count"
        );
    }

    #[test]
    fn ctrlmap_en_expected_key_counts() {
        // EN: 10 + 9 + 9 + 3 = 31
        assert_eq!(CTRLMAP_QWERTY_EN.len(), 31);
    }

    #[test]
    fn ctrlmap_de_expected_key_counts() {
        // DE: 10 + 9 + 9 + 6 = 34
        assert_eq!(CTRLMAP_QWERTY_DE.len(), 34);
    }

    #[test]
    fn ctrlmap_fr_expected_key_counts() {
        // FR: 10 + 10 + 8 + 6 = 34
        assert_eq!(CTRLMAP_QWERTY_FR.len(), 34);
    }

    #[test]
    fn ctrlmap_it_expected_key_counts() {
        // IT: 10 + 9 + 9 + 4 = 32
        assert_eq!(CTRLMAP_QWERTY_IT.len(), 32);
    }

    #[test]
    fn ctrlmap_en_action_keys_are_wider() {
        use super::{CTRL_CHECKED, CTRL_W1, CTRL_W2};
        // Row 3: ABC(idx 19) and Del(idx 27) must be wider than letter keys
        // and marked with CTRL_CHECKED for action-key styling.
        assert!(
            CTRLMAP_QWERTY_EN[19] > CTRL_W1,
            "ABC must be wider than a letter key"
        );
        assert!(
            CTRLMAP_QWERTY_EN[27] > CTRL_W1,
            "Del must be wider than a letter key"
        );
        assert_eq!(CTRLMAP_QWERTY_EN[19], CTRL_W2 | CTRL_CHECKED);
        assert_eq!(CTRLMAP_QWERTY_EN[27], CTRL_W2 | CTRL_CHECKED);
    }

    // ------------------------------------------------------------------
    // Key map terminators
    // ------------------------------------------------------------------

    #[test]
    fn qwerty_en_is_terminated() {
        assert_terminated("KEYMAP_QWERTY_EN", KEYMAP_QWERTY_EN);
    }

    #[test]
    fn qwerty_de_is_terminated() {
        assert_terminated("KEYMAP_QWERTY_DE", KEYMAP_QWERTY_DE);
    }

    #[test]
    fn qwerty_fr_is_terminated() {
        assert_terminated("KEYMAP_QWERTY_FR", KEYMAP_QWERTY_FR);
    }

    #[test]
    fn qwerty_it_is_terminated() {
        assert_terminated("KEYMAP_QWERTY_IT", KEYMAP_QWERTY_IT);
    }

    #[test]
    fn numpad_is_terminated() {
        assert_terminated("KEYMAP_NUMPAD", KEYMAP_NUMPAD);
    }

    // ------------------------------------------------------------------
    // LocaleSwitcher
    // ------------------------------------------------------------------

    #[test]
    fn switcher_current_starts_at_index_zero() {
        let sw = LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De]);
        assert_eq!(sw.current(), KeyboardLocale::EnUs);
        assert_eq!(sw.index(), 0);
    }

    #[test]
    fn switcher_next_advances_and_wraps() {
        let mut sw =
            LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De, KeyboardLocale::Fr]);
        assert_eq!(sw.next(), KeyboardLocale::De);
        assert_eq!(sw.next(), KeyboardLocale::Fr);
        // wrap
        assert_eq!(sw.next(), KeyboardLocale::EnUs);
        assert_eq!(sw.index(), 0);
    }

    #[test]
    fn switcher_set_found_returns_true() {
        let mut sw =
            LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De, KeyboardLocale::Fr]);
        assert!(sw.set(KeyboardLocale::Fr));
        assert_eq!(sw.current(), KeyboardLocale::Fr);
        assert_eq!(sw.index(), 2);
    }

    #[test]
    fn switcher_set_not_found_returns_false_unchanged() {
        let mut sw = LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De]);
        assert!(!sw.set(KeyboardLocale::It));
        assert_eq!(
            sw.current(),
            KeyboardLocale::EnUs,
            "index must not change on miss"
        );
    }

    #[test]
    fn switcher_len_equals_n() {
        let sw =
            LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De, KeyboardLocale::Fr]);
        assert_eq!(sw.len(), 3);
    }

    #[test]
    fn switcher_is_empty_always_false() {
        let sw = LocaleSwitcher::new([KeyboardLocale::EnUs]);
        assert!(!sw.is_empty());
    }

    /// Proves `LocaleSwitcher` can be stored in a `const` (compile-time
    /// `.rodata`) context — essential for embedded no_std firmware.
    #[test]
    fn switcher_is_const_constructible() {
        const SW: LocaleSwitcher<3> =
            LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De, KeyboardLocale::Fr]);
        assert_eq!(SW.len(), 3);
        assert_eq!(SW.current(), KeyboardLocale::EnUs);
    }

    /// Const static placement — verifies zero-cost storage in `.rodata`.
    #[test]
    fn switcher_as_static() {
        static LANGS: LocaleSwitcher<4> = LocaleSwitcher::new([
            KeyboardLocale::EnUs,
            KeyboardLocale::De,
            KeyboardLocale::Fr,
            KeyboardLocale::It,
        ]);
        assert_eq!(LANGS.len(), 4);
    }

    // ------------------------------------------------------------------
    // New LC/UC map terminators
    // ------------------------------------------------------------------

    #[test]
    fn en_lc_is_terminated() {
        assert_terminated("EN_LC", KEYMAP_QWERTY_EN_LC);
    }
    #[test]
    fn en_uc_is_terminated() {
        assert_terminated("EN_UC", KEYMAP_QWERTY_EN_UC);
    }
    #[test]
    fn de_lc_is_terminated() {
        assert_terminated("DE_LC", KEYMAP_QWERTY_DE_LC);
    }
    #[test]
    fn de_uc_is_terminated() {
        assert_terminated("DE_UC", KEYMAP_QWERTY_DE_UC);
    }
    #[test]
    fn fr_lc_is_terminated() {
        assert_terminated("FR_LC", KEYMAP_QWERTY_FR_LC);
    }
    #[test]
    fn fr_uc_is_terminated() {
        assert_terminated("FR_UC", KEYMAP_QWERTY_FR_UC);
    }
    #[test]
    fn it_lc_is_terminated() {
        assert_terminated("IT_LC", KEYMAP_QWERTY_IT_LC);
    }
    #[test]
    fn it_uc_is_terminated() {
        assert_terminated("IT_UC", KEYMAP_QWERTY_IT_UC);
    }
    #[test]
    fn frch_lc_is_terminated() {
        assert_terminated("FRCH_LC", KEYMAP_QWERTY_FRCH_LC);
    }
    #[test]
    fn frch_uc_is_terminated() {
        assert_terminated("FRCH_UC", KEYMAP_QWERTY_FRCH_UC);
    }
    #[test]
    fn ua_lc_is_terminated() {
        assert_terminated("UA_LC", KEYMAP_UA_LC);
    }
    #[test]
    fn ua_uc_is_terminated() {
        assert_terminated("UA_UC", KEYMAP_UA_UC);
    }
    #[test]
    fn special_is_terminated() {
        assert_terminated("KEYMAP_SPECIAL", KEYMAP_SPECIAL);
    }
    #[test]
    fn special_ctrl_length_matches() {
        // 11 + 12 + 12 + 3 keys = 38 ctrl entries.
        let key_count = KEYMAP_SPECIAL
            .iter()
            .filter(|e| {
                let s = unsafe { core::ffi::CStr::from_ptr(e.0) };
                !s.to_bytes().is_empty() && s.to_bytes() != b"\n"
            })
            .count();
        assert_eq!(
            key_count,
            CTRLMAP_SPECIAL.len(),
            "KEYMAP_SPECIAL keys vs CTRLMAP_SPECIAL"
        );
        assert_eq!(key_count, 38);
    }

    // ------------------------------------------------------------------
    // New LC/UC ctrl map length correctness
    // ------------------------------------------------------------------

    #[test]
    fn ctrlmap_en_lc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_EN_LC.len(),
            count_keys(KEYMAP_QWERTY_EN_LC),
            "EN_LC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_en_uc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_EN_UC.len(),
            count_keys(KEYMAP_QWERTY_EN_UC),
            "EN_UC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_de_lc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_DE_LC.len(),
            count_keys(KEYMAP_QWERTY_DE_LC),
            "DE_LC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_de_uc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_DE_UC.len(),
            count_keys(KEYMAP_QWERTY_DE_UC),
            "DE_UC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_fr_lc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_FR_LC.len(),
            count_keys(KEYMAP_QWERTY_FR_LC),
            "FR_LC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_fr_uc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_FR_UC.len(),
            count_keys(KEYMAP_QWERTY_FR_UC),
            "FR_UC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_it_lc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_IT_LC.len(),
            count_keys(KEYMAP_QWERTY_IT_LC),
            "IT_LC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_it_uc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_IT_UC.len(),
            count_keys(KEYMAP_QWERTY_IT_UC),
            "IT_UC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_frch_lc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_FRCH_LC.len(),
            count_keys(KEYMAP_QWERTY_FRCH_LC),
            "FRCH_LC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_frch_uc_length() {
        assert_eq!(
            CTRLMAP_QWERTY_FRCH_UC.len(),
            count_keys(KEYMAP_QWERTY_FRCH_UC),
            "FRCH_UC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_ua_lc_length() {
        assert_eq!(
            CTRLMAP_UA_LC.len(),
            count_keys(KEYMAP_UA_LC),
            "UA_LC ctrl/key count mismatch"
        );
    }
    #[test]
    fn ctrlmap_ua_uc_length() {
        assert_eq!(
            CTRLMAP_UA_UC.len(),
            count_keys(KEYMAP_UA_UC),
            "UA_UC ctrl/key count mismatch"
        );
    }

    // ------------------------------------------------------------------
    // map_pair returns pairs for all text locales
    // ------------------------------------------------------------------

    #[test]
    fn map_pair_en_us() {
        assert!(KeyboardLocale::EnUs.map_pair().is_some());
    }
    #[test]
    fn map_pair_de() {
        assert!(KeyboardLocale::De.map_pair().is_some());
    }
    #[test]
    fn map_pair_fr() {
        assert!(KeyboardLocale::Fr.map_pair().is_some());
    }
    #[test]
    fn map_pair_it() {
        assert!(KeyboardLocale::It.map_pair().is_some());
    }
    #[test]
    fn map_pair_frch() {
        assert!(KeyboardLocale::FrCh.map_pair().is_some());
    }
    #[test]
    fn map_pair_ua() {
        assert!(KeyboardLocale::Ua.map_pair().is_some());
    }
    #[test]
    fn map_pair_numeric_is_none() {
        assert!(KeyboardLocale::Numeric.map_pair().is_none());
    }
}
