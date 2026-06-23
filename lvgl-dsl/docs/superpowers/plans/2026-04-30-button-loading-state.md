# Button Loading State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a configurable loading state to `Button` with spinner/image/text/custom content, disabled interaction, automatic content restoration, and button-level minimum visible duration.

**Architecture:** Keep `Button` as the public widget and move loading-specific state/configuration into a focused `src/lvgl/button_loading.rs` module. `Button` owns an `Rc<RefCell<ButtonLoadingState>>` so simple APIs and `LoadingHandle` share the same state, while LVGL timers use boxed callback context that owns an `Rc` clone and never points at stack-owned `Button` values.

**Tech Stack:** Rust 2024, `alloc`, existing LVGL C bindings in `src/c_bindings.rs`, existing widget wrappers (`Button`, `Obj`, `Label`, `Spinner`, `Image`), existing mock LVGL spy/timer helpers.

---

## File structure

- Modify `src/c_bindings.rs`: add desktop/test declarations and spy support for spinner animation params, child iteration, object deletion, style translate/angle, and animation repeat count.
- Modify `src/lvgl/spinner.rs`: expose `Spinner::set_anim_params(spin_ms, arc_length_deg)`.
- Create `src/lvgl/button_loading.rs`: define loading config, indicator enum, handle, state machine, timer callback, child snapshot/restore helpers, loading content builders, and image rotation helper.
- Modify `src/lvgl/button.rs`: add loading state field, initialize it, expose loading config/control methods, and keep existing text/icon behavior intact.
- Modify `src/lvgl/mod.rs` and `src/lvgl/prelude.rs`: re-export `ButtonLoadingConfig`, `ButtonLoadingIndicator`, and `LoadingHandle`.
- Modify `DSL_REFERENCE.md`: document Button loading-state API with spinner, image, custom-content, simple on/off, handle, and min-duration examples.

## Implementation notes

- Keep public API builder methods consistent with the existing DSL: methods take `&self` where possible and return `&Self` for chaining.
- Use `ButtonLoadingConfig::default()` with `min_duration_ms = 300`, spinner indicator, no text, and `gap_px = 8`.
- Use `Option<fn(&LvObj)>` for the custom loading content hook. This supports no-capture builders in `no_std + alloc` without adding trait-object lifetime complexity.
- Use `ButtonLoadingIndicator::Image { src: ImageSrc, size_px: i32, rotation_ms: u32 }`; `rotation_ms == 0` means static image, any positive value starts a repeated 0-to-360-degree rotation animation.
- Store normal child pointers as `Vec<*mut lv_obj_t>`. Snapshot before creating the loading container, hide each saved child with `LvObjFlag::HIDDEN`, and unhide them on restore.
- Make `LoadingHandle::finish(self)` consume the handle. Its `Drop` implementation should restore immediately if the handle was never finished, preventing a forgotten handle from leaving the button disabled forever.
- For `Button::set_loading(false)`, respect minimum duration exactly like `LoadingHandle::finish()`.

---

### Task 1: Extend LVGL bindings and mock spy support

**Files:**
- Modify: `src/c_bindings.rs`

- [ ] **Step 1: Write failing binding tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `src/c_bindings.rs`:

```rust
#[test]
fn loading_bindings_record_spinner_params_and_children() {
    let _fx = SpyFixture::new();
    let parent = unsafe { lv_obj_create(core::ptr::null_mut()) };
    let child_a = unsafe { lv_label_create(parent) };
    let child_b = unsafe { lv_spinner_create(parent) };

    unsafe {
        lv_spinner_set_anim_params(child_b, 900, 90);
    }

    assert_eq!(unsafe { lv_obj_get_child_count(parent) }, 2);
    assert_eq!(unsafe { lv_obj_get_child(parent, 0) }, child_a);
    assert_eq!(unsafe { lv_obj_get_child(parent, 1) }, child_b);
    assert!(unsafe { lv_obj_get_child(parent, 2) }.is_null());

    let calls = spy_drain();
    assert!(
        calls.iter().any(|c| matches!(
            c,
            LvCall::SpinnerSetAnimParams { obj, spin_ms: 900, arc_length_deg: 90 }
                if *obj == child_b as usize
        )),
        "expected SpinnerSetAnimParams for child_b, got: {:?}",
        calls
    );
}

#[test]
fn loading_bindings_record_delete_translate_angle_and_repeat_count() {
    let _fx = SpyFixture::new();
    let obj = unsafe { lv_obj_create(core::ptr::null_mut()) };

    unsafe {
        lv_obj_set_style_translate_y(obj, 12, 0);
        lv_obj_set_style_transform_rotation(obj, 3600, 0);
        lv_obj_delete(obj);

        let mut anim = core::mem::MaybeUninit::<lv_anim_t>::uninit();
        lv_anim_init(anim.as_mut_ptr());
        lv_anim_set_repeat_count(anim.as_mut_ptr(), LV_ANIM_REPEAT_INFINITE);
    }

    let calls = spy_drain();
    assert!(
        calls.iter().any(|c| matches!(c, LvCall::SetStyleTranslateY { value: 12, .. })),
        "expected SetStyleTranslateY, got: {:?}",
        calls
    );
    assert!(
        calls.iter().any(|c| matches!(c, LvCall::SetStyleTransformRotation { angle: 3600, .. })),
        "expected SetStyleTransformRotation, got: {:?}",
        calls
    );
    assert!(
        calls.iter().any(|c| matches!(c, LvCall::ObjDelete { obj: deleted } if *deleted == obj as usize)),
        "expected ObjDelete for obj, got: {:?}",
        calls
    );
    assert!(
        calls.iter().any(|c| matches!(
            c,
            LvCall::AnimSetRepeatCount { count } if *count == LV_ANIM_REPEAT_INFINITE
        )),
        "expected AnimSetRepeatCount infinite, got: {:?}",
        calls
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/samback/Projects/jetbeep/rust/lvgl-dsl/.worktrees/button-loading-state
cargo test loading_bindings --quiet
```

Expected: compile failure mentioning missing `lv_spinner_set_anim_params`, `lv_obj_get_child`, `lv_obj_set_style_transform_rotation`, `lv_anim_set_repeat_count`, and new `LvCall` variants.

- [ ] **Step 3: Add desktop declarations and mock call variants**

In the desktop `unsafe extern "C"` block in `src/c_bindings.rs`, add declarations near related APIs:

```rust
pub fn lv_spinner_set_anim_params(obj: *mut lv_obj_t, spin_ms: u32, arc_length_deg: u32);
pub fn lv_obj_get_child(obj: *mut lv_obj_t, idx: u32) -> *mut lv_obj_t;
pub fn lv_obj_set_style_transform_rotation(obj: *mut lv_obj_t, angle: i32, selector: u32);
pub fn lv_anim_set_repeat_count(a: *mut lv_anim_t, count: u32);
```

Add this constant in both desktop and mock sections:

```rust
pub const LV_ANIM_REPEAT_INFINITE: u32 = 0xFFFF_FFFF;
```

Extend the mock `LvCall` enum:

```rust
SpinnerSetAnimParams { obj: usize, spin_ms: u32, arc_length_deg: u32 },
ObjDelete { obj: usize },
ObjGetChild { obj: usize, idx: u32, ret: usize },
SetStyleTranslateY { obj: usize, value: i32 },
SetStyleTransformRotation { obj: usize, angle: i32 },
AnimSetRepeatCount { count: u32 },
```

- [ ] **Step 4: Add mock child registry and implementations**

In the mock thread-local section, add a child registry:

```rust
pub(crate) static CHILDREN:
    RefCell<HashMap<usize, Vec<usize>>> = RefCell::new(HashMap::new());
```

Clear it in `reset_all_thread_local_spy_state()`:

```rust
CHILDREN.with(|m| m.borrow_mut().clear());
```

Add a helper near `alloc_fake_obj()`:

```rust
fn register_child(parent: *mut lv_obj_t, child: *mut lv_obj_t) {
    if parent.is_null() {
        return;
    }
    CHILDREN.with(|m| {
        m.borrow_mut()
            .entry(parent as usize)
            .or_default()
            .push(child as usize);
    });
}
```

Call `register_child(parent, obj);` in every mock create function that takes a parent and creates a normal child, including:

```rust
lv_obj_create
lv_button_create
lv_label_create
lv_spinner_create
lv_dropdown_create
lv_keyboard_create
lv_buttonmatrix_create
lv_textarea_create
lv_imagebutton_create
lv_qrcode_create
lv_image_create
```

Update `lv_obj_get_child_count` to prefer real tracked children:

```rust
pub unsafe fn lv_obj_get_child_count(obj: *mut lv_obj_t) -> u32 {
    let tracked = CHILDREN.with(|m| m.borrow().get(&(obj as usize)).map(|v| v.len() as u32));
    let v = tracked.unwrap_or_else(|| {
        CHILD_COUNTS.with(|m| m.borrow().get(&(obj as usize)).copied().unwrap_or(0))
    });
    SPY.with(|s| s.borrow_mut().push(LvCall::ObjGetChildCount { obj: obj as usize, ret: v }));
    v
}
```

Add `lv_obj_get_child`:

```rust
pub unsafe fn lv_obj_get_child(obj: *mut lv_obj_t, idx: u32) -> *mut lv_obj_t {
    let ret = CHILDREN.with(|m| {
        m.borrow()
            .get(&(obj as usize))
            .and_then(|children| children.get(idx as usize).copied())
            .unwrap_or(0)
    });
    SPY.with(|s| {
        s.borrow_mut().push(LvCall::ObjGetChild {
            obj: obj as usize,
            idx,
            ret,
        })
    });
    ret as *mut lv_obj_t
}
```

Replace the mock `lv_obj_delete` implementation with:

```rust
pub unsafe fn lv_obj_delete(obj: *mut lv_obj_t) {
    CHILDREN.with(|m| {
        let mut children = m.borrow_mut();
        children.remove(&(obj as usize));
        for list in children.values_mut() {
            list.retain(|child| *child != obj as usize);
        }
    });
    SPY.with(|s| s.borrow_mut().push(LvCall::ObjDelete { obj: obj as usize }));
}
```

Add the remaining mock implementations:

```rust
pub unsafe fn lv_spinner_set_anim_params(obj: *mut lv_obj_t, spin_ms: u32, arc_length_deg: u32) {
    SPY.with(|s| {
        s.borrow_mut().push(LvCall::SpinnerSetAnimParams {
            obj: obj as usize,
            spin_ms,
            arc_length_deg,
        })
    });
}

pub unsafe fn lv_obj_set_style_translate_y(obj: *mut lv_obj_t, value: i32, _selector: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::SetStyleTranslateY { obj: obj as usize, value }));
}

pub unsafe fn lv_obj_set_style_transform_rotation(obj: *mut lv_obj_t, angle: i32, _selector: u32) {
    SPY.with(|s| {
        s.borrow_mut().push(LvCall::SetStyleTransformRotation { obj: obj as usize, angle })
    });
}

pub unsafe fn lv_anim_set_repeat_count(_a: *mut lv_anim_t, count: u32) {
    SPY.with(|s| s.borrow_mut().push(LvCall::AnimSetRepeatCount { count }));
}
```

Update the mock symbol smoke test near the bottom of `src/c_bindings.rs` to reference the new functions so missing declarations are caught.

- [ ] **Step 5: Run binding tests**

Run:

```bash
cargo test loading_bindings --quiet
```

Expected: the two new tests pass.

- [ ] **Step 6: Commit bindings**

Run:

```bash
git add src/c_bindings.rs
git commit -m "test: add loading state lvgl binding support" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Add spinner animation parameters

**Files:**
- Modify: `src/lvgl/spinner.rs`

- [ ] **Step 1: Write failing spinner test**

Add this test to `src/lvgl/spinner.rs`:

```rust
#[test]
fn set_anim_params_records_spy() {
    use crate::c_bindings::{spy_drain, LvCall};

    reset_obj_pool();
    let p = Screen::active();
    let spinner = Spinner::new(&p);
    spy_drain();

    spinner.set_anim_params(900, 90);

    let calls = spy_drain();
    assert!(
        calls.iter().any(|c| matches!(
            c,
            LvCall::SpinnerSetAnimParams { spin_ms: 900, arc_length_deg: 90, .. }
        )),
        "expected SpinnerSetAnimParams, got: {:?}",
        calls
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test lvgl::spinner::tests::set_anim_params_records_spy --quiet
```

Expected: compile failure because `Spinner::set_anim_params` does not exist.

- [ ] **Step 3: Implement `Spinner::set_anim_params`**

Add this method inside `impl Spinner`:

```rust
/// Sets spinner animation duration and arc length.
///
/// `spin_ms` is the time for one full spin. `arc_length_deg` is the visible
/// arc length in degrees, matching LVGL's `lv_spinner_set_anim_params`.
pub fn set_anim_params(&self, spin_ms: u32, arc_length_deg: u32) -> &Self {
    unsafe {
        c_bindings::lv_spinner_set_anim_params(
            self.lv_obj().raw(),
            spin_ms,
            arc_length_deg,
        );
    }
    self
}
```

- [ ] **Step 4: Run spinner tests**

Run:

```bash
cargo test lvgl::spinner --quiet
```

Expected: all spinner tests pass.

- [ ] **Step 5: Commit spinner API**

Run:

```bash
git add src/lvgl/spinner.rs
git commit -m "feat: expose spinner animation parameters" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Add loading config types and Button state storage

**Files:**
- Create: `src/lvgl/button_loading.rs`
- Modify: `src/lvgl/button.rs`
- Modify: `src/lvgl/mod.rs`
- Modify: `src/lvgl/prelude.rs`

- [ ] **Step 1: Write failing config tests**

Create `src/lvgl/button_loading.rs` with only tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lvgl::image::ImageSrc;

    #[test]
    fn default_config_uses_spinner_and_300ms_min_duration() {
        let cfg = ButtonLoadingConfig::default();

        assert_eq!(cfg.min_duration_ms_value(), 300);
        assert_eq!(cfg.gap_px_value(), 8);
        assert!(cfg.text_value().is_none());
        assert!(matches!(cfg.indicator_value(), ButtonLoadingIndicator::Spinner {
            size_px: 24,
            spin_ms: 900,
            arc_length_deg: 90,
        }));
        assert!(cfg.custom_content_value().is_none());
    }

    #[test]
    fn builder_sets_text_indicator_gap_and_custom_content() {
        fn custom(_: &crate::lvgl::LvObj) {}

        let dummy: u8 = 0;
        let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
        let cfg = ButtonLoadingConfig::new()
            .text("Loading")
            .min_duration_ms(450)
            .gap_px(14)
            .indicator(ButtonLoadingIndicator::Image {
                src,
                size_px: 32,
                rotation_ms: 800,
            })
            .custom_content(custom);

        assert_eq!(cfg.text_value(), Some("Loading"));
        assert_eq!(cfg.min_duration_ms_value(), 450);
        assert_eq!(cfg.gap_px_value(), 14);
        assert!(cfg.custom_content_value().is_some());
        assert!(matches!(cfg.indicator_value(), ButtonLoadingIndicator::Image {
            size_px: 32,
            rotation_ms: 800,
            ..
        }));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test lvgl::button_loading --quiet
```

Expected: compile failure because the module and types are not wired.

- [ ] **Step 3: Implement config types**

Replace `src/lvgl/button_loading.rs` with:

```rust
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ffi::c_void;

use crate::c_bindings::{self, lv_obj_t, lv_timer_t};

use super::flex::FlexAlign;
use super::image::{Image, ImageSrc};
use super::label::Label;
use super::obj::Obj;
use super::size::Size;
use super::spinner::Spinner;
use super::state::{LvObjFlag, LvState};
use super::widget::{LvObj, Widget};
use super::{FlexFlow, LvAlign};

pub type ButtonLoadingCustomContent = fn(&LvObj);

#[derive(Copy, Clone)]
pub enum ButtonLoadingIndicator {
    Spinner {
        size_px: i32,
        spin_ms: u32,
        arc_length_deg: u32,
    },
    Image {
        src: ImageSrc,
        size_px: i32,
        rotation_ms: u32,
    },
    None,
}

#[derive(Clone)]
pub struct ButtonLoadingConfig {
    min_duration_ms: u32,
    text: Option<String>,
    indicator: ButtonLoadingIndicator,
    gap_px: i32,
    custom_content: Option<ButtonLoadingCustomContent>,
}

impl Default for ButtonLoadingConfig {
    fn default() -> Self {
        Self {
            min_duration_ms: 300,
            text: None,
            indicator: ButtonLoadingIndicator::Spinner {
                size_px: 24,
                spin_ms: 900,
                arc_length_deg: 90,
            },
            gap_px: 8,
            custom_content: None,
        }
    }
}

impl ButtonLoadingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_duration_ms(mut self, value: u32) -> Self {
        self.min_duration_ms = value;
        self
    }

    pub fn text(mut self, value: &str) -> Self {
        self.text = Some(value.to_string());
        self
    }

    pub fn clear_text(mut self) -> Self {
        self.text = None;
        self
    }

    pub fn indicator(mut self, value: ButtonLoadingIndicator) -> Self {
        self.indicator = value;
        self
    }

    pub fn gap_px(mut self, value: i32) -> Self {
        self.gap_px = value;
        self
    }

    pub fn custom_content(mut self, builder: ButtonLoadingCustomContent) -> Self {
        self.custom_content = Some(builder);
        self
    }

    pub fn min_duration_ms_value(&self) -> u32 {
        self.min_duration_ms
    }

    pub fn text_value(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn indicator_value(&self) -> ButtonLoadingIndicator {
        self.indicator
    }

    pub fn gap_px_value(&self) -> i32 {
        self.gap_px
    }

    pub fn custom_content_value(&self) -> Option<ButtonLoadingCustomContent> {
        self.custom_content
    }
}

pub(crate) struct ButtonLoadingState {
    pub(crate) config: ButtonLoadingConfig,
    pub(crate) active: bool,
    pub(crate) min_elapsed: bool,
    pub(crate) finish_pending: bool,
    pub(crate) normal_children: Vec<*mut lv_obj_t>,
    pub(crate) loading_container: *mut lv_obj_t,
    pub(crate) min_timer: *mut lv_timer_t,
    pub(crate) timer_ctx: *mut ButtonLoadingTimerCtx,
}

impl ButtonLoadingState {
    pub(crate) fn new() -> Self {
        Self {
            config: ButtonLoadingConfig::default(),
            active: false,
            min_elapsed: true,
            finish_pending: false,
            normal_children: Vec::new(),
            loading_container: core::ptr::null_mut(),
            min_timer: core::ptr::null_mut(),
            timer_ctx: core::ptr::null_mut(),
        }
    }
}

pub(crate) struct ButtonLoadingTimerCtx {
    pub(crate) button_obj: *mut lv_obj_t,
    pub(crate) state: Rc<RefCell<ButtonLoadingState>>,
}

pub struct LoadingHandle {
    button_obj: *mut lv_obj_t,
    state: Rc<RefCell<ButtonLoadingState>>,
    finished: bool,
}
```

Keep the test module from Step 1 at the bottom of the file.

- [ ] **Step 4: Wire module and Button storage**

In `src/lvgl/mod.rs`, add:

```rust
mod button_loading;
```

and change the button export to:

```rust
pub use self::button::Button;
pub use self::button_loading::{ButtonLoadingConfig, ButtonLoadingIndicator, LoadingHandle};
```

In `src/lvgl/prelude.rs`, add:

```rust
pub use super::button_loading::{ButtonLoadingConfig, ButtonLoadingIndicator, LoadingHandle};
```

In `src/lvgl/button.rs`, add imports:

```rust
use alloc::rc::Rc;
use core::cell::RefCell;

use super::button_loading::{ButtonLoadingConfig, ButtonLoadingState, LoadingHandle};
```

Change `Button` to:

```rust
pub struct Button {
    obj: LvObj,
    pub(crate) loading: Rc<RefCell<ButtonLoadingState>>,
}
```

Change `new` and `from_raw` constructors to initialize loading state:

```rust
Button {
    obj: LvObj::from_raw(obj),
    loading: Rc::new(RefCell::new(ButtonLoadingState::new())),
}
```

and:

```rust
Button {
    obj: LvObj::from_raw(ptr),
    loading: Rc::new(RefCell::new(ButtonLoadingState::new())),
}
```

Add a config method stub that compiles:

```rust
pub fn loading_config(&self, cfg: ButtonLoadingConfig) -> &Self {
    self.loading.borrow_mut().config = cfg;
    self
}
```

- [ ] **Step 5: Run config tests**

Run:

```bash
cargo test lvgl::button_loading --quiet
cargo test lvgl::button --quiet
```

Expected: loading config tests pass and existing button tests still pass.

- [ ] **Step 6: Commit config/state scaffolding**

Run:

```bash
git add src/lvgl/button_loading.rs src/lvgl/button.rs src/lvgl/mod.rs src/lvgl/prelude.rs
git commit -m "feat: add button loading configuration types" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Build loading content and start behavior

**Files:**
- Modify: `src/lvgl/button_loading.rs`
- Modify: `src/lvgl/button.rs`

- [ ] **Step 1: Write failing start-loading tests**

Add these tests to `src/lvgl/button.rs` in the existing `tests` module:

```rust
#[test]
fn start_loading_replaces_content_with_spinner_text_and_disables_button() {
    use crate::c_bindings::LvCall;
    use crate::lvgl::{ButtonLoadingConfig, ButtonLoadingIndicator};

    let p = parent();
    let btn = Button::new(&p);
    btn.text("Pay");
    spy_drain();

    btn.loading_config(
        ButtonLoadingConfig::new()
            .text("LOADING...")
            .indicator(ButtonLoadingIndicator::Spinner {
                size_px: 36,
                spin_ms: 700,
                arc_length_deg: 120,
            })
            .gap_px(12),
    );

    let _handle = btn.start_loading();
    let calls = spy_drain();

    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjGetChildCount { ret: 1, .. })), "expected child snapshot, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::AddState { state, .. } if *state == crate::lvgl::LvState::DISABLED.0)), "expected disabled state, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::AddFlag { flag, .. } if *flag == crate::lvgl::LvObjFlag::HIDDEN.0)), "expected normal child hidden, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjCreate { .. })), "expected loading container, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::SpinnerSetAnimParams { spin_ms: 700, arc_length_deg: 120, .. })), "expected spinner params, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 36, h: 36, .. })), "expected spinner size, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"LOADING...\0")), "expected loading text, got: {:?}", calls);
    assert!(btn.is_loading());
}

#[test]
fn start_loading_creates_image_indicator_and_custom_content() {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use crate::c_bindings::LvCall;
    use crate::lvgl::{ButtonLoadingConfig, ButtonLoadingIndicator, ImageSrc, Label};

    static CUSTOM_CALLS: AtomicUsize = AtomicUsize::new(0);
    fn custom(parent: &crate::lvgl::LvObj) {
        CUSTOM_CALLS.fetch_add(1, Ordering::SeqCst);
        Label::new(parent).text("custom");
    }

    let p = parent();
    let btn = Button::new(&p);
    let dummy: u8 = 0;
    let src = unsafe { ImageSrc::descriptor(core::ptr::addr_of!(dummy).cast()) };
    btn.loading_config(
        ButtonLoadingConfig::new()
            .indicator(ButtonLoadingIndicator::Image {
                src,
                size_px: 28,
                rotation_ms: 600,
            })
            .custom_content(custom),
    );
    spy_drain();

    let _handle = btn.start_loading();
    let calls = spy_drain();

    assert_eq!(CUSTOM_CALLS.load(Ordering::SeqCst), 1);
    assert!(calls.iter().any(|c| matches!(c, LvCall::ImageCreate { .. })), "expected ImageCreate, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::ImageSetSrc { .. })), "expected ImageSetSrc, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjSetSize { w: 28, h: 28, .. })), "expected image size, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::AnimSetRepeatCount { count } if *count == crate::c_bindings::LV_ANIM_REPEAT_INFINITE)), "expected repeated rotation animation, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"custom\0")), "expected custom label, got: {:?}", calls);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test lvgl::button::tests::start_loading --quiet
```

Expected: compile failure because `start_loading`, `is_loading`, and content builders are not implemented.

- [ ] **Step 3: Implement start-loading helpers**

Add these functions to `src/lvgl/button_loading.rs` after the type definitions:

```rust
impl LoadingHandle {
    pub(crate) fn new(button_obj: *mut lv_obj_t, state: Rc<RefCell<ButtonLoadingState>>) -> Self {
        Self { button_obj, state, finished: false }
    }
}

pub(crate) fn is_loading(state: &Rc<RefCell<ButtonLoadingState>>) -> bool {
    state.borrow().active
}

pub(crate) fn start(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    if state.borrow().active {
        return;
    }

    {
        let mut s = state.borrow_mut();
        s.active = true;
        s.min_elapsed = s.config.min_duration_ms_value() == 0;
        s.finish_pending = false;
        s.normal_children = snapshot_children(button_obj);
    }

    unsafe {
        c_bindings::lv_obj_add_state(button_obj, LvState::DISABLED.0);
    }

    for child in state.borrow().normal_children.iter().copied() {
        unsafe { c_bindings::lv_obj_add_flag(child, LvObjFlag::HIDDEN.0) };
    }

    let container = create_loading_container(button_obj, &state.borrow().config);
    state.borrow_mut().loading_container = container;
    create_loading_content(container, &state.borrow().config);
    start_min_timer(button_obj, state);
}

fn snapshot_children(button_obj: *mut lv_obj_t) -> Vec<*mut lv_obj_t> {
    let count = unsafe { c_bindings::lv_obj_get_child_count(button_obj) };
    let mut children = Vec::new();
    for idx in 0..count {
        let child = unsafe { c_bindings::lv_obj_get_child(button_obj, idx) };
        if !child.is_null() {
            children.push(child);
        }
    }
    children
}

fn create_loading_container(button_obj: *mut lv_obj_t, cfg: &ButtonLoadingConfig) -> *mut lv_obj_t {
    let container = unsafe { c_bindings::lv_obj_create(button_obj) };
    if container.is_null() {
        panic!("lv_obj_create returned null for button loading container");
    }

    let container_widget = unsafe { LvObj::from_raw(container) };
    container_widget
        .set_flex_flow(FlexFlow::Row)
        .flex_align(FlexAlign::Center, FlexAlign::Center, FlexAlign::Center)
        .gap(cfg.gap_px_value())
        .align(LvAlign::Center, 0, 0);

    container
}

fn create_loading_content(container: *mut lv_obj_t, cfg: &ButtonLoadingConfig) {
    let container_widget = unsafe { LvObj::from_raw(container) };

    match cfg.indicator_value() {
        ButtonLoadingIndicator::Spinner { size_px, spin_ms, arc_length_deg } => {
            Spinner::new(&container_widget)
                .size(Size::Px(size_px), Size::Px(size_px))
                .set_anim_params(spin_ms, arc_length_deg);
        }
        ButtonLoadingIndicator::Image { src, size_px, rotation_ms } => {
            let image = Image::new(&container_widget)
                .set_src(&src)
                .size(Size::Px(size_px), Size::Px(size_px));
            if rotation_ms > 0 {
                start_image_rotation(image.lv_obj().raw(), rotation_ms);
            }
        }
        ButtonLoadingIndicator::None => {}
    }

    if let Some(text) = cfg.text_value() {
        Label::new(&container_widget).text(text);
    }

    if let Some(builder) = cfg.custom_content_value() {
        builder(&container_widget);
    }
}

unsafe extern "C" fn rotate_image_exec(var: *mut c_void, angle: i32) {
    unsafe {
        c_bindings::lv_obj_set_style_transform_rotation(var.cast::<lv_obj_t>(), angle, 0);
    }
}

fn start_image_rotation(obj: *mut lv_obj_t, rotation_ms: u32) {
    unsafe {
        let mut anim = core::mem::MaybeUninit::<c_bindings::lv_anim_t>::uninit();
        let anim_ptr = anim.as_mut_ptr();
        c_bindings::lv_anim_init(anim_ptr);
        c_bindings::lv_anim_set_var(anim_ptr, obj.cast::<c_void>());
        c_bindings::lv_anim_set_exec_cb(anim_ptr, Some(rotate_image_exec));
        c_bindings::lv_anim_set_values(anim_ptr, 0, 3600);
        c_bindings::lv_anim_set_duration(anim_ptr, rotation_ms);
        c_bindings::lv_anim_set_repeat_count(anim_ptr, c_bindings::LV_ANIM_REPEAT_INFINITE);
        c_bindings::lv_anim_start(anim_ptr as *const _);
    }
}
```

Add `start_min_timer` as a stub for this task:

```rust
fn start_min_timer(_button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    if state.borrow().config.min_duration_ms_value() == 0 {
        state.borrow_mut().min_elapsed = true;
    }
}
```

- [ ] **Step 4: Expose Button loading methods**

In `src/lvgl/button.rs`, add:

```rust
pub fn start_loading(&self) -> LoadingHandle {
    super::button_loading::start(self.lv_obj().raw(), &self.loading);
    LoadingHandle::new(self.lv_obj().raw(), self.loading.clone())
}

pub fn is_loading(&self) -> bool {
    super::button_loading::is_loading(&self.loading)
}
```

- [ ] **Step 5: Run start-loading tests**

Run:

```bash
cargo test lvgl::button::tests::start_loading --quiet
```

Expected: both start-loading tests pass.

- [ ] **Step 6: Commit only if tests pass**

Expected: both start-loading tests pass.

Run:

```bash
git add src/lvgl/button_loading.rs src/lvgl/button.rs
git commit -m "feat: start button loading state" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Implement minimum-duration finish and LoadingHandle

**Files:**
- Modify: `src/lvgl/button_loading.rs`
- Modify: `src/lvgl/button.rs`

- [ ] **Step 1: Write failing finish tests**

Add these tests to `src/lvgl/button.rs`:

```rust
#[test]
fn set_loading_false_waits_for_min_duration_then_restores() {
    use crate::c_bindings::{spy_fire_timer, spy_live_timer_handles, LvCall};
    use crate::lvgl::ButtonLoadingConfig;

    let p = parent();
    let btn = Button::new(&p);
    btn.text("Submit");
    btn.loading_config(ButtonLoadingConfig::new().min_duration_ms(500).text("Loading"));
    spy_drain();

    btn.set_loading(true);
    assert!(btn.is_loading());
    let handles = spy_live_timer_handles();
    assert_eq!(handles.len(), 1, "expected one min-duration timer");
    spy_drain();

    btn.set_loading(false);
    assert!(btn.is_loading(), "finish before timer should remain pending");
    let early_calls = spy_drain();
    assert!(!early_calls.iter().any(|c| matches!(c, LvCall::ObjDelete { .. })), "should not restore before timer: {:?}", early_calls);

    spy_fire_timer(handles[0] as *mut crate::c_bindings::lv_timer_t);
    assert!(!btn.is_loading());
    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::ObjDelete { .. })), "expected loading container delete, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::RemoveFlag { flag, .. } if *flag == crate::lvgl::LvObjFlag::HIDDEN.0)), "expected normal child unhidden, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::RemoveState { state, .. } if *state == crate::lvgl::LvState::DISABLED.0)), "expected disabled state removed, got: {:?}", calls);
    assert!(calls.iter().any(|c| matches!(c, LvCall::TimerDelete { .. })), "expected timer deleted, got: {:?}", calls);
}

#[test]
fn loading_handle_finish_and_drop_are_idempotent() {
    use crate::c_bindings::LvCall;
    use crate::lvgl::ButtonLoadingConfig;

    let p = parent();
    let btn = Button::new(&p);
    btn.text("Submit");
    btn.loading_config(ButtonLoadingConfig::new().min_duration_ms(0).text("Loading"));
    spy_drain();

    let handle = btn.start_loading();
    assert!(btn.is_loading());
    handle.finish();
    assert!(!btn.is_loading());

    let calls = spy_drain();
    let deletes = calls.iter().filter(|c| matches!(c, LvCall::ObjDelete { .. })).count();
    assert_eq!(deletes, 1, "finish should delete loading container once: {:?}", calls);

    let handle = btn.start_loading();
    assert!(btn.is_loading());
    drop(handle);
    assert!(!btn.is_loading(), "dropping unfinished handle should restore immediately");
}
```

- [ ] **Step 2: Run finish tests to verify they fail**

Run:

```bash
cargo test lvgl::button::tests::set_loading_false_waits_for_min_duration_then_restores lvgl::button::tests::loading_handle_finish_and_drop_are_idempotent --quiet
```

Expected: compile failure or test failure because finish/timer behavior is incomplete.

- [ ] **Step 3: Implement timer and restore functions**

Replace the Task 4 `start_min_timer` stub in `src/lvgl/button_loading.rs` with:

```rust
fn start_min_timer(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    let period = state.borrow().config.min_duration_ms_value();
    if period == 0 {
        state.borrow_mut().min_elapsed = true;
        return;
    }

    let ctx = alloc::boxed::Box::new(ButtonLoadingTimerCtx {
        button_obj,
        state: state.clone(),
    });
    let raw_ctx = alloc::boxed::Box::into_raw(ctx);
    let timer = unsafe {
        c_bindings::lv_timer_create(
            Some(on_min_duration_elapsed),
            period,
            raw_ctx.cast::<c_void>(),
        )
    };
    unsafe {
        c_bindings::lv_timer_set_repeat_count(timer, 1);
    }

    let mut s = state.borrow_mut();
    s.min_timer = timer;
    s.timer_ctx = raw_ctx;
}

unsafe extern "C" fn on_min_duration_elapsed(timer: *mut lv_timer_t) {
    let ctx = unsafe { c_bindings::lv_timer_get_user_data(timer) } as *mut ButtonLoadingTimerCtx;
    if ctx.is_null() {
        return;
    }

    let (button_obj, state) = unsafe {
        let ctx_ref = &*ctx;
        (ctx_ref.button_obj, ctx_ref.state.clone())
    };

    {
        let mut s = state.borrow_mut();
        s.min_elapsed = true;
    }

    if state.borrow().finish_pending {
        restore(button_obj, &state);
    }
}
```

Add finish/restore helpers:

```rust
pub(crate) fn finish(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    if !state.borrow().active {
        return;
    }

    if state.borrow().min_elapsed {
        restore(button_obj, state);
    } else {
        state.borrow_mut().finish_pending = true;
    }
}

pub(crate) fn restore_now(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    if state.borrow().active {
        state.borrow_mut().min_elapsed = true;
        restore(button_obj, state);
    }
}

fn restore(button_obj: *mut lv_obj_t, state: &Rc<RefCell<ButtonLoadingState>>) {
    let (container, children, timer, ctx) = {
        let mut s = state.borrow_mut();
        if !s.active {
            return;
        }

        s.active = false;
        s.finish_pending = false;
        s.min_elapsed = true;

        let container = core::mem::replace(&mut s.loading_container, core::ptr::null_mut());
        let children = core::mem::take(&mut s.normal_children);
        let timer = core::mem::replace(&mut s.min_timer, core::ptr::null_mut());
        let ctx = core::mem::replace(&mut s.timer_ctx, core::ptr::null_mut());
        (container, children, timer, ctx)
    };

    if !container.is_null() {
        unsafe { c_bindings::lv_obj_delete(container) };
    }

    for child in children {
        unsafe { c_bindings::lv_obj_remove_flag(child, LvObjFlag::HIDDEN.0) };
    }

    unsafe { c_bindings::lv_obj_remove_state(button_obj, LvState::DISABLED.0) };

    if !timer.is_null() {
        unsafe { c_bindings::lv_timer_delete(timer) };
    }

    if !ctx.is_null() {
        unsafe { drop(alloc::boxed::Box::from_raw(ctx)) };
    }
}
```

- [ ] **Step 4: Implement handle and simple Button API**

In `src/lvgl/button_loading.rs`, implement:

```rust
impl LoadingHandle {
    pub fn finish(mut self) {
        if !self.finished {
            finish(self.button_obj, &self.state);
            self.finished = true;
        }
    }
}

impl Drop for LoadingHandle {
    fn drop(&mut self) {
        if !self.finished {
            restore_now(self.button_obj, &self.state);
            self.finished = true;
        }
    }
}
```

In `src/lvgl/button.rs`, add:

```rust
pub fn set_loading(&self, loading: bool) -> &Self {
    if loading {
        super::button_loading::start(self.lv_obj().raw(), &self.loading);
    } else {
        super::button_loading::finish(self.lv_obj().raw(), &self.loading);
    }
    self
}
```

- [ ] **Step 5: Run finish tests**

Run:

```bash
cargo test lvgl::button::tests::set_loading_false_waits_for_min_duration_then_restores lvgl::button::tests::loading_handle_finish_and_drop_are_idempotent --quiet
```

Expected: both tests pass.

- [ ] **Step 6: Run button loading tests together**

Run:

```bash
cargo test lvgl::button --quiet
cargo test lvgl::button_loading --quiet
```

Expected: all button and button_loading tests pass.

- [ ] **Step 7: Commit loading state machine**

Run:

```bash
git add src/lvgl/button_loading.rs src/lvgl/button.rs
git commit -m "feat: manage button loading duration and restore" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Polish tests, docs, and public examples

**Files:**
- Modify: `src/lvgl/button.rs`
- Modify: `src/lvgl/button_loading.rs`
- Modify: `DSL_REFERENCE.md`

- [ ] **Step 1: Add edge-case tests**

Add these tests to `src/lvgl/button.rs`:

```rust
#[test]
fn repeated_start_and_finish_calls_are_idempotent() {
    use crate::c_bindings::LvCall;
    use crate::lvgl::ButtonLoadingConfig;

    let p = parent();
    let btn = Button::new(&p);
    btn.text("Submit");
    btn.loading_config(ButtonLoadingConfig::new().min_duration_ms(0).text("Loading"));
    spy_drain();

    btn.set_loading(true);
    btn.set_loading(true);
    btn.set_loading(false);
    btn.set_loading(false);

    let calls = spy_drain();
    let containers = calls.iter().filter(|c| matches!(c, LvCall::ObjCreate { .. })).count();
    let deletes = calls.iter().filter(|c| matches!(c, LvCall::ObjDelete { .. })).count();
    assert_eq!(containers, 1, "second start should not create another container: {:?}", calls);
    assert_eq!(deletes, 1, "second finish should not delete twice: {:?}", calls);
    assert!(!btn.is_loading());
}

#[test]
fn none_indicator_with_text_only_creates_label_without_spinner_or_image() {
    use crate::c_bindings::LvCall;
    use crate::lvgl::{ButtonLoadingConfig, ButtonLoadingIndicator};

    let p = parent();
    let btn = Button::new(&p);
    btn.loading_config(
        ButtonLoadingConfig::new()
            .indicator(ButtonLoadingIndicator::None)
            .text("Please wait"),
    );
    spy_drain();

    let handle = btn.start_loading();
    let calls = spy_drain();
    assert!(calls.iter().any(|c| matches!(c, LvCall::LabelSetText { text_bytes, .. } if text_bytes == b"Please wait\0")), "expected text label, got: {:?}", calls);
    assert!(!calls.iter().any(|c| matches!(c, LvCall::SpinnerSetAnimParams { .. })), "did not expect spinner, got: {:?}", calls);
    assert!(!calls.iter().any(|c| matches!(c, LvCall::ImageCreate { .. })), "did not expect image, got: {:?}", calls);
    handle.finish();
}
```

- [ ] **Step 2: Run edge-case tests to verify they pass**

Run:

```bash
cargo test lvgl::button::tests::repeated_start_and_finish_calls_are_idempotent lvgl::button::tests::none_indicator_with_text_only_creates_label_without_spinner_or_image --quiet
```

Expected: both tests pass.

- [ ] **Step 3: Update `DSL_REFERENCE.md` Button methods**

In the Button methods table around the existing Button section, add:

```markdown
| `loading_config(ButtonLoadingConfig)` | Configures the loading-state presentation and minimum visible duration for this button. |
| `set_loading(bool)` | Starts or finishes loading using the configured minimum duration. |
| `start_loading() -> LoadingHandle` | Starts loading and returns a handle whose `finish()` method completes loading with minimum-duration handling. |
| `is_loading() -> bool` | Returns whether the button is currently showing loading content. |
```

- [ ] **Step 4: Add `DSL_REFERENCE.md` loading examples**

After the existing Button example, add:

````markdown
**Loading state**

```rust
let btn = Button::new(&container)
    .width(Size::Px(280))
    .height(Size::Px(64))
    .radius(CornerRadius::Full)
    .loading_config(
        ButtonLoadingConfig::new()
            .text("LOADING...")
            .min_duration_ms(300)
            .indicator(ButtonLoadingIndicator::Spinner {
                size_px: 36,
                spin_ms: 900,
                arc_length_deg: 90,
            })
            .gap_px(12),
    );

btn.text("Confirm");
btn.set_loading(true);
// Later, when the operation completes:
btn.set_loading(false);
```

Use a managed handle when the loading operation has a clear owner:

```rust
let loading = btn.start_loading();
// Perform work...
loading.finish();
```

Use a static or rotating image as the indicator:

```rust
let src = ImageSrc::file(c"/lfs/icons/sync.bin");
btn.loading_config(
    ButtonLoadingConfig::new()
        .text("SYNCING")
        .indicator(ButtonLoadingIndicator::Image {
            src,
            size_px: 32,
            rotation_ms: 800,
        }),
);
```

Use a custom child builder for project-specific loading content. This is also
the extension point for future widgets such as Lottie once a dedicated wrapper
exists:

```rust
fn loading_content(parent: &LvObj) {
    Label::new(parent).text("Please wait");
}

btn.loading_config(
    ButtonLoadingConfig::new()
        .indicator(ButtonLoadingIndicator::None)
        .custom_content(loading_content),
);
```
````

- [ ] **Step 5: Run documentation-sensitive checks**

Run:

```bash
cargo fmt --check
cargo test --quiet
```

Expected: formatting check passes and all tests pass.

- [ ] **Step 6: Commit docs and polish**

Run:

```bash
git add src/lvgl/button.rs src/lvgl/button_loading.rs DSL_REFERENCE.md
git commit -m "docs: document button loading state" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 7: Final verification

**Files:**
- No planned source edits.

- [ ] **Step 1: Run full verification**

Run:

```bash
cd /Users/samback/Projects/jetbeep/rust/lvgl-dsl/.worktrees/button-loading-state
cargo fmt --check
cargo build --quiet
cargo test --quiet
git --no-pager status --short
```

Expected:

- `cargo fmt --check` exits 0.
- `cargo build --quiet` exits 0.
- `cargo test --quiet` exits 0 with all tests passing.
- `git status --short` is empty.

- [ ] **Step 2: Inspect branch commits**

Run:

```bash
git --no-pager log --oneline --decorate -6
```

Expected: recent commits include the design spec commit plus the implementation commits from this plan.

- [ ] **Step 3: Prepare completion handoff**

If all verification passes, use the `verification-before-completion` skill before claiming completion, then use the `requesting-code-review` skill to review the completed feature before any merge/PR work.

---

## Self-review notes

- Spec coverage: the plan covers button-level config, spinner/image/text/custom content, simple and handle APIs, disabled state, restoration, 300 ms minimum duration, unsupported Lottie as custom hook only, tests, and docs.
- Red-flag scan: no task uses incomplete marker language; each task names exact files, tests, commands, and expected results.
- Type consistency: `ButtonLoadingConfig`, `ButtonLoadingIndicator`, `LoadingHandle`, `start_loading`, `set_loading`, and `is_loading` are introduced before later tasks use them.
