# Button loading state design

## Problem

Buttons need a first-class loading state that can replace normal button content
with a configurable loading presentation, block interaction while work is in
progress, and keep the loading animation visible for a configurable minimum
duration to avoid flicker.

The implementation will extend the existing `Button` wrapper in the Rust LVGL
DSL. It will run on branch `feature/button-loading-state` in the
`.worktrees/button-loading-state` worktree.

## Goals

- Add button-level loading configuration.
- Support built-in spinner, image, and text loading content.
- Provide an extension hook for custom loading children, including future Lottie
  support once a real Lottie wrapper exists.
- Support both simple on/off loading control and a managed `LoadingHandle`.
- Disable button interaction while loading.
- Restore existing button children automatically after loading completes.
- Enforce a configurable minimum visible loading duration, defaulting to 300 ms.
- Document the new API and examples in `DSL_REFERENCE.md`.

## Non-goals

- Add a full Lottie widget wrapper in this feature.
- Fake unsupported animation types when LVGL bindings are missing.
- Rework unrelated button styling or event APIs.

## Public API

`Button` will gain loading configuration and control methods:

```rust
let btn = Button::new(&container)
    .loading_config(
        ButtonLoadingConfig::new()
            .text("LOADING...")
            .min_duration_ms(300)
            .indicator(ButtonLoadingIndicator::Spinner {
                size_px: 36,
                spin_ms: 900,
                arc_length_deg: 90,
            })
            .gap_px(12)
    );

btn.set_loading(true);
btn.set_loading(false);

let loading = btn.start_loading();
loading.finish();
```

`ButtonLoadingConfig` will include:

- `min_duration_ms(u32)`, default `300`.
- `text(&str)` for optional loading text.
- `indicator(ButtonLoadingIndicator)`.
- `gap_px(i32)` for spacing between indicator and text.
- a custom-content hook that receives the loading container parent and can build
  arbitrary child widgets.

`ButtonLoadingIndicator` will include:

- `Spinner { size_px, spin_ms, arc_length_deg }`.
- `Image { src, size_px, rotation_ms }`.
- `None`.

The custom-content hook is the supported path for project-specific loading
content, including a future Lottie child once the DSL exposes one.

## Behavior

Starting loading will:

1. Snapshot the button's current direct children.
2. Add `LvState::DISABLED` to block clicks and apply disabled styling.
3. Hide the saved normal children.
4. Create a centered loading container child.
5. Populate that container with the configured spinner, image, text, and/or
   custom content.
6. Start a one-shot LVGL timer for the configured minimum duration.

Finishing loading will:

1. If the minimum duration has elapsed, restore immediately.
2. If the minimum duration has not elapsed, mark finish as pending and restore
   from the timer callback.
3. Delete the loading container.
4. Unhide the saved normal children.
5. Remove `LvState::DISABLED`.

Repeated `set_loading(true)`, `set_loading(false)`, and `LoadingHandle::finish`
calls will be idempotent. Dropping an unfinished handle will clean up the timer
and loading content so the button is not left in a permanently disabled state.

## Data flow and implementation notes

The implementation will add bindings for LVGL child iteration where needed
(`lv_obj_get_child`) so normal children can be hidden and restored without
destroying caller-created content. It will reuse existing wrappers for `Obj`,
`Label`, `Spinner`, and `Image`, and extend `Spinner` only as needed to expose
animation parameters.

Minimum-duration handling will use LVGL timers rather than host wall-clock time:
the start path creates a one-shot timer, the timer marks the minimum duration as
elapsed, and the finish path either restores immediately or waits for that timer.

Image rotation animation will use the existing LVGL animation bindings already
used by the keyboard slide animation. Lottie is intentionally left to the custom
content hook until a dedicated wrapper and bindings are introduced.

## Error handling

Widget creation will follow existing DSL conventions and panic if LVGL returns a
null pointer. The API will not silently degrade unsupported content. `ImageSrc`
will keep its existing lifetime requirement: LVGL stores the source pointer, so
the source must outlive widgets using it.

## Testing

Tests will cover:

- default config values, including 300 ms minimum duration;
- spinner, image, text, and custom loading content creation;
- hiding existing children and restoring them after loading;
- disabled state add/remove behavior;
- simple `set_loading(true/false)`;
- managed `LoadingHandle::finish`;
- pending finish before the minimum duration elapses;
- immediate finish after the timer fires;
- idempotent repeated finish calls;
- cleanup when an unfinished handle is dropped.

## Documentation

`DSL_REFERENCE.md` will document the new Button loading-state API with examples
for spinner + text, image indicator, custom content, and minimum-duration
configuration.
