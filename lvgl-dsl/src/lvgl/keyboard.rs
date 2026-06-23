use crate::c_bindings;
use core::sync::atomic::{AtomicPtr, Ordering};

use super::align::LvAlign;
use super::color::Color;
use super::corner_radius::CornerRadius;
use super::event::{Event, LvEventCode};
use super::keyboard_layout::{
    CTRL_DISABLED, CTRLMAP_SPECIAL, KEY_CONTINUE, KEYMAP_SPECIAL, KeyMap, KeyboardLayout,
    KeyboardLocale, LvKeyboardMode,
};

// Most of these constants are used in cfg(not(test)) event handler
// functions; `is_continue_label_allowed` (compiled in tests too) uses a
// subset of them. The `allow(unused_imports)` covers the variants not
// referenced in any specific cfg.
#[allow(unused_imports)]
use super::keyboard_layout::{
    KEY_123, KEY_ABC, KEY_ABC_LOWER, KEY_BACK, KEY_BACKSPACE, KEY_DEL, KEY_LANG, KEY_LANG_CH,
    KEY_LANG_DE, KEY_LANG_EN, KEY_LANG_FR, KEY_LANG_IT, KEY_LANG_UA, KEY_OK, KEY_SPECIAL,
    accent_variants,
};
use super::keyboard_theme::{
    KeyboardTheme, SELECTOR_KEY_ACTION, SELECTOR_KEY_DISABLED, SELECTOR_KEY_NORMAL,
    SELECTOR_KEY_PRESSED,
};
use super::anim::{Anim, Path as AnimPath};
use super::font::Font;
use super::size::Size;
use super::state::LvObjFlag;
use super::textarea::TextArea;
use super::widget::{LvObj, Widget};

use core::cell::UnsafeCell;
use core::ffi::c_char;

/// `LV_BUTTONMATRIX_BUTTON_NONE` — sentinel returned when no button is selected.
const LV_BTNMATRIX_BTN_NONE: u32 = 0xFFFF;

// ── Per-keyboard custom handler state ─────────────────────────────────────

/// Tracks the current keyboard state needed by the custom event handler.
///
/// LVGL is strictly single-threaded — exactly one keyboard is active at a
/// time on the LVGL thread.  The state is stored in a plain `static` (via
/// [`LvglCell`]) instead of `thread_local!` so it compiles on `no_std`
/// bare-metal targets (e.g. Zephyr on Cortex-M) where TLS is unavailable.
///
/// # Limitation — single keyboard
///
/// Only **one** `Keyboard` instance may be active at a time.  Each
/// [`Keyboard::new`] call overwrites the global state, so event callbacks
/// for a previously-created keyboard will read the new keyboard's state.
/// This matches the embedded use-case of a single on-screen keyboard.
struct KbHandlerState {
    /// Raw `lv_obj_t *` of the keyboard object.
    obj: *mut c_bindings::lv_obj_t,
    /// Current locale — used to select the correct LC/UC map on shift toggle.
    locale: KeyboardLocale,
    /// Whether uppercase mode is currently active.
    uppercase: bool,
    /// Optional callback when the 🌐 (language) key is pressed.
    on_lang_cb: Option<fn(Event)>,
    /// Active accent variant popup (buttonmatrix), or null if none.
    accent_popup: *mut c_bindings::lv_obj_t,
    /// Whether the active layout has a ctrl map installed.
    ///
    /// When `false` (e.g. [`KeyboardLayout::Custom`] without a ctrl map),
    /// [`Keyboard::popover_keys(true)`] is silently ignored to prevent a
    /// null-pointer dereference inside LVGL's `lv_keyboard_update_ctrl_map`.
    ctrl_map_installed: bool,
    /// Font pointer configured via [`Keyboard::text_font`], if any.
    ///
    /// Stored so accent-popup buttonmatrix children (which are created
    /// dynamically and do not inherit the keyboard's local styles) can be
    /// styled with the same glyph-providing font. Null when no custom font
    /// was set — in that case the popup falls back to LVGL's default font.
    font_ptr: *const c_bindings::lv_font_t,
    /// Whether the consumer asked to replace the textual "Del" key with the
    /// ⌫ icon at install time (see [`Keyboard::del_as_icon`]).
    del_as_icon: bool,
    /// Per-mode mirror buffers used when `del_as_icon` is enabled **or**
    /// when [`Keyboard::continue_label`] sets a label override.
    ///
    /// Both transforms rewrite individual cells of the installed map, and
    /// LVGL retains the pointer passed to `lv_keyboard_set_map` for each
    /// mode slot and dereferences it on every repaint. Sharing a single
    /// mirror across modes would let a later install (into a different mode)
    /// silently rewrite the buffer that an earlier-installed mode still
    /// points at. We keep one stable, heap-owned buffer per mode instead.
    key_label_mirrors: alloc::collections::BTreeMap<u32, alloc::boxed::Box<[*const c_char; KEY_LABEL_MIRROR_LEN]>>,
    /// Optional override for the visible label of the `Continue` key.
    ///
    /// When `Some(label)`, every cell whose text matches [`KEY_CONTINUE`]
    /// in the installed map is rewritten (in the per-mode mirror buffer)
    /// to point at `label`. The key continues to behave like Continue —
    /// the event handler matches against both [`KEY_CONTINUE`] and this
    /// override, so [`Keyboard::on_continue`] callbacks still fire and
    /// [`Keyboard::set_continue_enabled`] still works.
    continue_label: Option<&'static core::ffi::CStr>,
    /// The most recently installed layout, used by [`Keyboard::del_as_icon`]
    /// to reinstall the *currently active* map (rather than always falling
    /// back to the locale's lc/uc map, which would clobber a
    /// [`KeyboardLayout::Custom`] selection).
    active_layout: Option<KeyboardLayout>,
}

/// A `Sync` wrapper around a value that is only accessed from a single thread.
///
/// LVGL runs all UI work (including every event callback) on one thread/task.
/// This wrapper makes it possible to store LVGL-related state in a plain
/// `static` without requiring `thread_local!` (which needs TLS and is not
/// available on bare-metal `no_std` targets).
///
/// # Safety
///
/// The caller **must** guarantee that all access occurs from a single thread.
/// Concurrent access from multiple threads is undefined behaviour.
struct LvglCell<T>(UnsafeCell<T>);

// SAFETY: All access is confined to the single LVGL thread (both the public
// Keyboard methods and the C event callbacks run on that same thread).
unsafe impl<T> Sync for LvglCell<T> {}

impl<T> LvglCell<T> {
    const fn new(val: T) -> Self {
        LvglCell(UnsafeCell::new(val))
    }

    /// Returns a shared reference to the inner value.
    ///
    /// # Safety
    ///
    /// Must be called from the LVGL thread only.  No mutable alias may exist.
    #[inline]
    unsafe fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }

    /// Returns a mutable reference to the inner value.
    ///
    /// # Safety
    ///
    /// Must be called from the LVGL thread only.  No other alias may exist.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

static KB_STATE: LvglCell<Option<KbHandlerState>> = LvglCell::new(None);

/// Per-mode runtime mirror used by key-label transforms (`del_as_icon`,
/// `continue_label`).
///
/// LVGL stores the map pointer it was handed and reads from it on every
/// repaint, so we can't mutate the read-only static keymap in place. Each
/// mode slot gets its own heap-owned `Box<[*const c_char; KEY_LABEL_MIRROR_LEN]>`
/// in [`KbHandlerState::key_label_mirrors`]; the helper below copies the active
/// map into that slot, conditionally rewrites cells based on which
/// transforms are currently enabled (`del_as_icon` swaps `KEY_DEL` →
/// `KEY_BACKSPACE`; `continue_label` swaps `KEY_CONTINUE` → the override
/// label), and returns the slot's stable pointer to `lv_keyboard_set_map`.
/// 128 entries is comfortably more than any layout we ship (largest is
/// QWERTY ≈ 40 cells including row separators and terminator).
const KEY_LABEL_MIRROR_LEN: usize = 128;

/// Pure helper extracted from [`accent_long_press_cb`] so the "undo base
/// char after popup opens" behaviour is unit-testable.
///
/// Returns `true` iff long-pressing `ch` should open an accent popup —
/// equivalent to "the base character was just inserted by the press and
/// must be retracted before the popup takes over".
fn should_undo_base_char_after_popup(ch: &core::ffi::CStr) -> bool {
    super::keyboard_layout::accent_variants(ch).is_some()
}

/// Returns `false` if `label` collides with a label in the reserved
/// action-key set dispatched by `custom_kb_event_cb` (layout/language/
/// backspace/back keys). Reusing such a label for Continue would either
/// hijack the reserved key's behaviour or cause Continue to never fire
/// `Ready` — depending on dispatch order — so we forbid the collision
/// entirely. Also rejects labels that break the LVGL map structure
/// (empty or `"\n"`).
fn is_continue_label_allowed(label: &core::ffi::CStr) -> bool {
    let bytes = label.to_bytes();
    // Empty / newline break the LVGL map (empty terminates the entire
    // map; newline is the row separator).
    if bytes.is_empty() || bytes == b"\n" {
        return false;
    }
    // Reserved action labels dispatched by custom_kb_event_cb. Any
    // collision is rejected to avoid label-based dispatch ambiguity
    // regardless of which handler arm runs first.
    const RESERVED: &[&core::ffi::CStr] = &[
        KEY_ABC,
        KEY_ABC_LOWER,
        KEY_123,
        KEY_BACK,
        KEY_BACKSPACE,
        KEY_DEL,
        KEY_LANG,
        KEY_LANG_EN,
        KEY_LANG_DE,
        KEY_LANG_FR,
        KEY_LANG_IT,
        KEY_LANG_CH,
        KEY_LANG_UA,
        // LVGL-internal action labels (mode-switch + LVGL-default Continue
        // label) — including these prevents collisions even though our
        // own dispatcher doesn't currently handle them.
        KEY_SPECIAL,
        KEY_OK,
    ];
    !RESERVED.iter().any(|r| r.to_bytes() == bytes)
}

/// Copies `src` into the [`KbHandlerState::key_label_mirrors`] entry for `mode`,
/// applying the currently-active text-cell transforms:
///
/// * If [`KbHandlerState::del_as_icon`] is set, every cell whose label is
///   the literal `"Del"` (matched by bytes, not pointer identity) is
///   rewritten to point at [`KEY_BACKSPACE`].
/// * If [`KbHandlerState::continue_label`] is `Some(label)`, every cell
///   whose label is [`KEY_CONTINUE`] is rewritten to point at `label`.
///
/// Returns a `*const *const c_char` suitable for `lv_keyboard_set_map`, or
/// the original `src` pointer (no swap performed) when the source map
/// cannot fit in the fixed-size mirror buffer. In that fallback case a
/// warning is logged (when std is available) so a runaway / oversized
/// custom keymap is visible at runtime instead of silently writing past
/// the buffer.
///
/// Matching by bytes (not pointer identity) means custom keymaps that
/// instantiate their own `c"Del"` string still get the icon swap, matching
/// the user-facing contract of [`Keyboard::del_as_icon`].
///
/// # Safety
/// `src` must point to an array of valid C-string pointers terminated by
/// `c""` (a non-null pointer to an empty C string), as required by
/// `lv_keyboard_set_map`. A null entry is also accepted as a terminator
/// and is normalised to `c""` in the mirror so the produced map remains a
/// valid LVGL keymap.
unsafe fn install_key_label_mirror(
    state: &mut KbHandlerState,
    mode: u32,
    src: *const *const c_char,
) -> *const *const c_char {
    let bs_ptr = KEY_BACKSPACE.as_ptr();
    let empty_sentinel: *const c_char = c"".as_ptr();
    let continue_override_ptr: Option<*const c_char> = state.continue_label.map(|s| s.as_ptr());

    // Probe the source map first; if it doesn't fit, fall back without
    // touching the destination (so the previously installed mirror for
    // this mode — if any — keeps its content for any in-flight repaint).
    let mut i = 0usize;
    let mut terminator: Option<*const c_char> = None;
    while i < KEY_LABEL_MIRROR_LEN.saturating_sub(1) {
        let p = unsafe { *src.add(i) };
        if p.is_null() {
            terminator = Some(empty_sentinel);
            break;
        }
        let bytes = unsafe { core::ffi::CStr::from_ptr(p) }.to_bytes();
        if bytes.is_empty() {
            terminator = Some(p);
            break;
        }
        i += 1;
    }
    let Some(term) = terminator else {
        // Source map is larger than the mirror capacity: leave it untouched
        // (no swaps applied) rather than corrupting memory. Both the
        // del-as-icon and continue-label overrides are skipped together.
        #[cfg(any(test, no_zephyr))]
        eprintln!(
            "[lvgl-dsl] install_key_label_mirror: keymap exceeds \
             KEY_LABEL_MIRROR_LEN capacity ({}); del-as-icon and \
             continue-label overrides skipped for this map",
            KEY_LABEL_MIRROR_LEN
        );
        return src;
    };

    // Get-or-allocate the per-mode mirror. The Box keeps the buffer at a
    // stable address even when the BTreeMap rebalances, so any pointer
    // LVGL retains for another mode remains valid.
    let mirror = state.key_label_mirrors.entry(mode).or_insert_with(|| {
        alloc::boxed::Box::new([core::ptr::null::<c_char>(); KEY_LABEL_MIRROR_LEN])
    });
    let term_idx = i;
    let mut j = 0usize;
    while j < term_idx {
        let p = unsafe { *src.add(j) };
        // Non-null and non-empty here by construction of the probe above.
        let bytes = unsafe { core::ffi::CStr::from_ptr(p) }.to_bytes();
        mirror[j] = if state.del_as_icon && bytes == b"Del" {
            bs_ptr
        } else if bytes == KEY_CONTINUE.to_bytes() {
            if let Some(override_ptr) = continue_override_ptr {
                override_ptr
            } else {
                p
            }
        } else {
            p
        };
        j += 1;
    }
    mirror[term_idx] = term;
    (**mirror).as_ptr()
}

/// Installs the lowercase map for the given locale into its native LVGL mode slot.
fn install_lc_map(obj: *mut c_bindings::lv_obj_t, locale: KeyboardLocale) {
    if let Some((lc_map, lc_ctrl, _, _)) = locale.map_pair() {
        let mut map_ptr = lc_map.as_ptr() as *const *const core::ffi::c_char;
        // SAFETY: KB_STATE is accessed only from the LVGL thread.
        unsafe {
            if let Some(state) = KB_STATE.get_mut().as_mut() {
                if state.del_as_icon || state.continue_label.is_some() {
                    map_ptr = install_key_label_mirror(state, locale.native_mode(), map_ptr);
                }
            }
        }
        unsafe {
            c_bindings::lv_keyboard_set_map(
                obj,
                locale.native_mode(),
                map_ptr,
                lc_ctrl.as_ptr(),
            );
        }
    }
    // Always switch mode — even locales without a custom map (e.g. Numeric)
    // need lv_keyboard_set_mode to activate their LVGL built-in slot.
    unsafe {
        c_bindings::lv_keyboard_set_mode(obj, locale.native_mode());
    }
}

/// Installs the uppercase map for the given locale into its native LVGL mode slot.
fn install_uc_map(obj: *mut c_bindings::lv_obj_t, locale: KeyboardLocale) {
    if let Some((_, _, uc_map, uc_ctrl)) = locale.map_pair() {
        let mut map_ptr = uc_map.as_ptr() as *const *const core::ffi::c_char;
        unsafe {
            if let Some(state) = KB_STATE.get_mut().as_mut() {
                if state.del_as_icon || state.continue_label.is_some() {
                    map_ptr = install_key_label_mirror(state, locale.native_mode(), map_ptr);
                }
            }
        }
        unsafe {
            c_bindings::lv_keyboard_set_map(
                obj,
                locale.native_mode(),
                map_ptr,
                uc_ctrl.as_ptr(),
            );
        }
    }
    unsafe {
        c_bindings::lv_keyboard_set_mode(obj, locale.native_mode());
    }
}

/// Installs the [`KEYMAP_SPECIAL`] / [`CTRLMAP_SPECIAL`] custom map into
/// LVGL's built-in [`LvKeyboardMode::Special`] slot so the `123` toggle
/// no longer falls back to LVGL's default Special map (which uses
/// Font-Awesome glyphs for action keys and renders as tofu when the
/// keyboard font lacks the symbol range).
fn install_special_map(obj: *mut c_bindings::lv_obj_t) {
    let mut map_ptr = KEYMAP_SPECIAL.as_ptr() as *const *const core::ffi::c_char;
    // SAFETY: KB_STATE is accessed only from the LVGL thread.
    unsafe {
        if let Some(state) = KB_STATE.get_mut().as_mut() {
            if state.del_as_icon || state.continue_label.is_some() {
                map_ptr =
                    install_key_label_mirror(state, LvKeyboardMode::Special as u32, map_ptr);
            }
        }
    }
    unsafe {
        c_bindings::lv_keyboard_set_map(
            obj,
            LvKeyboardMode::Special as u32,
            map_ptr,
            CTRLMAP_SPECIAL.as_ptr(),
        );
    }
}

// ── Accent popup helpers ──────────────────────────────────────────────────

/// Height (px) of the accent popup strip.
const ACCENT_POPUP_H: i32 = 44;
/// Vertical gap (px) between the popup and the top of the keyboard.
const ACCENT_POPUP_GAP: i32 = 4;

/// Destroys the active accent popup (if any) and clears the state.
#[cfg(not(test))]
fn dismiss_accent_popup() {
    // SAFETY: called from the LVGL thread only (event callback context).
    let state = unsafe { KB_STATE.get_mut() };
    if let Some(state) = state.as_mut() {
        if !state.accent_popup.is_null() {
            unsafe { c_bindings::lv_obj_delete(state.accent_popup) };
            state.accent_popup = core::ptr::null_mut();
        }
    }
}

/// Event handler for VALUE_CHANGED on the accent popup buttonmatrix.
///
/// Inserts the selected accent variant into the textarea, then destroys
/// the popup.
#[cfg(not(test))]
unsafe extern "C" fn accent_popup_event_cb(e: *mut c_bindings::lv_event_t) {
    let popup_obj = unsafe { c_bindings::lv_event_get_target(e) as *mut c_bindings::lv_obj_t };
    let btn_id = unsafe { c_bindings::lv_buttonmatrix_get_selected_button(popup_obj) };

    if btn_id == LV_BTNMATRIX_BTN_NONE {
        return;
    }

    let txt_ptr =
        unsafe { c_bindings::lv_buttonmatrix_get_button_text(popup_obj as *const _, btn_id) };
    if txt_ptr.is_null() {
        return;
    }

    // Insert the accent character into the keyboard's bound textarea.
    // SAFETY: called from the LVGL thread (event callback).
    let state = unsafe { KB_STATE.get() };
    if let Some(state) = state.as_ref() {
        let ta = unsafe { c_bindings::lv_keyboard_get_textarea(state.obj) };
        if !ta.is_null() {
            unsafe { c_bindings::lv_textarea_add_text(ta, txt_ptr) };
        }
    }

    // Destroy the popup.
    dismiss_accent_popup();
}

/// Event handler for LV_EVENT_LONG_PRESSED on the keyboard.
///
/// If the long-pressed key has accent variants, creates a popup
/// buttonmatrix above the keyboard showing those variants.
#[cfg(not(test))]
unsafe extern "C" fn accent_long_press_cb(e: *mut c_bindings::lv_event_t) {
    let obj = unsafe { c_bindings::lv_event_get_target(e) as *mut c_bindings::lv_obj_t };
    let btn_id = unsafe { c_bindings::lv_buttonmatrix_get_selected_button(obj) };

    if btn_id == LV_BTNMATRIX_BTN_NONE {
        return;
    }

    let txt_ptr = unsafe { c_bindings::lv_buttonmatrix_get_button_text(obj as *const _, btn_id) };
    if txt_ptr.is_null() {
        return;
    }

    let txt = unsafe { core::ffi::CStr::from_ptr(txt_ptr) };

    // Look up accent variants for this key.
    // Use the same predicate (`accent_variants(..).is_some()`) for both
    // "does this need a popup?" and "did the regular VALUE_CHANGED handler
    // already insert the base char that we need to undo?". The helper
    // [`should_undo_base_char_after_popup`] is the testable form.
    let variants = match accent_variants(txt) {
        Some(v) => v,
        None => return,
    };
    debug_assert!(should_undo_base_char_after_popup(txt));

    // Dismiss any previous popup.
    dismiss_accent_popup();

    // Create the accent popup as a child of the keyboard's parent (screen).
    let parent = unsafe { c_bindings::lv_obj_get_parent(obj) };
    if parent.is_null() {
        return;
    }

    let popup = unsafe { c_bindings::lv_buttonmatrix_create(parent) };
    if popup.is_null() {
        return;
    }

    // The keyboard's parent may be a flex/grid container (e.g. the
    // courier_user_list screen uses a flex_col root). LVGL layout managers
    // reposition managed children and ignore lv_obj_set_pos, which would
    // snap the popup to the bottom of the column instead of floating it
    // above the keyboard. LV_OBJ_FLAG_FLOATING opts the popup out of the
    // parent's layout so the explicit position below takes effect.
    unsafe {
        c_bindings::lv_obj_add_flag(popup, LvObjFlag::FLOATING.0);
    }

    // Set the accent variant map on the popup.
    unsafe {
        c_bindings::lv_buttonmatrix_set_map(
            popup,
            variants.as_ptr() as *const *const core::ffi::c_char,
        );
    }

    // Compute number of buttons (map entries excluding the "" terminator).
    let btn_count = variants.len().saturating_sub(1) as i32;
    let btn_w: i32 = 36;
    let popup_w = btn_count * btn_w + (btn_count - 1) * 4 + 16; // buttons + gaps + padding

    // Position the popup: centered above the keyboard.
    let kb_x = unsafe { c_bindings::lv_obj_get_x(obj) };
    let kb_y = unsafe { c_bindings::lv_obj_get_y(obj) };
    let kb_w = unsafe { c_bindings::lv_obj_get_width(obj) };
    let popup_x = kb_x + (kb_w - popup_w) / 2;
    let popup_y = kb_y - ACCENT_POPUP_H - ACCENT_POPUP_GAP;

    unsafe {
        c_bindings::lv_obj_set_size(popup, popup_w, ACCENT_POPUP_H);
        c_bindings::lv_obj_set_pos(popup, popup_x, popup_y);

        // Style the popup: dark background, rounded corners.
        c_bindings::lv_obj_set_style_bg_color(
            popup,
            c_bindings::lv_color_hex(0x333333),
            0, // LV_PART_MAIN
        );
        c_bindings::lv_obj_set_style_bg_opa(popup, 255, 0);
        c_bindings::lv_obj_set_style_radius(popup, 8, 0);
        c_bindings::lv_obj_set_style_pad_top(popup, 4, 0);
        c_bindings::lv_obj_set_style_pad_bottom(popup, 4, 0);
        c_bindings::lv_obj_set_style_pad_left(popup, 4, 0);
        c_bindings::lv_obj_set_style_pad_right(popup, 4, 0);
        c_bindings::lv_obj_set_style_pad_column(popup, 4, 0);

        // Style the buttons (LV_PART_ITEMS = 0x00050000).
        let part_items: u32 = 0x00050000;
        c_bindings::lv_obj_set_style_bg_color(
            popup,
            c_bindings::lv_color_hex(0x555555),
            part_items,
        );
        c_bindings::lv_obj_set_style_bg_opa(popup, 255, part_items);
        c_bindings::lv_obj_set_style_radius(popup, 6, part_items);
        c_bindings::lv_obj_set_style_text_color(popup, c_bindings::lv_color_white(), part_items);

        // Apply the keyboard's configured custom font to the popup so glyphs
        // in the U+00A0–U+017F accent ranges actually render (LVGL's default
        // Montserrat subset does not cover them, so they would appear as
        // tofu □□□□). The font pointer is captured when the consumer calls
        // [`Keyboard::text_font`]. Falls back to LVGL's default font when no
        // custom font was configured.
        let font_ptr = KB_STATE
            .get()
            .as_ref()
            .map_or(core::ptr::null(), |s| s.font_ptr);
        if !font_ptr.is_null() {
            c_bindings::lv_obj_set_style_text_font(popup, font_ptr, 0);
            c_bindings::lv_obj_set_style_text_font(popup, font_ptr, part_items);
        }

        // Register click handler on the popup.
        c_bindings::lv_obj_add_event_cb(
            popup,
            Some(accent_popup_event_cb),
            LvEventCode::ValueChanged as u32,
            core::ptr::null_mut(),
        );
    }

    // Store the popup pointer in the handler state.
    // SAFETY: called from the LVGL thread (event callback).
    let state = unsafe { KB_STATE.get_mut() };
    if let Some(state) = state.as_mut() {
        state.accent_popup = popup;
    }

    // Bring the popup to the top of the parent's z-order so it always
    // paints above sibling content (search bar, results list, etc.).
    // Equivalent to the v8 compat helper `lv_obj_move_foreground`.
    unsafe {
        let n = c_bindings::lv_obj_get_child_count(parent);
        if n > 0 {
            c_bindings::lv_obj_move_to_index(popup, (n - 1) as i32);
        }
    }

    // Bug 1: lv_buttonmatrix fires LV_EVENT_VALUE_CHANGED on PRESS (not
    // release), so the regular `custom_kb_event_cb` already inserted the
    // base character ~400 ms before LV_EVENT_LONG_PRESSED fired. Now that
    // the popup has been successfully created, retract that base char so
    // the user sees only the accent they ultimately choose (or nothing if
    // they dismiss the popup by tapping elsewhere).
    unsafe {
        let ta = c_bindings::lv_keyboard_get_textarea(obj);
        if !ta.is_null() {
            c_bindings::lv_textarea_delete_char(ta);
        }
    }

    // Tell LVGL to ignore the rest of this press from the active input
    // device until the user lifts their finger. Otherwise the touchscreen
    // keeps firing LV_EVENT_LONG_PRESSED_REPEAT every ~100 ms, each of
    // which the keyboard turns into LV_EVENT_VALUE_CHANGED — that path
    // dismisses the freshly-opened popup and inserts the base character
    // repeatedly ("aaaa…").
    unsafe {
        let indev = c_bindings::lv_indev_active();
        if !indev.is_null() {
            c_bindings::lv_indev_wait_release(indev);
        }
    }
}

/// Custom event handler for VALUE_CHANGED events on the keyboard.
///
/// Replaces LVGL's `lv_keyboard_def_event_cb` to support the new layout keys:
/// `ABC`, `abc`, `⌫`, `Back`, `Continue`, `🌐`, `123`.
#[cfg(not(test))]
unsafe extern "C" fn custom_kb_event_cb(e: *mut c_bindings::lv_event_t) {
    // If an accent popup is active, dismiss it and suppress this key event.
    // SAFETY: called from the LVGL thread (event callback).
    let popup_active = unsafe { KB_STATE.get() }
        .as_ref()
        .map_or(false, |st| !st.accent_popup.is_null());
    if popup_active {
        dismiss_accent_popup();
        return;
    }

    let obj = unsafe { c_bindings::lv_event_get_target(e) as *mut c_bindings::lv_obj_t };
    let btn_id = unsafe { c_bindings::lv_buttonmatrix_get_selected_button(obj) };

    // LV_BUTTONMATRIX_BUTTON_NONE
    if btn_id == LV_BTNMATRIX_BTN_NONE {
        return;
    }

    let txt_ptr = unsafe { c_bindings::lv_buttonmatrix_get_button_text(obj as *const _, btn_id) };
    if txt_ptr.is_null() {
        return;
    }

    let txt = unsafe { core::ffi::CStr::from_ptr(txt_ptr) };

    // ── Action key dispatch ───────────────────────────────────────────
    if txt == KEY_ABC {
        // Switch to uppercase
        // SAFETY: called from the LVGL thread (event callback).
        let state = unsafe { KB_STATE.get_mut() };
        if let Some(state) = state.as_mut() {
            state.uppercase = true;
            install_uc_map(state.obj, state.locale);
        }
        return;
    }

    if txt == KEY_ABC_LOWER {
        // Switch to lowercase
        // SAFETY: called from the LVGL thread (event callback).
        let state = unsafe { KB_STATE.get_mut() };
        if let Some(state) = state.as_mut() {
            state.uppercase = false;
            install_lc_map(state.obj, state.locale);
        }
        return;
    }

    if txt == KEY_123 {
        // Switch to special mode — LVGL's Special layout includes numerals
        // in row 1 plus symbols below, matching the conventional "123"
        // mobile keyboard affordance.
        unsafe {
            c_bindings::lv_keyboard_set_mode(
                obj,
                super::keyboard_layout::LvKeyboardMode::Special as u32,
            );
        }
        return;
    }

    if txt == KEY_DEL {
        // Delete character
        let ta = unsafe { c_bindings::lv_keyboard_get_textarea(obj) };
        if !ta.is_null() {
            unsafe { c_bindings::lv_textarea_delete_char(ta) };
        }
        return;
    }

    if txt == KEY_BACKSPACE {
        // ⌫ symbol — identical behaviour to KEY_DEL
        let ta = unsafe { c_bindings::lv_keyboard_get_textarea(obj) };
        if !ta.is_null() {
            unsafe { c_bindings::lv_textarea_delete_char(ta) };
        }
        return;
    }

    if txt == KEY_BACK {
        // Fire cancel event
        unsafe {
            c_bindings::lv_obj_send_event(obj, LvEventCode::Cancel as u32, core::ptr::null_mut());
        }
        return;
    }

    if txt == KEY_CONTINUE
        || unsafe { KB_STATE.get() }
            .as_ref()
            .and_then(|st| st.continue_label)
            .is_some_and(|lbl| txt == lbl)
    {
        // Fire ready event
        unsafe {
            c_bindings::lv_obj_send_event(obj, LvEventCode::Ready as u32, core::ptr::null_mut());
        }
        return;
    }

    if txt == KEY_LANG
        || txt == KEY_LANG_EN
        || txt == KEY_LANG_DE
        || txt == KEY_LANG_FR
        || txt == KEY_LANG_IT
        || txt == KEY_LANG_CH
        || txt == KEY_LANG_UA
    {
        // Fire language callback
        // SAFETY: called from the LVGL thread (event callback).
        let cb = unsafe { KB_STATE.get() }
            .as_ref()
            .and_then(|st| st.on_lang_cb);
        if let Some(cb) = cb {
            cb(Event::from_raw(e));
        }
        return;
    }

    // ── Default: delegate to LVGL's built-in handler ─────────────────
    //
    // Keys we don't explicitly recognise (regular characters on custom
    // locale maps, or LVGL's standard action labels like LV_SYMBOL_BACKSPACE
    // / "Ok" / "#@!" on built-in layouts) are forwarded to
    // `lv_keyboard_def_event_cb`.  This preserves correct behaviour for
    // `KeyboardLayout::Qwerty`, `QwertyUpper`, `NumberPad`, `SpecialChars`,
    // and `Custom` maps that reuse LVGL's standard action labels, while also
    // handling plain character insertion for our locale maps.
    unsafe { c_bindings::lv_keyboard_def_event_cb(e) };
}

/// Keyboard object awaiting its HIDDEN flag after a slide-hide animation.
/// LVGL is single-threaded, so one global suffices.
static SLIDE_HIDE_PENDING: AtomicPtr<c_bindings::lv_obj_t> = AtomicPtr::new(core::ptr::null_mut());

/// Animation executor: applies y-coordinate during a slide animation.
///
/// Compatible with `lv_anim_exec_xcb_t` (`void (*)(void *, int32_t)`).
/// `lv_obj_t *` and `void *` share the same representation on all targets.
unsafe extern "C" fn slide_exec_y(var: *mut core::ffi::c_void, val: i32) {
    // SAFETY: `var` was set via lv_anim_set_var to a valid `lv_obj_t *`.
    // We animate lv_obj_set_style_translate_y instead of lv_obj_set_y so the
    // translate delta is layered on top of LVGL's stored BottomMid alignment
    // rather than fighting it (lv_obj_set_y would be overridden by the layout
    // pass on every lv_timer_handler tick).
    unsafe { c_bindings::lv_obj_set_style_translate_y(var as *mut c_bindings::lv_obj_t, val, 0) }
}

/// Animation completion callback: hides the keyboard once it has fully slid
/// off the bottom of the screen.
unsafe extern "C" fn on_slide_hide_done(_anim: *mut c_bindings::lv_anim_t) {
    let obj = SLIDE_HIDE_PENDING.load(Ordering::Relaxed);
    if !obj.is_null() {
        // SAFETY: obj was stored as a valid `lv_obj_t *` in slide_hide().
        unsafe { c_bindings::lv_obj_add_flag(obj, LvObjFlag::HIDDEN.0) };
        SLIDE_HIDE_PENDING.store(core::ptr::null_mut(), Ordering::Relaxed);
    }
}

/// High-level wrapper around an LVGL keyboard widget (`lv_keyboard`).
///
/// Wraps an `lv_keyboard_create`-allocated object and inherits all layout,
/// style, event, and state methods from the [`Widget`] trait.
///
/// A keyboard is typically docked to the bottom of the screen and bound to a
/// [`TextArea`] using [`bind_textarea`](Keyboard::bind_textarea).
///
/// Requires `CONFIG_LV_USE_KEYBOARD=y` in the LVGL/Kconfig configuration.
///
/// # Single-instance limitation
///
/// Only one `Keyboard` may be active at a time.  Creating a second instance
/// with [`Keyboard::new`] overwrites the global handler state (`KB_STATE`),
/// making the previous instance inoperable.  Drop or hide the first keyboard
/// before constructing a new one.
///
/// # LVGL thread affinity
///
/// LVGL is not thread-safe.  All methods on `Keyboard` — including
/// construction — must be called exclusively from the LVGL/UI task.  This
/// type is deliberately `!Send + !Sync` (enforced by the [`core::marker::PhantomData`]
/// field) to prevent accidental cross-thread usage at compile time.
///
/// # Example
///
/// ```rust,ignore
/// use lvgl_dsl::lvgl::prelude::*;
///
/// fn on_ready(_e: Event) { /* commit input */ }
///
/// let screen = Screen::new();
///
/// let ta = TextArea::new(&screen)
///     .placeholder_text("Type here…")
///     .one_line(true);
///
/// let kb = Keyboard::new(&screen)
///     .full_width()
///     .layout(KeyboardLayout::Qwerty)
///     .theme(&KeyboardTheme::DARK)
///     .popover_keys(true)
///     .bind_textarea(&ta)
///     .on_ready(on_ready);
///
/// screen.load();
/// ```
pub struct Keyboard {
    obj: LvObj,
    /// Marker that makes `Keyboard` explicitly `!Send + !Sync`.
    ///
    /// LVGL objects must only be accessed from the LVGL/UI thread.  Even
    /// though `LvObj` already carries a raw pointer (which is implicitly
    /// `!Send + !Sync`), the explicit marker documents the invariant and
    /// guards against future refactoring that could inadvertently remove it.
    _thread_bound: core::marker::PhantomData<*const ()>,
}

impl Widget for Keyboard {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Keyboard {
    /// Creates a new keyboard widget as a child of `parent`.
    ///
    /// Replaces LVGL's default keyboard event handler with a custom Rust
    /// handler that supports the new layout keys (`Back`, `Continue`, `🌐`,
    /// `⌫`, `ABC`/`abc` shift toggle, `123`).
    ///
    /// The keyboard starts in English US lowercase mode.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> Keyboard {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_keyboard_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_keyboard_create returned null");
        }

        // Replace the default LVGL event handler with our custom one.
        #[cfg(not(test))]
        unsafe {
            // Remove LVGL's built-in handler.
            c_bindings::lv_obj_remove_event_cb(obj, Some(c_bindings::lv_keyboard_def_event_cb));

            // Install custom handler for VALUE_CHANGED events.
            c_bindings::lv_obj_add_event_cb(
                obj,
                Some(custom_kb_event_cb),
                LvEventCode::ValueChanged as u32,
                core::ptr::null_mut(),
            );

            // Install long-press handler for accent variant popup.
            c_bindings::lv_obj_add_event_cb(
                obj,
                Some(accent_long_press_cb),
                LvEventCode::LongPressed as u32,
                core::ptr::null_mut(),
            );
        }

        // Initialize handler state — default to EnUs lowercase.
        // SAFETY: called from the LVGL thread (construction is always on the
        // UI thread); no other alias to KB_STATE exists at this point.
        unsafe {
            *KB_STATE.get_mut() = Some(KbHandlerState {
                obj,
                locale: KeyboardLocale::EnUs,
                uppercase: false,
                on_lang_cb: None,
                accent_popup: core::ptr::null_mut(),
                ctrl_map_installed: true,
                font_ptr: core::ptr::null(),
                del_as_icon: false,
                key_label_mirrors: alloc::collections::BTreeMap::new(),
                continue_label: None,
                active_layout: Some(KeyboardLayout::Locale(KeyboardLocale::EnUs)),
            });
        }

        // Install the EnUs lowercase map.
        install_lc_map(obj, KeyboardLocale::EnUs);

        // Install the custom Special-mode map so the `123` toggle renders
        // action keys (Del / Back / Continue / abc) instead of unsupported
        // LVGL Font-Awesome glyphs.
        install_special_map(obj);

        // Install default disabled-state visuals so keys toggled via
        // `set_continue_enabled(false)` (or any future per-button DISABLED
        // ctrl) render in a recognisably greyed-out style.  Only the
        // affected button enters `LV_STATE_DISABLED`, so this style does
        // not bleed into other keys.
        // SAFETY: obj is non-null; selector is LV_PART_ITEMS | LV_STATE_DISABLED.
        unsafe {
            c_bindings::lv_obj_set_style_bg_color(
                obj,
                Color::hex(0xE5E7EA).to_lv(),
                SELECTOR_KEY_DISABLED,
            );
            c_bindings::lv_obj_set_style_text_color(
                obj,
                Color::hex(0x9CA3AF).to_lv(),
                SELECTOR_KEY_DISABLED,
            );
        }

        Keyboard {
            obj: LvObj::from_raw(obj),
            _thread_bound: core::marker::PhantomData,
        }
    }

    // -----------------------------------------------------------------------
    // Sizing & positioning
    // -----------------------------------------------------------------------

    /// Docks the keyboard to the bottom of its parent at full width.
    ///
    /// Sets width to 100 % of the parent and aligns the bottom-mid anchor
    /// flush against the parent's bottom edge.
    pub fn full_width(&self) -> &Self {
        self.width(Size::Pct(100)).align(LvAlign::BottomMid, 0, 0)
    }

    /// Sets an explicit width and height for the keyboard.
    pub fn custom_size(&self, w: Size, h: Size) -> &Self {
        self.width(w).height(h)
    }

    /// Adjusts the keyboard position by pixel offsets relative to its current
    /// alignment anchor.
    pub fn offset(&self, x: i32, y: i32) -> &Self {
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe {
            c_bindings::lv_obj_align(self.lv_obj().raw(), LvAlign::BottomMid as u32, x, y);
        }
        self
    }

    // -----------------------------------------------------------------------
    // Layout selection
    // -----------------------------------------------------------------------

    /// Switches the keyboard to one of the predefined or custom layouts.
    ///
    /// For [`KeyboardLayout::Locale`] and [`KeyboardLayout::Custom`] this also
    /// calls `lv_keyboard_set_map` to install the map (and ctrl map where
    /// available) before switching mode.
    pub fn layout(&self, layout: KeyboardLayout) -> &Self {
        // Sync locale state for any Locale(..) variant, regardless of whether
        // a custom map is installed (Numeric has no map but still needs the update).
        unsafe {
            let state = KB_STATE.get_mut();
            if let Some(state) = state.as_mut() {
                if let KeyboardLayout::Locale(locale) = &layout {
                    state.locale = *locale;
                    state.uppercase = false;
                }
                state.active_layout = Some(layout);
            }
        }
        if let Some((map, ctrl)) = layout.maps() {
            // Track whether this layout has a ctrl map so popover_keys()
            // can reject enabling popovers when one is missing.
            let has_ctrl = ctrl.is_some();
            unsafe {
                let state = KB_STATE.get_mut();
                if let Some(state) = state.as_mut() {
                    state.ctrl_map_installed = has_ctrl;
                }
            }
            // Popovers must be disabled before calling lv_keyboard_set_map
            // with a null ctrl map.  lv_keyboard_update_ctrl_map only reads
            // the ctrl map when popovers are enabled, so disabling them here
            // makes passing null safe.
            if !has_ctrl {
                unsafe { c_bindings::lv_keyboard_set_popovers(self.lv_obj().raw(), false) }
            }
            // Install the map + ctrl map before switching mode.
            // SAFETY: `map` is a `&'static KeyMap`; `ctrl` is `&'static CtrlMap` or null.
            unsafe {
                let mut map_ptr = map.as_ptr() as *const *const core::ffi::c_char;
                if let Some(state) = KB_STATE.get_mut().as_mut() {
                    if state.del_as_icon || state.continue_label.is_some() {
                        map_ptr = install_key_label_mirror(state, layout.lv_mode(), map_ptr);
                    }
                }
                c_bindings::lv_keyboard_set_map(
                    self.lv_obj().raw(),
                    layout.lv_mode(),
                    // SAFETY: KeyMapEntry is repr(transparent) over *const c_char.
                    map_ptr,
                    ctrl.map_or(core::ptr::null(), |c| c.as_ptr()),
                );
            }
        } else {
            // Built-in layouts (Qwerty, QwertyUpper, NumberPad, SpecialChars, and
            // Locale(Numeric)) use LVGL's own ctrl maps, so popovers are always safe.
            unsafe {
                let state = KB_STATE.get_mut();
                if let Some(state) = state.as_mut() {
                    state.ctrl_map_installed = true;
                }
            }
        }
        // SAFETY: lv_mode() returns a valid lv_keyboard_mode_t integer.
        unsafe { c_bindings::lv_keyboard_set_mode(self.lv_obj().raw(), layout.lv_mode()) }
        self
    }

    /// Selects a keyboard mode by raw `LvKeyboardMode` value.
    ///
    /// Use [`layout`](Keyboard::layout) for the DSL-friendly API.
    pub fn mode(&self, mode: super::keyboard_layout::LvKeyboardMode) -> &Self {
        // SAFETY: mode is a repr(u32) enum value.
        unsafe { c_bindings::lv_keyboard_set_mode(self.lv_obj().raw(), mode as u32) }
        self
    }

    /// Selects a layout based on a [`KeyboardLocale`] shorthand.
    ///
    /// Updates the internal locale state and installs the appropriate
    /// lowercase map for the new locale.
    pub fn locale(&self, locale: KeyboardLocale) -> &Self {
        // SAFETY: called from the LVGL thread (public API, UI context).
        let state = unsafe { KB_STATE.get_mut() };
        if let Some(state) = state.as_mut() {
            state.locale = locale;
            state.uppercase = false;
            // All locales either install a custom ctrl map (map_pair is Some) or
            // use an LVGL built-in mode that has its own ctrl map. Popovers are
            // always safe after a locale switch.
            state.ctrl_map_installed = true;
            state.active_layout = Some(KeyboardLayout::Locale(locale));
        }
        // Install the locale's lowercase map.
        install_lc_map(self.lv_obj().raw(), locale);
        self
    }

    /// Installs a custom [`KeyMap`] into the `User1` slot and activates it.
    ///
    /// `map` must be a `&'static KeyMap` — a flat array of `*const c_char`
    /// pointers using `c"\n"` as row separators and terminated by `c""`
    /// (a non-null pointer to an empty C string), as required by
    /// `lv_keyboard_set_map`. Do **not** terminate the map with a null
    /// pointer: LVGL would dereference it.
    pub fn custom_map(&self, map: &'static KeyMap) -> &Self {
        self.layout(KeyboardLayout::Custom(map))
    }

    /// Sets the font used to render key labels.
    ///
    /// Overrides the `Widget::text_font` default to also record the font
    /// pointer in the global [`KB_STATE`] so the accent-popup buttonmatrix
    /// (created dynamically on long-press, outside the keyboard's style
    /// inheritance scope) can render glyphs from the same font. Without
    /// this, accent variants in U+00A0–U+017F would fall back to LVGL's
    /// default Montserrat subset and appear as tofu.
    pub fn text_font(&self, font: &Font) -> &Self {
        // SAFETY: obj is non-null; font.as_ptr() is a static linker symbol.
        unsafe {
            c_bindings::lv_obj_set_style_text_font(self.lv_obj().raw(), font.as_ptr(), 0);
        }
        // SAFETY: called from the LVGL thread (UI context).
        let state = unsafe { KB_STATE.get_mut() };
        if let Some(state) = state.as_mut() {
            state.font_ptr = font.as_ptr();
        }
        self
    }

    /// Returns the font pointer currently stored for popup styling.
    ///
    /// Exposed for tests; null when [`text_font`](Self::text_font) was never
    /// called on the active keyboard.
    #[doc(hidden)]
    pub fn current_font_ptr(&self) -> *const c_bindings::lv_font_t {
        // SAFETY: called from the LVGL thread.
        unsafe { KB_STATE.get() }
            .as_ref()
            .map_or(core::ptr::null(), |s| s.font_ptr)
    }

    /// When enabled, every key labelled with the literal text `"Del"` in the
    /// active keymap is rendered as the `⌫` (U+232B) icon instead.
    ///
    /// The label-swap is done by **byte content** (not pointer identity),
    /// so custom keymaps that build their own `c"Del"` string still get the
    /// icon swap. Built-in locale maps use [`KEY_DEL`](super::keyboard_layout::KEY_DEL)
    /// (`c"Del"`), which matches the same bytes.
    ///
    /// The handler in `custom_kb_event_cb` already treats `Del` and `⌫`
    /// identically, so this is purely a label swap. The swap is applied to
    /// the active map on every subsequent `install_lc_map` / `install_uc_map`
    /// / `Keyboard::layout` call (including the implicit one fired by this
    /// method itself when an EnUs lowercase map is already installed).
    ///
    /// Requires the keyboard's font to include a glyph for U+232B — set one
    /// via [`Keyboard::text_font`] before relying on this.
    pub fn del_as_icon(&self, enabled: bool) -> &Self {
        // SAFETY: called from the LVGL thread (UI context).
        let (uppercase, active_layout) = unsafe {
            let state = KB_STATE.get_mut();
            if let Some(state) = state.as_mut() {
                state.del_as_icon = enabled;
                (state.uppercase, state.active_layout)
            } else {
                return self;
            }
        };
        // Reinstall whatever map is currently active so the icon swap (or
        // its removal) takes effect immediately. Re-routing through the
        // *active* layout — rather than always going through the locale's
        // lc/uc map — preserves a [`KeyboardLayout::Custom`] selection.
        match active_layout {
            Some(KeyboardLayout::Locale(locale)) => {
                if uppercase {
                    install_uc_map(self.lv_obj().raw(), locale);
                } else {
                    install_lc_map(self.lv_obj().raw(), locale);
                }
            }
            Some(layout) => {
                // Custom / Qwerty / QwertyUpper / NumberPad / SpecialChars.
                self.layout(layout);
            }
            None => {}
        }
        // The Special-mode map lives in its own slot (independent of the
        // active layout) and is consulted whenever the `123` key toggles
        // to special mode. Reinstall it too so the swap stays in sync.
        install_special_map(self.lv_obj().raw());
        // NOTE: We intentionally do NOT clear `key_label_mirrors` here, even
        // when `enabled == false`. LVGL retains the map pointer for
        // every mode slot we have ever installed, not just the currently
        // active one. Other slots (e.g. a `KeyboardLayout::Custom` map
        // previously installed with transforms active) may still point
        // into a mirror buffer; dropping the buffer would cause a
        // use-after-free the next time that mode is reactivated.
        self
    }

    /// Overrides the visible label of the `Continue` key.
    ///
    /// Passing `Some(label)` rewrites every cell whose text matches
    /// [`KEY_CONTINUE`] in the installed maps (lc/uc per locale, plus the
    /// custom Special map) to display `label` instead. The key continues
    /// to behave as Continue: it still fires `LvEventCode::Ready` (so
    /// [`Keyboard::on_continue`] callbacks fire) and is still recognised
    /// by [`Keyboard::set_continue_enabled`].
    ///
    /// Action-key dispatch is label-based: `custom_kb_event_cb` matches
    /// keys by comparing their visible text. If any other key in the
    /// active map uses the same `label`, the two keys become
    /// indistinguishable — pressing either one fires the Continue handler
    /// (and possibly the colliding action handler too). `continue_label`
    /// rejects collisions only against the small *reserved action-key*
    /// label set (`Del`, `⌫`, `Back`, `ABC`, `abc`, `123`, the
    /// language-toggle labels, and the LVGL built-ins `#@!` / `Ok`)
    /// because those are the ones we know are dispatched as actions.
    /// **Callers are responsible for not picking a label that already
    /// appears in their own custom Special map or as a text key in any
    /// installed layout.** Labels like empty strings or `"\n"` also break
    /// the LVGL map (newline separates rows, empty terminates the
    /// entire map) and
    /// are rejected.
    ///
    /// For these reasons `continue_label` rejects reserved labels: on
    /// debug builds it panics via `debug_assert!`. On non-debug builds it
    /// becomes a no-op (leaving the previous label in place); when
    /// compiled with std (`test` or `no_zephyr`) it also prints a
    /// diagnostic to stderr, but Zephyr/`no_std` release builds are
    /// silent because this crate has no `log` dependency.
    ///
    /// Passing `None` restores the default `Continue` label.
    ///
    /// `label` must be `&'static CStr` because LVGL retains the pointer
    /// and dereferences it on every repaint.
    pub fn continue_label(&self, label: Option<&'static core::ffi::CStr>) -> &Self {
        if let Some(lbl) = label {
            if !is_continue_label_allowed(lbl) {
                debug_assert!(
                    false,
                    "Keyboard::continue_label: label {:?} collides with a \
                     reserved action-key label or breaks the LVGL map",
                    lbl
                );
                #[cfg(any(test, no_zephyr))]
                eprintln!(
                    "[lvgl-dsl] Keyboard::continue_label: rejected label \
                     {:?} (collides with reserved action-key label or breaks LVGL \
                     map); keeping previous label",
                    lbl
                );
                return self;
            }
        }
        let (uppercase, active_layout) = unsafe {
            let state = KB_STATE.get_mut();
            if let Some(state) = state.as_mut() {
                state.continue_label = label;
                (state.uppercase, state.active_layout)
            } else {
                return self;
            }
        };
        match active_layout {
            Some(KeyboardLayout::Locale(locale)) => {
                if uppercase {
                    install_uc_map(self.lv_obj().raw(), locale);
                } else {
                    install_lc_map(self.lv_obj().raw(), locale);
                }
            }
            Some(layout) => {
                self.layout(layout);
            }
            None => {}
        }
        install_special_map(self.lv_obj().raw());
        // See `del_as_icon`: we do NOT clear `key_label_mirrors` when `label`
        // is `None` and `del_as_icon` is also off. LVGL retains map
        // pointers for *every* mode slot we have ever installed, and we
        // only reinstall the currently-active slot (plus Special) here.
        // A previously-installed slot for a different mode (e.g. a
        // custom layout that was active when transforms were enabled)
        // may still dereference its mirror buffer when reactivated.
        self
    }

    // -----------------------------------------------------------------------
    // Per-key styling
    // -----------------------------------------------------------------------

    /// Sets the background colour and corner radius for regular (non-action) keys.
    pub fn key_style_normal(&self, color: Color, radius: impl Into<CornerRadius>) -> &Self {
        // SAFETY: obj is non-null and valid; selector is a valid LVGL part+state combination.
        unsafe {
            c_bindings::lv_obj_set_style_bg_color(
                self.lv_obj().raw(),
                color.to_lv(),
                SELECTOR_KEY_NORMAL,
            );
            c_bindings::lv_obj_set_style_radius(
                self.lv_obj().raw(),
                radius.into().into_lv_value(),
                SELECTOR_KEY_NORMAL,
            );
        }
        self
    }

    /// Sets the background colour for action keys (Enter, Backspace, etc.).
    ///
    /// Targets keys marked with `LV_BUTTONMATRIX_CTRL_CHECKED` in the ctrl map
    /// (LVGL renders them with `LV_STATE_CHECKED`).
    pub fn key_style_action(&self, color: Color) -> &Self {
        // SAFETY: obj is non-null and valid; selector is LV_PART_ITEMS | LV_STATE_CHECKED.
        unsafe {
            c_bindings::lv_obj_set_style_bg_color(
                self.lv_obj().raw(),
                color.to_lv(),
                SELECTOR_KEY_ACTION,
            );
        }
        self
    }

    /// Sets the background colour shown when any key is pressed.
    pub fn key_style_pressed(&self, color: Color) -> &Self {
        // SAFETY: obj is non-null and valid; selector is LV_PART_ITEMS | LV_STATE_PRESSED.
        unsafe {
            c_bindings::lv_obj_set_style_bg_color(
                self.lv_obj().raw(),
                color.to_lv(),
                SELECTOR_KEY_PRESSED,
            );
        }
        self
    }

    // -----------------------------------------------------------------------
    // Theme
    // -----------------------------------------------------------------------

    /// Applies a [`KeyboardTheme`] atomically: background, key colours, radius,
    /// and optionally a font for key labels.
    pub fn theme(&self, t: &KeyboardTheme) -> &Self {
        self.bg_color(Color::hex(t.bg_hex))
            .key_style_normal(
                Color::hex(t.key_normal_hex),
                CornerRadius::Px(t.key_radius_px),
            )
            .key_style_action(Color::hex(t.key_action_hex));
        if let Some(font_fn) = t.font {
            self.text_font(&font_fn());
        }
        self
    }

    // -----------------------------------------------------------------------
    // TextArea binding
    // -----------------------------------------------------------------------

    /// Binds the keyboard to a [`TextArea`] widget.
    ///
    /// Keystrokes from this keyboard are sent to the bound text area.
    /// The text area must outlive the keyboard.
    pub fn bind_textarea(&self, ta: &TextArea) -> &Self {
        // SAFETY: both obj and ta are non-null, valid LVGL objects.
        unsafe {
            c_bindings::lv_keyboard_set_textarea(self.lv_obj().raw(), ta.lv_obj().raw());
        }
        self
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    /// Registers a callback fired when the user presses the Enter / Ok key.
    ///
    /// Corresponds to `LvEventCode::Ready` (LVGL `LV_EVENT_READY`).
    pub fn on_ready(&self, cb: fn(Event)) -> &Self {
        self.on_event(cb, LvEventCode::Ready)
    }

    /// Registers a callback fired when the user presses the Esc / Close key.
    ///
    /// Corresponds to `LvEventCode::Cancel` (LVGL `LV_EVENT_CANCEL`).
    pub fn on_cancel(&self, cb: fn(Event)) -> &Self {
        self.on_event(cb, LvEventCode::Cancel)
    }

    /// Registers a callback fired when the user presses the `Back` key.
    ///
    /// The `Back` key fires `LvEventCode::Cancel`, so this is equivalent to
    /// [`on_cancel`](Keyboard::on_cancel).
    pub fn on_back(&self, cb: fn(Event)) -> &Self {
        self.on_event(cb, LvEventCode::Cancel)
    }

    /// Registers a callback fired when the user presses the `Continue` key.
    ///
    /// The `Continue` key fires `LvEventCode::Ready`, so this is equivalent
    /// to [`on_ready`](Keyboard::on_ready).
    pub fn on_continue(&self, cb: fn(Event)) -> &Self {
        self.on_event(cb, LvEventCode::Ready)
    }

    /// Registers a callback fired when the user presses the `🌐` (language
    /// change) key.
    ///
    /// Unlike `on_back`/`on_continue`, this stores a direct function pointer
    /// in the handler state because there is no dedicated LVGL event code for
    /// language switching.
    pub fn on_lang(&self, cb: fn(Event)) -> &Self {
        // SAFETY: called from the LVGL thread (public API, UI context).
        let state = unsafe { KB_STATE.get_mut() };
        if let Some(state) = state.as_mut() {
            state.on_lang_cb = Some(cb);
        }
        self
    }

    /// Enables or disables the `Continue` key on the active layout.
    ///
    /// When disabled the key:
    /// - Will not fire `LV_EVENT_READY` / its `on_continue` callback.
    /// - Renders in `LV_STATE_DISABLED` (greyed-out background and text by
    ///   default — see [`Keyboard::new`] for the default colours).
    ///
    /// Use this to gate a "next step" CTA on whatever in-screen selection
    /// the consumer requires (e.g. a recipient row being highlighted).
    ///
    /// No-op when the active map has no key labelled `"Continue"` (or no key
    /// labelled with the active override set by [`Keyboard::continue_label`]).
    pub fn set_continue_enabled(&self, enabled: bool) -> &Self {
        let override_label = unsafe { KB_STATE.get() }
            .as_ref()
            .and_then(|st| st.continue_label);
        // Prefer the override label when set: with an active
        // continue_label, the displayed cells were already rewritten to
        // the override string and `KEY_CONTINUE` no longer appears in the
        // map. Fall back to the canonical `Continue` label when no
        // override is installed.
        let idx_opt = override_label
            .and_then(|lbl| self.find_button_index(lbl))
            .or_else(|| self.find_button_index(KEY_CONTINUE));
        let Some(idx) = idx_opt else {
            return self;
        };
        // SAFETY: obj is non-null and valid; idx came from the live map
        // via lv_buttonmatrix_get_button_text.
        unsafe {
            if enabled {
                c_bindings::lv_buttonmatrix_clear_button_ctrl(
                    self.lv_obj().raw(),
                    idx,
                    CTRL_DISABLED,
                );
            } else {
                c_bindings::lv_buttonmatrix_set_button_ctrl(
                    self.lv_obj().raw(),
                    idx,
                    CTRL_DISABLED,
                );
            }
        }
        self
    }

    /// Returns the button-matrix index of the first key whose label matches
    /// `target`, or `None` if no such key is present in the active map.
    ///
    /// Indices are logical (excluding `\n` row separators), as expected by
    /// `lv_buttonmatrix_set_button_ctrl` and friends.
    fn find_button_index(&self, target: &core::ffi::CStr) -> Option<u32> {
        // The loop terminates on the first null returned by
        // `lv_buttonmatrix_get_button_text` (which signals an out-of-range
        // index), so this upper bound is purely defensive against a
        // pathological LVGL version that never returns null. `MAX_BUTTONS`
        // is generously larger than any keymap we ship (the biggest is
        // QWERTY ≈ 40 keys including row separators).
        const MAX_BUTTONS: u32 = 1024;
        for idx in 0..MAX_BUTTONS {
            // SAFETY: obj is non-null; lv_buttonmatrix_get_button_text is
            // safe to call with any index and returns null for invalid ids.
            let txt_ptr = unsafe {
                c_bindings::lv_buttonmatrix_get_button_text(
                    self.lv_obj().raw() as *const _,
                    idx,
                )
            };
            if txt_ptr.is_null() {
                return None;
            }
            // SAFETY: lv_buttonmatrix_get_button_text returns a pointer to
            // a NUL-terminated C string owned by the keyboard's map.
            let txt = unsafe { core::ffi::CStr::from_ptr(txt_ptr) };
            if txt == target {
                return Some(idx);
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Visibility
    // -----------------------------------------------------------------------

    /// Makes the keyboard visible.
    ///
    /// Clears `LvObjFlag::HIDDEN`.
    pub fn show(&self) -> &Self {
        self.remove_flag(LvObjFlag::HIDDEN)
    }

    /// Hides the keyboard.
    ///
    /// Sets `LvObjFlag::HIDDEN`.  The keyboard remains in memory and can be
    /// shown again with [`show`](Keyboard::show).
    pub fn hide(&self) -> &Self {
        #[cfg(not(test))]
        dismiss_accent_popup();
        self.add_flag(LvObjFlag::HIDDEN)
    }

    // -----------------------------------------------------------------------
    // Slide animations
    // -----------------------------------------------------------------------

    /// Slides the keyboard in from the bottom of the screen (300 ms, ease-out).
    ///
    /// Clears `HIDDEN`, cancels any in-flight slide-hide animation (preventing
    /// the completion callback from re-hiding the widget), snaps the widget to
    /// `y = display_height` (off-screen), then animates y upward to its docked
    /// position.
    pub fn slide_show(&self) -> &Self {
        #[cfg(not(test))]
        dismiss_accent_popup();
        self.remove_flag(LvObjFlag::HIDDEN);
        // SAFETY: raw pointers are valid for the lifetime of `self`.
        unsafe {
            let obj = self.lv_obj().raw();
            let kb_h = c_bindings::lv_obj_get_height(obj);

            // Clear the pending-hide flag so that on_slide_hide_done (fired by
            // any in-flight hide animation) does not re-hide the widget after
            // this show call.  LVGL's lv_anim_start automatically replaces any
            // existing animation for the same var+exec_cb pair, so no explicit
            // cancel call is needed.
            SLIDE_HIDE_PENDING.store(core::ptr::null_mut(), Ordering::Relaxed);

            // Snap translate_y to +kb_h (keyboard sits just below the screen)
            // before starting the animation so every call begins from the same
            // off-screen position regardless of prior translate state.
            c_bindings::lv_obj_set_style_translate_y(obj, kb_h, 0);

            // Animate translate_y: kb_h (off-screen below) → 0 (docked)
            Anim::new(obj as *mut core::ffi::c_void)
                .values(kb_h, 0)
                .duration_ms(300)
                .path(AnimPath::EaseOut)
                .exec_extern(slide_exec_y)
                .start_detached();
        }
        self
    }

    /// Slides the keyboard off the bottom of the screen (250 ms, ease-in).
    ///
    /// Animates y from the docked position to `display_height`.  `HIDDEN` is
    /// applied by [`on_slide_hide_done`] after the animation completes so that
    /// off-screen touch events are blocked.
    pub fn slide_hide(&self) -> &Self {
        #[cfg(not(test))]
        dismiss_accent_popup();
        // SAFETY: raw pointers are valid for the lifetime of `self`.
        unsafe {
            let obj = self.lv_obj().raw();
            let kb_h = c_bindings::lv_obj_get_height(obj);

            SLIDE_HIDE_PENDING.store(obj, Ordering::Relaxed);

            // Animate translate_y: 0 (docked) → kb_h (off-screen below)
            Anim::new(obj as *mut core::ffi::c_void)
                .values(0, kb_h)
                .duration_ms(250)
                .path(AnimPath::EaseIn)
                .exec_extern(slide_exec_y)
                .completed_extern(on_slide_hide_done)
                .start_detached();
        }
        self
    }

    // -----------------------------------------------------------------------
    // Pop-over key previews
    // -----------------------------------------------------------------------

    /// Enables or disables pop-over key previews on press.
    ///
    /// When `true`, a small label appears above the pressed key showing its
    /// value — useful as a visual affordance on touch screens.
    ///
    /// **Note:** enabling popovers is silently ignored when the active layout
    /// has no ctrl map (e.g. [`KeyboardLayout::Custom`]) because LVGL would
    /// dereference a null ctrl-map pointer.
    pub fn popover_keys(&self, enabled: bool) -> &Self {
        if enabled {
            // SAFETY: called from the LVGL thread (public API, UI context).
            let blocked = unsafe { KB_STATE.get() }
                .as_ref()
                .map_or(false, |st| !st.ctrl_map_installed);
            if blocked {
                return self;
            }
        }
        // SAFETY: obj is non-null and valid for the lifetime of this widget.
        unsafe { c_bindings::lv_keyboard_set_popovers(self.lv_obj().raw(), enabled) }
        self
    }

    // -----------------------------------------------------------------------
    // Locale preloading
    // -----------------------------------------------------------------------

    /// Pre-installs the custom lowercase maps for each language locale in
    /// `locales` into their dedicated LVGL User slots without changing the
    /// active mode.
    ///
    /// Call this once after construction so that subsequent
    /// [`locale()`](Keyboard::locale) calls only need to switch the map
    /// (for the current locale) rather than install from scratch.
    ///
    /// Locales without a custom map (`Numeric`) are silently skipped.
    pub fn preload_locale_maps(&self, locales: &[KeyboardLocale]) -> &Self {
        for &locale in locales {
            if let Some((map, ctrl)) = locale.maps() {
                unsafe {
                    c_bindings::lv_keyboard_set_map(
                        self.lv_obj().raw(),
                        locale.native_mode(),
                        map.as_ptr() as *const *const core::ffi::c_char,
                        ctrl.as_ptr(),
                    );
                }
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{KEY_LABEL_MIRROR_LEN, install_key_label_mirror};
    use crate::c_bindings::{LvCall, reset_obj_pool, spy_drain};
    use crate::lvgl::LvAlign;
    use crate::lvgl::event::LvEventCode;
    use crate::lvgl::keyboard::Keyboard;
    use crate::lvgl::keyboard_layout::{
        KEYMAP_NUMPAD, KEYMAP_QWERTY_EN_LC, KeyboardLayout, KeyboardLocale, LocaleSwitcher,
        LvKeyboardMode,
    };
    use crate::lvgl::keyboard_theme::KeyboardTheme;
    use crate::lvgl::screen::Screen;
    use crate::lvgl::textarea::TextArea;
    use core::ffi::c_char;

    fn setup() -> KbTestScreen {
        // Acquire the serialization lock first; it is held for as long as the
        // returned `KbTestScreen` lives (i.e. the whole test body), then
        // released on drop — including on panic via unwinding.
        let guard = lock_keyboard_test();
        reset_obj_pool();
        // Clear the process-global keyboard handler state and any in-flight
        // slide-hide pointer left over from a previous test. `KB_STATE` and
        // `SLIDE_HIDE_PENDING` are single-thread-only globals (see `LvglCell`);
        // with the test serialized by `guard`, resetting them here gives every
        // test a clean slate and drops the previous `KbHandlerState` (Boxed
        // closures and label mirrors) deterministically.
        unsafe {
            *super::KB_STATE.get_mut() = None;
        }
        super::SLIDE_HIDE_PENDING
            .store(core::ptr::null_mut(), core::sync::atomic::Ordering::Relaxed);
        KbTestScreen {
            screen: Screen::active(),
            _guard: guard,
        }
    }

    /// RAII handle returned by [`setup`]. It bundles the active [`Screen`] with
    /// the keyboard-test serialization guard so the guard is released exactly
    /// when the test's `screen` binding goes out of scope (or the test panics).
    ///
    /// It implements [`Widget`] by delegating to the inner `Screen`, so tests
    /// can keep using `&screen` as a parent (`Keyboard::new(&screen)`, …)
    /// without any call-site changes.
    struct KbTestScreen {
        screen: Screen,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl crate::lvgl::widget::Widget for KbTestScreen {
        fn lv_obj(&self) -> &crate::lvgl::widget::LvObj {
            self.screen.lv_obj()
        }
    }

    /// Serialize keyboard tests so they never touch the process-global
    /// `KB_STATE` / `SLIDE_HIDE_PENDING` concurrently.
    ///
    /// The keyboard module deliberately stores handler state in a plain
    /// `static` (`LvglCell`) because production LVGL is single-threaded —
    /// concurrent access is documented as undefined behaviour. Cargo's test
    /// harness, however, runs tests on multiple threads, which races on that
    /// shared state (manifesting as flaky assertion failures or SIGSEGV/SIGABRT
    /// from dropping a `KbHandlerState` aliased across threads).
    ///
    /// `setup()` calls this and binds the returned guard inside the
    /// [`KbTestScreen`] it hands back, so at most one keyboard test runs at a
    /// time (regardless of `--test-threads`) and the lock is always released
    /// when the test ends — never leaked onto a worker thread that goes on to
    /// run unrelated tests.
    fn lock_keyboard_test() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::Mutex;
        static KB_TEST_LOCK: Mutex<()> = Mutex::new(());
        // A panicking keyboard test poisons the mutex; recover the guard so the
        // remaining keyboard tests still serialize instead of all erroring out.
        KB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    #[test]
    fn new_emits_create_call() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::KeyboardCreate { .. })),
            "expected KeyboardCreate, got: {calls:?}"
        );
        drop(kb);
    }

    // -----------------------------------------------------------------------
    // Sizing & positioning
    // -----------------------------------------------------------------------

    #[test]
    fn full_width_aligns_bottom_mid() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.full_width();
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::Align { align, .. }
                if *align == LvAlign::BottomMid as u32
            )),
            "expected BottomMid align, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Layout
    // -----------------------------------------------------------------------

    #[test]
    fn layout_qwerty_sets_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.layout(KeyboardLayout::Qwerty);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::KeyboardSetMode { mode, .. }
                if *mode == LvKeyboardMode::TextLower as u32
            )),
            "expected TextLower mode, got: {calls:?}"
        );
    }

    #[test]
    fn layout_custom_calls_set_map_then_set_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.layout(KeyboardLayout::Custom(KEYMAP_NUMPAD));
        let calls = spy_drain();
        let has_map = calls.iter().any(|c| {
            matches!(
                c,
                LvCall::KeyboardSetMap { mode, .. }
                if *mode == LvKeyboardMode::User1 as u32
            )
        });
        let has_mode = calls.iter().any(|c| {
            matches!(
                c,
                LvCall::KeyboardSetMode { mode, .. }
                if *mode == LvKeyboardMode::User1 as u32
            )
        });
        assert!(has_map, "expected KeyboardSetMap, got: {calls:?}");
        assert!(has_mode, "expected KeyboardSetMode, got: {calls:?}");
    }

    #[test]
    fn locale_en_us_sets_text_lower() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.locale(KeyboardLocale::EnUs);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::KeyboardSetMode { mode, .. }
                if *mode == LvKeyboardMode::TextLower as u32
            )),
            "expected TextLower for EnUs, got: {calls:?}"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::KeyboardSetMap { .. })),
            "EnUs now installs a custom map, got: {calls:?}"
        );
    }

    #[test]
    fn custom_map_convenience() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.custom_map(KEYMAP_NUMPAD);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::KeyboardSetMap { .. })),
            "expected KeyboardSetMap, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // TextArea binding
    // -----------------------------------------------------------------------

    #[test]
    fn bind_textarea_emits_spy() {
        let screen = setup();
        let ta = TextArea::new(&screen);
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.bind_textarea(&ta);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::KeyboardSetTextarea { .. })),
            "expected KeyboardSetTextarea, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[test]
    fn on_ready_registers_event() {
        fn noop(_: crate::lvgl::event::Event) {}
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.on_ready(noop);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::AddEventCb { code, .. }
                if *code == LvEventCode::Ready as u32
            )),
            "expected AddEventCb with Ready code, got: {calls:?}"
        );
    }

    #[test]
    fn on_cancel_registers_event() {
        fn noop(_: crate::lvgl::event::Event) {}
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.on_cancel(noop);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::AddEventCb { code, .. }
                if *code == LvEventCode::Cancel as u32
            )),
            "expected AddEventCb with Cancel code, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Visibility
    // -----------------------------------------------------------------------

    #[test]
    fn hide_adds_hidden_flag() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.hide();
        let calls = spy_drain();
        use crate::lvgl::state::LvObjFlag;
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::AddFlag { flag, .. }
                if *flag == LvObjFlag::HIDDEN.0
            )),
            "expected AddFlag(HIDDEN), got: {calls:?}"
        );
    }

    #[test]
    fn show_removes_hidden_flag() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.show();
        let calls = spy_drain();
        use crate::lvgl::state::LvObjFlag;
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::RemoveFlag { flag, .. }
                if *flag == LvObjFlag::HIDDEN.0
            )),
            "expected RemoveFlag(HIDDEN), got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Continue-key enable/disable
    // -----------------------------------------------------------------------

    #[test]
    fn set_continue_enabled_false_sets_disabled_ctrl_on_continue_key() {
        use crate::lvgl::keyboard_layout::CTRL_DISABLED;
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // Install a layout that contains a `Continue` key.
        kb.layout(KeyboardLayout::Locale(KeyboardLocale::EnUs));
        spy_drain();
        kb.set_continue_enabled(false);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonCtrl { ctrl, .. }
                if *ctrl == CTRL_DISABLED
            )),
            "expected ButtonMatrixSetButtonCtrl(CTRL_DISABLED), got: {calls:?}"
        );
    }

    #[test]
    fn set_continue_enabled_true_clears_disabled_ctrl_on_continue_key() {
        use crate::lvgl::keyboard_layout::CTRL_DISABLED;
        let screen = setup();
        let kb = Keyboard::new(&screen);
        kb.layout(KeyboardLayout::Locale(KeyboardLocale::EnUs));
        spy_drain();
        kb.set_continue_enabled(true);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixClearButtonCtrl { ctrl, .. }
                if *ctrl == CTRL_DISABLED
            )),
            "expected ButtonMatrixClearButtonCtrl(CTRL_DISABLED), got: {calls:?}"
        );
    }

    #[test]
    fn set_continue_enabled_is_noop_for_layout_without_continue_key() {
        use crate::lvgl::keyboard_layout::{CTRL_DISABLED, KEYMAP_NUMPAD};
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // The numpad custom map ships with no `Continue` key.
        kb.layout(KeyboardLayout::Custom(KEYMAP_NUMPAD));
        spy_drain();
        kb.set_continue_enabled(false);
        let calls = spy_drain();
        assert!(
            !calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonCtrl { ctrl, .. }
                if *ctrl == CTRL_DISABLED
            )),
            "expected no DISABLED ctrl set, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Popovers
    // -----------------------------------------------------------------------

    #[test]
    fn popover_keys_emits_spy() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.popover_keys(true);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ButtonMatrixSetPopovers { en: true, .. })),
            "expected ButtonMatrixSetPopovers(true), got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Theme
    // -----------------------------------------------------------------------

    #[test]
    fn theme_applies_without_panic() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.theme(&KeyboardTheme::DARK);
        // No assertion — just ensure no panic and calls were emitted.
        assert!(!spy_drain().is_empty(), "theme() should emit C calls");
    }

    // -----------------------------------------------------------------------
    // Builder chaining
    // -----------------------------------------------------------------------

    #[test]
    fn full_builder_chain_compiles() {
        fn on_ready(_: crate::lvgl::event::Event) {}
        let screen = setup();
        let ta = TextArea::new(&screen);
        let kb = Keyboard::new(&screen);
        kb.full_width()
            .layout(KeyboardLayout::Qwerty)
            .theme(&KeyboardTheme::DARK)
            .popover_keys(true)
            .bind_textarea(&ta)
            .on_ready(on_ready);
    }

    // -----------------------------------------------------------------------
    // Locale — dedicated User-slot routing
    // -----------------------------------------------------------------------

    #[test]
    fn locale_de_installs_user2_map_and_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.locale(KeyboardLocale::De);
        let calls = spy_drain();
        let has_map = calls.iter().any(|c| {
            matches!(
                c,
                LvCall::KeyboardSetMap { mode, .. }
                if *mode == LvKeyboardMode::User2 as u32
            )
        });
        let has_mode = calls.iter().any(|c| {
            matches!(
                c,
                LvCall::KeyboardSetMode { mode, .. }
                if *mode == LvKeyboardMode::User2 as u32
            )
        });
        assert!(
            has_map,
            "expected KeyboardSetMap for De (User2), got: {calls:?}"
        );
        assert!(
            has_mode,
            "expected KeyboardSetMode for De (User2), got: {calls:?}"
        );
    }

    #[test]
    fn locale_fr_installs_user3_map_and_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.locale(KeyboardLocale::Fr);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::KeyboardSetMap { mode, .. }
                if *mode == LvKeyboardMode::User3 as u32
            )),
            "expected KeyboardSetMap for Fr (User3), got: {calls:?}"
        );
    }

    #[test]
    fn locale_it_installs_user4_map_and_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.locale(KeyboardLocale::It);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::KeyboardSetMap { mode, .. }
                if *mode == LvKeyboardMode::User4 as u32
            )),
            "expected KeyboardSetMap for It (User4), got: {calls:?}"
        );
    }

    #[test]
    fn locale_en_us_installs_map_and_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.locale(KeyboardLocale::EnUs);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::KeyboardSetMap { .. })),
            "EnUs now installs a custom map, got: {calls:?}"
        );
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::KeyboardSetMode { mode, .. }
                if *mode == LvKeyboardMode::TextLower as u32
            )),
            "expected KeyboardSetMode(TextLower) for EnUs, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // preload_locale_maps
    // -----------------------------------------------------------------------

    #[test]
    fn preload_locale_maps_installs_de_fr_it_slots() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.preload_locale_maps(&[KeyboardLocale::De, KeyboardLocale::Fr, KeyboardLocale::It]);
        let calls = spy_drain();
        let modes: Vec<u32> = calls
            .iter()
            .filter_map(|c| {
                if let LvCall::KeyboardSetMap { mode, .. } = c {
                    Some(*mode)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            modes.contains(&(LvKeyboardMode::User2 as u32)),
            "missing User2 in {modes:?}"
        );
        assert!(
            modes.contains(&(LvKeyboardMode::User3 as u32)),
            "missing User3 in {modes:?}"
        );
        assert!(
            modes.contains(&(LvKeyboardMode::User4 as u32)),
            "missing User4 in {modes:?}"
        );
        // Must NOT emit KeyboardSetMode — preload only installs, does not switch
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, LvCall::KeyboardSetMode { .. })),
            "preload must not change active mode, got: {calls:?}"
        );
    }

    #[test]
    fn preload_locale_maps_skips_numeric() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.preload_locale_maps(&[KeyboardLocale::Numeric]);
        let calls = spy_drain();
        assert!(
            calls.is_empty(),
            "Numeric locale must not emit any C calls, got: {calls:?}"
        );
    }

    #[test]
    fn locale_switcher_drives_keyboard_cycle() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        let mut sw =
            LocaleSwitcher::new([KeyboardLocale::EnUs, KeyboardLocale::De, KeyboardLocale::Fr]);
        spy_drain();
        // Cycle once: EnUs → De → Fr → EnUs
        kb.locale(sw.next()); // De
        kb.locale(sw.next()); // Fr
        kb.locale(sw.next()); // EnUs (wrap)
        assert_eq!(sw.current(), KeyboardLocale::EnUs);
    }

    // -----------------------------------------------------------------------
    // Regression: locale(Numeric) must switch to Number mode (was a no-op)
    // -----------------------------------------------------------------------

    #[test]
    fn locale_numeric_sets_number_mode() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();
        kb.locale(KeyboardLocale::Numeric);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::KeyboardSetMode { mode, .. }
                if *mode == LvKeyboardMode::Number as u32
            )),
            "locale(Numeric) must emit KeyboardSetMode(Number), got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: ctrl_map_installed reset when switching from Custom to built-in
    // -----------------------------------------------------------------------

    #[test]
    fn builtin_layout_after_custom_re_enables_popovers() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // Custom layout without ctrl map — disables popovers
        kb.layout(KeyboardLayout::Custom(KEYMAP_QWERTY_EN_LC));
        spy_drain();
        // Switch to a built-in layout — ctrl_map_installed must be reset to true
        kb.layout(KeyboardLayout::Qwerty);
        spy_drain();
        // Now popover_keys(true) should actually emit the call
        kb.popover_keys(true);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ButtonMatrixSetPopovers { en: true, .. })),
            "popover_keys(true) should be allowed after switching back to built-in layout, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: ctrl_map_installed restored after locale() following Custom
    // -----------------------------------------------------------------------

    #[test]
    fn locale_after_custom_layout_re_enables_popovers() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // Set a Custom layout (no ctrl map)
        kb.layout(KeyboardLayout::Custom(KEYMAP_QWERTY_EN_LC));
        spy_drain();
        // Switch via locale() — must set ctrl_map_installed = true
        kb.locale(KeyboardLocale::EnUs);
        spy_drain();
        kb.popover_keys(true);
        let calls = spy_drain();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, LvCall::ButtonMatrixSetPopovers { en: true, .. })),
            "popover_keys(true) should work after locale() following a Custom layout, got: {calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: CTRL_CHECKED is re-exported from the public API surface
    // -----------------------------------------------------------------------

    #[test]
    fn ctrl_checked_is_accessible_from_public_api() {
        // This is a compile-time test — if CTRL_CHECKED is not exported, it won't compile.
        use crate::lvgl::{CTRL_CHECKED, CTRL_W2};
        let val: u32 = CTRL_W2 | CTRL_CHECKED;
        assert_eq!(val, 2 | 0x0100);
    }

    // -----------------------------------------------------------------------
    // Regression: CtrlMap is [u32] — verify constant types match
    // -----------------------------------------------------------------------

    #[test]
    fn ctrl_constants_are_u32() {
        use crate::lvgl::{CTRL_CHECKED, CTRL_HIDDEN, CTRL_SPACE_W, CTRL_SPACER, CTRL_W1};
        // Type annotations below enforce u32 — if any constant is u16 this won't compile.
        let _h: u32 = CTRL_HIDDEN;
        let _w: u32 = CTRL_W1;
        let _s: u32 = CTRL_SPACE_W;
        let _c: u32 = CTRL_CHECKED;
        let _sp: u32 = CTRL_SPACER;
    }

    // -----------------------------------------------------------------------
    // Regression: slide_show cancels in-flight slide_hide (animation race)
    // -----------------------------------------------------------------------

    #[test]
    fn slide_show_clears_pending_hide() {
        use core::sync::atomic::Ordering;

        let screen = setup();
        let kb = Keyboard::new(&screen);
        spy_drain();

        // Simulate a hide in flight by storing a sentinel non-null pointer.
        let sentinel = 1usize as *mut crate::c_bindings::lv_obj_t;
        super::SLIDE_HIDE_PENDING.store(sentinel, Ordering::Relaxed);

        // slide_show must clear SLIDE_HIDE_PENDING so on_slide_hide_done is a no-op.
        kb.slide_show();

        assert!(
            super::SLIDE_HIDE_PENDING.load(Ordering::Relaxed).is_null(),
            "slide_show must clear SLIDE_HIDE_PENDING to prevent re-hide after rapid hide→show"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: layout(Locale(x)) must sync KB_STATE.locale and uppercase
    // so that the ABC/abc shift toggle installs the correct map afterwards.
    // -----------------------------------------------------------------------

    #[test]
    fn layout_locale_syncs_state_locale() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // Start with De locale so state.locale is De.
        kb.locale(KeyboardLocale::De);
        spy_drain();

        // Switch via layout(Locale(EnUs)) — state.locale must update to EnUs.
        // The observable side-effect: a subsequent locale(EnUs) call should be
        // a no-op for locale state (already EnUs). We verify the state is
        // consistent by checking a fresh locale() call doesn't cause a second
        // unexpected lv_keyboard_set_mode for De mode.
        kb.layout(KeyboardLayout::Locale(KeyboardLocale::EnUs));
        spy_drain();

        // Call locale() again for EnUs — should only see one lv_keyboard_set_mode
        // (for TextLower / mode 0), never for De's User2 mode.
        kb.locale(KeyboardLocale::EnUs);
        let calls = spy_drain();
        let mode_calls: Vec<u32> = calls
            .iter()
            .filter_map(|c| {
                if let LvCall::KeyboardSetMode { mode, .. } = c {
                    Some(*mode)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            mode_calls
                .iter()
                .all(|&m| m != crate::lvgl::keyboard_layout::LvKeyboardMode::User2 as u32),
            "after layout(Locale(EnUs)), locale(EnUs) must not install De maps; got modes: {mode_calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: layout(Locale(Numeric)) (no custom map, else branch) must
    // still update KB_STATE.locale — previously skipped because maps() → None.
    // -----------------------------------------------------------------------

    #[test]
    fn layout_locale_numeric_syncs_state() {
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // Put KB_STATE in De locale first.
        kb.locale(KeyboardLocale::De);
        spy_drain();

        // Switch to Numeric via layout — this takes the else branch (no custom map).
        kb.layout(KeyboardLayout::Locale(KeyboardLocale::Numeric));
        spy_drain();

        // A follow-up locale(Numeric) should emit Number mode, not De's User2 mode.
        kb.locale(KeyboardLocale::Numeric);
        let calls = spy_drain();
        let mode_calls: Vec<u32> = calls
            .iter()
            .filter_map(|c| {
                if let LvCall::KeyboardSetMode { mode, .. } = c {
                    Some(*mode)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            mode_calls
                .iter()
                .all(|&m| m != crate::lvgl::keyboard_layout::LvKeyboardMode::User2 as u32),
            "layout(Locale(Numeric)) must update state.locale; locale(Numeric) must not emit De mode; got: {mode_calls:?}"
        );
        assert!(
            mode_calls
                .iter()
                .any(|&m| m == crate::lvgl::keyboard_layout::LvKeyboardMode::Number as u32),
            "locale(Numeric) must emit Number mode; got: {mode_calls:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Bug 1: long-press popup must retract the just-inserted base char.
    // -----------------------------------------------------------------------

    #[test]
    fn should_undo_base_char_after_popup_true_for_accent_keys() {
        // "a" has accent variants — the popup will open, so the base char
        // inserted on PRESS must be retracted.
        assert!(super::should_undo_base_char_after_popup(c"a"));
        assert!(super::should_undo_base_char_after_popup(c"e"));
    }

    #[test]
    fn should_undo_base_char_after_popup_false_for_non_accent_keys() {
        // No popup → nothing to retract. The action key labels are not
        // accent-eligible either.
        assert!(!super::should_undo_base_char_after_popup(c"q"));
        assert!(!super::should_undo_base_char_after_popup(c"Del"));
        assert!(!super::should_undo_base_char_after_popup(c"Continue"));
    }

    // -----------------------------------------------------------------------
    // Bug 2: text_font must record the font pointer so accent_long_press_cb
    // can style the popup with a font that covers accent ranges.
    // -----------------------------------------------------------------------

    #[test]
    fn text_font_records_font_ptr_for_popup_styling() {
        use crate::lvgl::Font;
        let screen = setup();
        let kb = Keyboard::new(&screen);
        assert!(
            kb.current_font_ptr().is_null(),
            "fresh keyboard must have null font_ptr (popup falls back to default)"
        );

        // Construct a Font whose backing static address we can compare to.
        // Any valid `lv_font_t` symbol works — Montserrat 14 is always present.
        let font = Font::montserrat_14();
        let expected = font.as_ptr();
        kb.text_font(&font);
        assert_eq!(
            kb.current_font_ptr(),
            expected,
            "text_font() must store the font pointer in KB_STATE"
        );
    }

    // -----------------------------------------------------------------------
    // Bug 3a: del_as_icon swaps "Del" cells for "⌫" in the installed map.
    // -----------------------------------------------------------------------

    #[test]
    fn del_as_icon_swaps_del_cells_for_backspace_in_active_map() {
        use crate::c_bindings::lv_buttonmatrix_get_button_text;
        use crate::lvgl::keyboard_layout::{KEY_BACKSPACE, KEY_DEL};
        use crate::lvgl::widget::Widget;
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // EnUs lowercase map is installed in Keyboard::new — it contains a "Del" cell.
        kb.del_as_icon(true);
        spy_drain();

        let raw = kb.lv_obj().raw() as *const _;
        let mut found_backspace = false;
        let mut found_del = false;
        for btn_id in 0..64u32 {
            let p = unsafe { lv_buttonmatrix_get_button_text(raw, btn_id) };
            if p.is_null() {
                break;
            }
            let bytes = unsafe { core::ffi::CStr::from_ptr(p) }.to_bytes();
            if bytes == KEY_DEL.to_bytes() {
                found_del = true;
            }
            if bytes == KEY_BACKSPACE.to_bytes() {
                found_backspace = true;
            }
        }
        assert!(
            !found_del,
            "after del_as_icon(true), no cell may still point at \"Del\""
        );
        assert!(
            found_backspace,
            "after del_as_icon(true), at least one cell must point at \"⌫\""
        );
    }

    /// Test-only helper: a stand-alone `KbHandlerState` we can pass to
    /// `install_key_label_mirror` without standing up a real keyboard.
    fn fresh_test_state() -> super::KbHandlerState {
        super::KbHandlerState {
            obj: core::ptr::null_mut(),
            locale: crate::lvgl::keyboard_layout::KeyboardLocale::EnUs,
            uppercase: false,
            on_lang_cb: None,
            accent_popup: core::ptr::null_mut(),
            ctrl_map_installed: true,
            font_ptr: core::ptr::null(),
            del_as_icon: true,
            key_label_mirrors: alloc::collections::BTreeMap::new(),
            continue_label: None,
            active_layout: None,
        }
    }

    #[test]
    fn install_key_label_mirror_falls_back_when_keymap_too_large() {
        let oversized = KEY_LABEL_MIRROR_LEN + 8;
        let owned: alloc::vec::Vec<alloc::ffi::CString> = (0..oversized)
            .map(|i| alloc::ffi::CString::new(alloc::format!("k{i}")).unwrap())
            .collect();
        let mut raw: alloc::vec::Vec<*const c_char> =
            owned.iter().map(|s| s.as_ptr()).collect();
        raw.push(core::ptr::null());

        let mut state = fresh_test_state();
        let src = raw.as_ptr();
        let out = unsafe { install_key_label_mirror(&mut state, 0, src) };

        assert_eq!(
            out, src,
            "oversized keymap must fall back to the unswapped source pointer \
             instead of writing past the mirror"
        );
        assert!(
            state.key_label_mirrors.is_empty(),
            "fallback path must not allocate a mirror entry"
        );
    }

    #[test]
    fn install_key_label_mirror_swaps_by_bytes_not_by_pointer_identity() {
        use crate::lvgl::keyboard_layout::KEY_BACKSPACE;
        let custom_del = alloc::ffi::CString::new("Del").unwrap();
        let custom_a = alloc::ffi::CString::new("a").unwrap();
        let raw: alloc::vec::Vec<*const c_char> = alloc::vec![
            custom_a.as_ptr(),
            custom_del.as_ptr(),
            core::ptr::null(),
        ];
        assert_ne!(
            custom_del.as_ptr(),
            crate::lvgl::keyboard_layout::KEY_DEL.as_ptr(),
            "test precondition: custom CString must have a distinct pointer"
        );

        let mut state = fresh_test_state();
        let out = unsafe { install_key_label_mirror(&mut state, 0, raw.as_ptr()) };
        let swapped_cell = unsafe { *out.add(1) };
        assert_eq!(
            swapped_cell,
            KEY_BACKSPACE.as_ptr(),
            "swap must match by bytes (b\"Del\"), not by pointer identity"
        );
    }

    #[test]
    fn install_key_label_mirror_normalises_null_terminator_to_empty_cstr() {
        // A consumer who terminates a custom keymap with a null pointer
        // (instead of LVGL's expected `c""` sentinel) must still produce a
        // map whose terminator is a non-null pointer to an empty C string —
        // otherwise lv_keyboard_set_map would dereference null.
        let custom_a = alloc::ffi::CString::new("a").unwrap();
        let raw: alloc::vec::Vec<*const c_char> =
            alloc::vec![custom_a.as_ptr(), core::ptr::null()];

        let mut state = fresh_test_state();
        let out = unsafe { install_key_label_mirror(&mut state, 0, raw.as_ptr()) };
        let terminator = unsafe { *out.add(1) };
        assert!(
            !terminator.is_null(),
            "null terminator in source must be normalised to a non-null pointer"
        );
        let term_bytes = unsafe { core::ffi::CStr::from_ptr(terminator) }.to_bytes();
        assert!(
            term_bytes.is_empty(),
            "normalised terminator must point at an empty C string (c\"\")"
        );
    }

    #[test]
    fn install_key_label_mirror_uses_distinct_buffers_per_mode() {
        // Installing into two different modes must produce two distinct
        // stable pointers: LVGL retains both, so sharing a single buffer
        // would let the second install silently rewrite the first.
        let a = alloc::ffi::CString::new("a").unwrap();
        let b = alloc::ffi::CString::new("b").unwrap();
        let raw_a: alloc::vec::Vec<*const c_char> =
            alloc::vec![a.as_ptr(), core::ptr::null()];
        let raw_b: alloc::vec::Vec<*const c_char> =
            alloc::vec![b.as_ptr(), core::ptr::null()];

        let mut state = fresh_test_state();
        let out_mode_0 = unsafe { install_key_label_mirror(&mut state, 0, raw_a.as_ptr()) };
        let out_mode_1 = unsafe { install_key_label_mirror(&mut state, 1, raw_b.as_ptr()) };

        assert_ne!(
            out_mode_0, out_mode_1,
            "each mode must get its own stable mirror pointer"
        );
        // After the second install, the first mode's mirror must still
        // contain the first map's bytes (not have been clobbered).
        let cell_mode_0 = unsafe { *out_mode_0 };
        let bytes_mode_0 = unsafe { core::ffi::CStr::from_ptr(cell_mode_0) }.to_bytes();
        assert_eq!(
            bytes_mode_0, b"a",
            "second install into a different mode must not corrupt the first"
        );
    }

    // -----------------------------------------------------------------------
    // continue_label override: visible label, event matching, isolation
    // -----------------------------------------------------------------------

    #[test]
    fn continue_label_override_swaps_continue_cells_only() {
        use crate::lvgl::keyboard_layout::{KEY_BACKSPACE, KEY_CONTINUE, KEY_DEL};
        // Map: ["a", "Continue", "Del", terminator]. With del_as_icon = false
        // and continue_label = Some("Apply"), only the Continue cell must be
        // rewritten — the "Del" cell must remain unswapped.
        let custom_a = alloc::ffi::CString::new("a").unwrap();
        let raw: alloc::vec::Vec<*const c_char> = alloc::vec![
            custom_a.as_ptr(),
            KEY_CONTINUE.as_ptr(),
            KEY_DEL.as_ptr(),
            core::ptr::null(),
        ];

        let mut state = fresh_test_state();
        state.del_as_icon = false;
        state.continue_label = Some(c"Apply");

        let out = unsafe { install_key_label_mirror(&mut state, 0, raw.as_ptr()) };

        let continue_cell = unsafe { *out.add(1) };
        let del_cell = unsafe { *out.add(2) };
        let continue_bytes = unsafe { core::ffi::CStr::from_ptr(continue_cell) }.to_bytes();
        assert_eq!(
            continue_bytes, b"Apply",
            "Continue cell must display the override label"
        );
        assert_ne!(
            del_cell,
            KEY_BACKSPACE.as_ptr(),
            "Del cell must NOT be swapped to ⌫ when del_as_icon is false \
             (continue_label must not enable del-as-icon as a side effect)"
        );
        assert_eq!(
            del_cell,
            KEY_DEL.as_ptr(),
            "Del cell must remain the original KEY_DEL pointer when del_as_icon is false"
        );
    }

    #[test]
    fn continue_label_and_del_as_icon_both_swap_when_both_enabled() {
        use crate::lvgl::keyboard_layout::{KEY_BACKSPACE, KEY_CONTINUE, KEY_DEL};
        let custom_a = alloc::ffi::CString::new("a").unwrap();
        let raw: alloc::vec::Vec<*const c_char> = alloc::vec![
            custom_a.as_ptr(),
            KEY_CONTINUE.as_ptr(),
            KEY_DEL.as_ptr(),
            core::ptr::null(),
        ];

        let mut state = fresh_test_state();
        state.del_as_icon = true;
        state.continue_label = Some(c"Apply");

        let out = unsafe { install_key_label_mirror(&mut state, 0, raw.as_ptr()) };

        let continue_cell = unsafe { *out.add(1) };
        let del_cell = unsafe { *out.add(2) };
        let continue_bytes = unsafe { core::ffi::CStr::from_ptr(continue_cell) }.to_bytes();
        assert_eq!(continue_bytes, b"Apply", "Continue cell rewritten to override");
        assert_eq!(
            del_cell,
            KEY_BACKSPACE.as_ptr(),
            "Del cell swapped to ⌫ when del_as_icon is true"
        );
    }

    #[test]
    fn continue_label_none_leaves_continue_cell_untouched() {
        use crate::lvgl::keyboard_layout::KEY_CONTINUE;
        let raw: alloc::vec::Vec<*const c_char> = alloc::vec![
            KEY_CONTINUE.as_ptr(),
            core::ptr::null(),
        ];
        let mut state = fresh_test_state();
        state.del_as_icon = true;
        state.continue_label = None;

        let out = unsafe { install_key_label_mirror(&mut state, 0, raw.as_ptr()) };

        let continue_cell = unsafe { *out };
        assert_eq!(
            continue_cell,
            KEY_CONTINUE.as_ptr(),
            "Continue cell must remain the canonical KEY_CONTINUE pointer when override is None"
        );
    }

    #[test]
    fn set_continue_enabled_finds_overridden_continue_key() {
        use crate::c_bindings::lv_buttonmatrix_get_button_text;
        use crate::lvgl::keyboard_layout::CTRL_DISABLED;
        use crate::lvgl::widget::Widget;
        let screen = setup();
        let kb = Keyboard::new(&screen);
        // Install Apply override; the active EnUs lc map contains "Continue".
        kb.continue_label(Some(c"Apply"));
        spy_drain();

        // Sanity: button-matrix now shows "Apply" somewhere.
        let raw = kb.lv_obj().raw() as *const _;
        let mut found_apply = false;
        for btn_id in 0..64u32 {
            let p = unsafe { lv_buttonmatrix_get_button_text(raw, btn_id) };
            if p.is_null() {
                break;
            }
            let bytes = unsafe { core::ffi::CStr::from_ptr(p) }.to_bytes();
            if bytes == b"Apply" {
                found_apply = true;
                break;
            }
        }
        assert!(
            found_apply,
            "an Apply cell must be present after continue_label override"
        );

        // set_continue_enabled(false) must locate the key (by override label)
        // and emit a ButtonMatrixSetButtonCtrl(CTRL_DISABLED) call. Without
        // the override-aware lookup it would silently no-op.
        kb.set_continue_enabled(false);
        let calls = spy_drain();
        assert!(
            calls.iter().any(|c| matches!(
                c,
                LvCall::ButtonMatrixSetButtonCtrl { ctrl, .. }
                if *ctrl == CTRL_DISABLED
            )),
            "set_continue_enabled(false) must find the overridden Apply key \
             and emit ButtonMatrixSetButtonCtrl(CTRL_DISABLED); got: {calls:?}"
        );
    }

    #[test]
    fn is_continue_label_allowed_rejects_reserved_and_map_breakers() {
        use super::is_continue_label_allowed;
        use crate::lvgl::keyboard_layout::{
            KEY_123, KEY_ABC, KEY_BACK, KEY_BACKSPACE, KEY_DEL, KEY_LANG,
        };
        assert!(!is_continue_label_allowed(c""), "empty label breaks LVGL map");
        assert!(!is_continue_label_allowed(c"\n"), "newline starts a new row");
        assert!(!is_continue_label_allowed(KEY_ABC));
        assert!(!is_continue_label_allowed(KEY_123));
        assert!(!is_continue_label_allowed(KEY_BACK));
        assert!(!is_continue_label_allowed(KEY_BACKSPACE));
        assert!(!is_continue_label_allowed(KEY_DEL));
        assert!(!is_continue_label_allowed(KEY_LANG));
        assert!(is_continue_label_allowed(c"Apply"));
        assert!(is_continue_label_allowed(c"Send"));
        assert!(is_continue_label_allowed(c"OK"));
    }
}
