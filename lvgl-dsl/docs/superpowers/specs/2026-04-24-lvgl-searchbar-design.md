# LVGL SearchBar — Design (v1, revised after fifth rubber-duck pass)

Status: approved for planning
Crate: `lvgl-dsl`
Target: `no_std` + `extern crate alloc` (Zephyr embedded + desktop-sim)
LVGL version: **pinned to LVGL v9.2.x** (label recolor API and timer API as in v9.2). Any deviation requires re-pinning §6 and §8.

## 1. Problem & Goals

Build a `SearchBar` widget for the existing safe-Rust LVGL v9 DSL that combines a search input, an asynchronously-fed result list, and visual loading / error / empty states. The widget is a single composite that consumers configure declaratively (icons, columns, highlight, callbacks) and feed from network code without owning any networking concerns itself.

Reference UX is the WECHIP "Who is the recipient?" screens (empty / typing / results+selection / loading) supplied with the brief.

### v1 in scope

- Search input (text area) with optional left icon and optional right "clear" icon.
- Empty-state placeholder shown when the query is empty.
- Schema-driven column rendering only (one `Row` per result; one `Label` per cell).
- Match highlighting via `lv_label_set_recolor` markup (color-only — no font-weight change). Algorithms: `Substring | Prefix | WordPrefix`; case sensitivity configurable; highlight color configurable.
- Single, multi, or no row selection with per-row indicator and selected-row tint.
- **Token-based async data lifecycle:** every async-replyable callback hands the caller a `RequestToken` plus, for pagination, a monotonic `page_index`. Every responding setter requires the matching token; mismatched tokens are silently dropped (with a debug-build counter for visibility).
- Debounced `on_query_changed` via `lv_timer`; opt-out with `debounce_ms(0)`. Duplicate-query suppression.
- Pagination: `on_load_more(token, page_index)` on scroll-near-end, gated by `has_more`. Auto-fill: when results don't fill the viewport and `has_more` is true, SearchBar auto-issues `on_load_more` (with the footer loading slot auto-shown) until the viewport is full or `has_more` is false.
- Four caller-fillable slot containers exposed via accessors: `initial_loading_slot()`, `initial_error_slot()`, `footer_loading_slot()`, `footer_error_slot()`. Caller creates spinner / error widgets *as children of these slots*. SearchBar toggles slot visibility.
- Built-in integration with the existing `Keyboard` widget via `attach_keyboard(&kb)` (last-bind-wins).
- Inner-widget accessors (`text_area()`, `input_container()`, `result_container()`) returning `&dyn Widget` for advanced styling.
- Static `fn`-pointer callbacks (matches the rest of the DSL).
- Optional stable row IDs (`Row::cells(&[…]).id(u64)`); SearchBar generates sequential IDs when not provided.
- `max_rows(n)` hard cap; on reaching the cap, SearchBar internally forces `has_more=false` and stops pagination.

### v1 explicitly out of scope (revisit in v2)

- Custom row builder (Mode B with per-row closures).
- Live re-highlighting of existing rows on every keystroke (`highlight_live`).
- Row recycling (rows are materialized fresh on every `set_results`).
- Bold or font-weight changes inside highlighted matches (LVGL build does not include a bold Montserrat).
- Built-in spinner widget.
- Drop semantics on individual sub-widgets — matches the existing "lifetime = parent screen" pattern.
- Column sorting, column resizing, true virtualised scrolling.
- Reparenting of caller-provided widgets after the slot accessor is first used.

## 2. Architecture & Widget Tree

```
SearchBar (root Obj, flex column)
├── input_container        (Obj, flex row, rounded border)
│   ├── left_icon          (Image, optional)
│   ├── text_area          (TextArea, single-line, flex-grow)
│   └── right_icon_btn     (ImageButton, optional clear — auto-hidden when text empty)
├── empty_state            (Obj, hidden by default; icon + text)
├── no_results             (Obj, hidden by default; icon + text)
├── initial_loading_slot   (Obj, hidden by default; caller creates children)
├── initial_error_slot     (Obj, hidden by default; caller creates children)
└── result_container       (Obj, flex column, scrollable, hidden by default)
    ├── row_0              (Obj, flex row: select_indicator + cell_0..N)
    ├── row_N
    ├── footer_loading_slot (Obj, hidden by default; caller creates children)
    └── footer_error_slot   (Obj, hidden by default; caller creates children)
```

- Exactly one of `empty_state`, `no_results`, `initial_loading_slot`, `initial_error_slot`, `result_container` is visible at a time.
- `footer_loading_slot` and `footer_error_slot` live inside `result_container` so they scroll with the rows; they are mutually exclusive (showing one hides the other).
- All four slot containers are created **empty** by `SearchBar::new`. The caller fills them by constructing widgets with the slot as parent (e.g., `MySpinner::new(bar.footer_loading_slot())`). SearchBar never reparents anything.

**Slot ownership contract (reserved capabilities — caller must NOT touch on the slot containers themselves):**

| Capability | Owned by SearchBar (forbidden to caller) | Caller may freely use on slot **children** |
|---|---|---|
| `LV_OBJ_FLAG_HIDDEN` on the slot container | ✓ — SearchBar drives via state machine | ✓ on children |
| `lv_obj_set_user_data` on the slot container | ✓ — reserved for future internal bookkeeping (currently unused) | ✓ on children |
| `lv_obj_delete` on the slot container | ✓ — slots live for the SearchBar's lifetime | ✓ on children |
| Reparenting the slot container | ✓ — never reparented | ✓ children may be reparented elsewhere by caller |
| Position / size / layout of the slot container | ✓ — derived from SearchBar layout | ✓ on children |
| Style / decoration of the slot container | ✗ — caller may style if desired | ✓ |
| Adding/removing children inside the slot | ✗ — caller-owned domain | ✓ |
| Event callbacks on slot **children** | ✗ — caller-owned | ✓ |

**Slot default layout & styling.** Each slot container is a plain `Obj` with:
- `lv_obj_set_layout(slot, LV_LAYOUT_FLEX)` + `lv_obj_set_flex_flow(slot, LV_FLEX_FLOW_COLUMN)` + flex align center.
- Default size: `LV_PCT(100) × LV_SIZE_CONTENT`.
- No padding, no border, no background; transparent so caller-drawn children render naturally.
- `LV_OBJ_FLAG_HIDDEN` set at construction.

Caller adds children with the slot as parent; the children are flex-stacked vertically and centered. Caller may override the slot's layout (e.g., switch to grid, change alignment) — this falls under "Style / decoration" in the ownership table above and is permitted.

The slot accessors return `&dyn Widget`. The ownership contract above is **documented behavior**, not type-enforced. Violations (e.g., caller hides the slot themselves) put the state machine into an inconsistent state; debug builds `debug_assert!` slot visibility **immediately after each state transition** (before returning control to the caller) — assertions are not re-checked between transitions, so caller mutations after a transition will not spuriously fire. A future revision may return a narrower `SlotParent` handle exposing only `as_widget_for_parenting()` to make this type-safe.

## 3. Public API Surface

```rust
use lvgl_dsl::lvgl::prelude::*;

let bar = SearchBar::new(&screen)
    // Input row
    .placeholder("Search by name, phone or ID")
    .left_icon(ImageSrc::Symbol("\u{F002}"))
    .right_icon_clear(ImageSrc::Symbol("\u{F00D}"))
    .max_length(64)
    .auto_focus(true)

    // Empty / no-results state
    .empty_state_text("Start typing to find a recipient")
    .empty_state_icon(ImageSrc::Symbol("\u{F05A}"))
    .no_results_text("No matches found")
    .no_results_icon(ImageSrc::Symbol("\u{F119}"))

    // Schema-driven columns
    .column(ColumnConfig::new()
        .weight(2)
        .font(&Font::montserrat_30())          // &'static Font
        .color(Color::black())
        .long_mode(LongMode::Dot)
        .highlight(true))                      // recolor only this column on match
    .column(ColumnConfig::new().weight(1).color(Color::hex(0x666666)))
    .column(ColumnConfig::new().weight(1).color(Color::hex(0x666666)))

    // Highlight customisation (color-only — no bold in v1)
    .highlight_color(Color::hex(0xF26B1F))
    .case_sensitive(false)
    .match_mode(MatchMode::Substring)          // Substring | Prefix | WordPrefix

    // Selection
    .selection_mode(SelectionMode::Single)     // None | Single | Multi
    .selected_row_bg(Color::hex(0xFFF0E6))
    .selected_indicator_color(Color::hex(0xF26B1F))
    .indicator_size(24)
    .indicator_border_width(2)

    // Pagination & debounce
    .debounce_ms(250)
    .min_query_len(1)
    .load_more_threshold_rows(3)
    .load_more_threshold_px(120)               // fallback when measured row height = 0
    .max_rows(usize::MAX);

bar.attach_keyboard(&keyboard);                // last-bind-wins; SearchBar tracks the kb pointer
bar.detach_keyboard();                         // calls lv_keyboard_set_textarea(kb, NULL); also auto-called on LV_EVENT_DELETE

// Caller fills slot containers — SearchBar never reparents:
let _spinner_initial = MySpinner::new(bar.initial_loading_slot());
let _err_initial     = MyErrorView::new(bar.initial_error_slot());
let _spinner_footer  = MySpinner::new(bar.footer_loading_slot());
let _err_footer      = MyErrorView::new(bar.footer_error_slot());

// Token-aware async callbacks (static fn pointers)
bar.on_query_changed(|token: RequestToken, q: &str| { /* fetch(q, token) */ });
bar.on_load_more(|token: RequestToken, page_index: u32| { /* fetch_next_page(token, page_index) */ });
bar.on_select(|row_id: u64, selected: bool| { /* selection toggled */ });
bar.on_query_cleared(|| { /* any path to empty/too-short: clear, backspace, clear_query() */ });
bar.on_retry(|token: RequestToken, q: &str| { /* re-issue the in-flight query */ });

// Token-required data lifecycle
bar.set_loading(token, true);
bar.set_results(token,
                &[Row::cells(&["Alice", "+1 555…", "ID-001"]).id(1)],
                /* has_more */ true);
bar.append_results(token, &more_rows, /* has_more */ false);
bar.set_loading(token, false);
bar.set_error(token, true);                    // implicitly hides loading slot at this placement
bar.set_error(token, false);

// Token-free convenience (no async coupling)
bar.cancel_pending_load_more();                // re-arms pagination on next scroll
bar.clear_query();                             // fires on_query_cleared; suppresses on_query_changed
bar.clear_selection();
bar.set_text(initial_value);                   // fires on_query_changed (debounced) like a keystroke
bar.emit_retry();                              // fires on_retry(current_token, current_query); caller's
                                               // error-slot child wires its own button click here

// Introspection
let _ = bar.selected_row_ids();                // alloc::vec::Vec<u64>
let _ = bar.is_selected_id(row_id);
let _ = bar.selected_count();
let _ = bar.query_text();                      // alloc::string::String
let _ = bar.current_token();                   // RequestToken — caller bookkeeping
let _ = bar.stale_drop_count();                // usize — counts setters dropped due to stale token

// Inner widgets for advanced styling
bar.text_area();                 // &dyn Widget
bar.input_container();           // &dyn Widget
bar.result_container();          // &dyn Widget
bar.initial_loading_slot();      // &dyn Widget — also the parent for caller-created children
bar.initial_error_slot();        // &dyn Widget
bar.footer_loading_slot();       // &dyn Widget
bar.footer_error_slot();         // &dyn Widget
```

`Row` API:

```rust
pub struct Row { /* private */ }

impl Row {
    pub fn cells(cells: &[&str]) -> Self;        // clones into String
    pub fn cells_owned(cells: Vec<String>) -> Self;
    pub fn id(self, id: u64) -> Self;            // optional caller-supplied id
}
```

`RequestToken` is a `Copy` newtype around `u64`. Callers may store, compare, and pass it freely; SearchBar uses it solely as an equality check against its current generation.

All callbacks are `fn` pointers with no captured state. Caller state lives in `static` / `AtomicUsize` etc., consistent with the rest of the crate. Documented contract: callback `&str` arguments must not be retained past return.

**Retry ownership.** SearchBar does not own any built-in retry control or button. The `on_retry` callback fires only when the caller invokes `bar.emit_retry()` programmatically — typically from a button click handler on a child the caller has placed inside `initial_error_slot()` or `footer_error_slot()`. SearchBar's role is restricted to:
- Showing/hiding the error slot (per the state machine).
- Carrying the `current_token` and current raw query into the `on_retry` callback.

This keeps the slot ownership boundary clean (§2): SearchBar never inspects or touches caller-created widgets inside the slots. Recommended caller pattern:

```rust
let retry_btn = MyRetryButton::new(bar.footer_error_slot());
retry_btn.on_click(|| BAR.with(|b| b.emit_retry()));   // BAR is a caller-side static
```

## 4. State Machine and Token Semantics

### States

`Empty`, `Loading`, `Results`, `NoResults`, `Error`.

```
                    ┌─────────┐
   start ───────────►  EMPTY  │  query trims to "" or canonical.chars().count() < min_query_len
                    └────┬────┘
        debounce fires   │ for new valid query
                         ▼
                    ┌─────────┐    set_results(token, [], _) → normalize has_more=false
                    │ LOADING │ ─────────────────────────────► ┌────────────┐
                    └────┬────┘                                │ NO_RESULTS │
                         │ set_results / append_results        └────┬───────┘
                         │ with !rows.is_empty()                    │ append_results(non-empty)
                         ▼                                          │ promotes here
                    ┌─────────┐ ◄── append_results ─────────────────┘
                    │ RESULTS │
                    └────┬────┘
        query → ""       │
                         └──────────► EMPTY ◄────────────────────────┘

   any non-EMPTY state ── set_error(token, true) ──► ERROR
   ERROR ── set_error(token, false) ──► previous data state (Results/Loading)
   ERROR ── new query / clear ──► EMPTY → LOADING
```

### Visibility per state

| State | empty_state | no_results | result_container | initial_loading_slot | initial_error_slot | footer_loading_slot | footer_error_slot |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| Empty                      | ✓ | – | – | – | – | – | – |
| Loading (no rows yet)      | – | – | – | ✓ | – | – | – |
| Loading (rows present)     | – | – | ✓ | – | – | ✓ | – |
| Results                    | – | – | ✓ | – | – | – | – |
| NoResults                  | – | ✓ | – | – | – | – | – |
| Error (no rows yet)        | – | – | – | – | ✓ | – | – |
| Error (rows present)       | – | – | ✓ | – | – | – | ✓ |

### Token semantics (revised — fixes v1 timing bug, v2 stale-render race, AND v3 canonicalization contradiction)

**`canonical_query(text, case_sensitive)`** is a single function used everywhere a query is compared:

```rust
pub(crate) fn canonical_query(s: &str, case_sensitive: bool) -> alloc::string::String {
    let trimmed = s.trim();
    if case_sensitive { trimmed.to_owned() } else { trimmed.to_lowercase() }
}
```

It is the canonical form for: dedupe in `fire_query_changed_now`, condition 2 of the acceptance gate, the `min_query_len` check (`canonical.chars().count()`), and the empty-query state pivot (`canonical.is_empty()`). **Exactly one normalization rule** across the widget.

Generation/token rules:

- `Inner.generation: u64` increments **only when `on_query_changed` is actually fired** (i.e., the debounce timer fires AND duplicate-suppression does NOT apply AND `min_query_len` is met against the canonical form).
- `Inner.last_fired_canonical: String` stores `canonical_query(text)` at the most recent fire.
  - On `clear_query()` and any "query → empty" transition: `last_fired_canonical` is **reset to the empty string** AND generation is bumped. This guarantees that "type → fire → clear → retype same string" is **not** dedupe-suppressed (retype produces non-empty canonical that differs from `""`).
- A fresh `RequestToken(generation)` is minted *at fire time* and passed to `on_query_changed` and to all subsequent `on_load_more` for this query family.

**Two-condition acceptance gate.** A response setter (`set_results`, `append_results`, `set_loading(_, true)`, `set_error(_, true)`) is accepted only if **both**:

1. `token == state.current_token()` — token matches the latest fired query, AND
2. `last_fired_canonical == canonical_query(current_textarea_text(), case_sensitive)` — textarea **at the moment the response arrives** still canonicalizes to what the in-flight query was for.

If condition 2 fails, the user has already typed further (debounce pending) but the next debounce fire has not yet bumped the token; without this gate the in-flight response would render under stale UI input. On condition-2 failure: `stale_drop_count += 1`, no state change, no callback. Performance: condition 2 reads the textarea via `lv_textarea_get_text` (returns `*const c_char`, O(1)) and runs canonicalization (one `trim` + optional `to_lowercase`), bounded by query length (typically < 64 chars). Documented as a hot path consideration.

`set_loading(_, false)` and `set_error(_, false)` only check condition 1 — turning off a flag that may already be off is harmless and these may legitimately race a query change.

Concrete trace — v1 bug case (still passes):

```
state: generation=5, last_fired_canonical="alice", T5 in flight
user types "alicex"     → debounce timer starts, no fire yet
user backspaces "alice" → debounce timer reset, no fire yet
debounce fires          → canonical("alice")=="alice"==last_fired → suppressed
                          generation NOT bumped → T5 still valid
T5 response arrives     → token ✓; canonical("alice")==last_fired ✓ → ACCEPTED
```

Concrete trace — v2 stale-render race (caught):

```
state: gen=5, last_fired_canonical="alice", T5 in flight
user types "alicex"     → debounce timer starts, no fire yet
T5 response arrives     → token ✓; canonical("alicex")!="alice" → DROPPED, stale_drop_count++
debounce fires "alicex" → canonical "alicex"!=last_fired → fire → gen=6, T6 issued
T6 response arrives     → token ✓; canonical("alicex")==last_fired ✓ → ACCEPTED
```

Concrete trace — v3 clear-then-retype (caught):

```
state: gen=5, last_fired_canonical="alice", T5 in flight
user clears             → gen=6, last_fired_canonical="", T5 invalidated
user retypes "alice"    → debounce fires
                          canonical("alice")="alice" != last_fired "" → NOT suppressed
                          gen=7, T7 issued, on_query_changed fires
T5 late response        → token mismatch → DROPPED, stale_drop_count++
T7 response arrives     → ACCEPTED
```

Edge case — trailing whitespace edit (intentional behavior):

```
state: last_fired_canonical = "alice"
user types " " at end (now "alice ") → debounce starts
T(prev) response arrives → canonical("alice ")="alice"==last_fired ✓ → ACCEPTED
debounce fires           → canonical "alice"==last_fired → suppressed (no extra fire)
```

Trailing-only whitespace is intentionally not a "different query". Whitespace-significant matching is out of scope for v1.

Setters drop on stale token (sketch — actual mutation goes through `dispatch` per §7):

```rust
fn apply_set_results(s: &mut InnerState, token: RequestToken,
                     rows: Vec<Row>, has_more: bool) {
    if token != s.current_token() { s.stale_drop_count += 1; return; }
    let canonical_now = canonical_query(&textarea_get_text(s.text_area_ptr),
                                         s.case_sensitive);
    if canonical_now != s.last_fired_canonical { s.stale_drop_count += 1; return; }
    let has_more = !rows.is_empty() && has_more
                && s.row_count + rows.len() < s.max_rows;
    // …apply, then transition_to_results(s, has_more)
}
```


### Normalization rules (close ambiguities the rubber-duck flagged)

- `set_results(token, rows, has_more)` with `rows.is_empty()`: SearchBar forces `has_more = false` regardless of caller value, transitions to `NoResults`. No auto-fill is triggered for empty results.
- `set_results` / `append_results` whose new total exceeds `max_rows`: SearchBar truncates to fit and forces `has_more = false`. Pagination stops; no further `on_load_more` will fire until query changes.
- `set_loading(token, true)` and `set_error(token, true)` are mutually exclusive at each placement. Calling either implicitly hides the matching slot at the same placement.
- Programmatic `set_text(t)` triggers the same `LV_EVENT_VALUE_CHANGED` path as a keystroke; debounce + token semantics apply identically.
- **`append_results` outside `Results` state** (explicit decision):
  - From `Loading` (no rows yet) with non-empty rows AND condition gate passes → `append_results` is treated as the **first** `set_results` for this query: rows are installed, transition `Loading → Results`. Rationale: a caller may stream results without an explicit "first batch" set call.
  - From `Loading` (rows present, i.e. paginating) → standard append behavior.
  - From `NoResults` → `append_results` with non-empty rows promotes to `Results` (transition `NoResults → Results`); empty append is a no-op (`stale_drop_count` not incremented; documented as a benign caller mistake).
  - From `Empty` or `Error` → `append_results` is dropped silently and `stale_drop_count += 1` (the token check usually catches this anyway, since these states involve generation bumps).
  - From `Results` → standard append with auto-fill / divergence-guard logic per §7.
- **`set_error(token, true)` source-state restrictions** (explicit decision):
  - Legal from `Loading` and `Results` states (with or without rows).
  - **Forbidden from `Empty` and `NoResults`** — there is no in-flight request to fail. Calling `set_error(_, true)` from these states is a no-op + `stale_drop_count++`. This avoids ambiguity around "what does set_error(false) restore to?" when there was no prior data state.
  - Legal from `Error` itself (idempotent; just refreshes the visible error slot).
- **`set_error(token, false)` restores to**:
  - `Results` if rows are present (regardless of whether prior was Loading or Results).
  - `Loading` if no rows AND no `pending_load_more` was cleared by the error.
  - **`Empty` if the query has been cleared** while error was visible (rare race).
  - The widget tracks the pre-error state in `pre_error_state: Option<State>` to make this deterministic.

### Query-driven transitions (automatic)

The widget treats canonical-query length against `min_query_len` as the primary classifier. Three "buckets":

- **EMPTY**: `canonical.is_empty()` — the user's effective query is the empty string.
- **TOO_SHORT**: `0 < canonical.chars().count() < min_query_len` — there is text but not enough to search.
- **VALID**: `canonical.chars().count() >= min_query_len`.

Transitions:

- Query → **EMPTY** (clear button, keyboard backspace-to-empty, `clear_query()`, or `set_text("")`):
  - Drop row buffer, clear selection, transition to `Empty` state, fire `on_query_cleared()`.
  - **Does not** fire `on_query_changed("")`.
  - Cancel pending debounce timer; clear `pending_load_more`.
  - Bump generation; reset `last_fired_canonical = ""`. Any in-flight response is now stale.
- Query → **TOO_SHORT** (typed at least one char but below `min_query_len`):
  - Treated **exactly like EMPTY** for state-machine purposes:
    - Drop row buffer, clear selection, transition to `Empty` state.
    - Fire `on_query_cleared()` (single hook for "no longer searchable" — caller doesn't need to distinguish empty from too-short for v1; documented).
    - Cancel pending debounce timer; clear `pending_load_more`.
    - Bump generation; reset `last_fired_canonical = ""`.
  - Rationale: a too-short query is semantically "no current search". Emitting `on_query_changed("ab")` only to have the caller manually filter by length would push this concern to every caller. v1 makes the boundary firmly internal.
  - Right-icon clear button stays visible (the field is non-empty); user can still clear with one tap.
- Query → **VALID** from any prior bucket (via keystroke, `set_text`, etc.):
  - Wait for debounce. On fire (subject to dedupe via `canonical_query`), bump generation, clear `pending_load_more`, mint token, fire `on_query_changed(token, raw_text)`. Visual unchanged until caller responds.

The right-icon clear button is auto-shown when raw query is non-empty (regardless of bucket), auto-hidden when raw query is empty (only if `right_icon_clear` was configured).

## 5. Selection Model

```rust
pub enum SelectionMode { None, Single, Multi }
```

- Per-row `select_indicator` is a child `Obj` (24×24 default).
  - `None` → indicator hidden (`LvObjFlag::Hidden`).
  - `Single` → circle (`LV_RADIUS_CIRCLE`) with 2 px border. Selected → fill with `selected_indicator_color` and add an inner filled circle child.
  - `Multi` → 4 px-radius rounded square. Selected → fill + a `Label` set to `LV_SYMBOL_OK`.
- Selected row container additionally gets `selected_row_bg` background color.

Click handling (`LV_EVENT_CLICKED` on each row container — LVGL filters out scroll gestures):

- `None` → fire `on_select(row_id, true)` (one-shot semantics; nothing stored).
- `Single` → clear all `selected[*]`, set `selected[idx] = true`, repaint previously-selected and newly-selected rows, fire `on_select(row_id, true)`.
- `Multi` → toggle `selected[idx]`, repaint that row, fire `on_select(row_id, selected[idx])`.

Selection storage is `Vec<bool>` indexed by row position; `Vec<u64>` parallel array stores the row IDs. Callbacks always carry `row_id`, never the index.

Lifetime across data changes:

- `set_results(...)` clears selection (length reset, all `false`). All previous row IDs are forgotten.
  **Selection-clear callback rule:** clearing is **silent** — `on_select(id, false)` is **NOT** fired for previously-selected rows. Rationale: the row IDs are forgotten on `set_results`, so the callback would report on stale identity. Callers that need to mirror selection state must call `selected_row_ids()` before issuing the next query, or track selection in their own state. This is documented on `on_select`.
- `append_results(...)` extends with `false`s; existing entries preserved by index. Existing row IDs remain valid. Selection is **not** touched.
- `clear_selection()` clears all and repaints affected rows. Also **silent** — no `on_select(_, false)` callbacks fire (consistent with `set_results`).
- A new query (any path that bumps generation) implicitly clears selection on the next `set_results`. There is no separate `on_query_changed → selection cleared` callback; selection survives until results are replaced.

Public getters: `selected_row_ids() -> Vec<u64>`, `is_selected_id(u64) -> bool`, `selected_count() -> usize`.

## 6. Highlight Rendering (v1)

```rust
pub enum MatchMode { Substring, Prefix, WordPrefix }
pub enum LongMode  { Dot, Wrap, Scroll, Clip }   // → lv_label_set_long_mode
```

`pub fn find_match(text: &str, query: &str, mode: MatchMode, case_sensitive: bool) -> Option<(usize, usize)>` — returns the byte range of the first match (UTF-8 char-boundary safe), or `None`.

### Cell rendering

- One `Label` per cell.
- If `column.highlight && !query.is_empty() && find_match.is_some()`:
  - Build a recolor-marked string covering the **whole** cell text.
  - Enable recolor on the Label via `lv_label_set_recolor(label, true)` (LVGL v9.2 API).
  - Set text via `lv_label_set_text` with the marked-up content.
- Otherwise: set raw text via `lv_label_set_text` with `lv_label_set_recolor(label, false)`.

Long-text mode is set per column via `lv_label_set_long_mode` (default `Dot`). Truncation, ellipsis, and wrapping work normally because each cell is still a single `Label`.

Highlights are rebuilt only when `set_results` / `append_results` is invoked — not on every keystroke.

### Markup builder (full-text escape)

LVGL's recolor parser treats `#` as significant; `##` is the literal-`#` escape. The builder must escape `#` over the **entire** rendered cell text — prefix and suffix as well as the matched substring — otherwise an existing `#` in cell data corrupts subsequent rendering.

Pseudocode:

```rust
fn build_marked(text: &str, span: (usize, usize), color: Color) -> String {
    let (s, e) = span;
    let extra = count_hashes(text);                          // O(text.len())
    let cap   = text.len() + extra
              + 2 /* "#"+" " */ + 6 /* RRGGBB */ + 1 /* '#' close */;
    let mut out = String::with_capacity(cap);
    push_escaped(&mut out, &text[..s]);
    out.push('#');
    out.push_str(&color.as_hex_rrggbb());                    // e.g. "F26B1F"
    out.push(' ');
    push_escaped(&mut out, &text[s..e]);
    out.push('#');
    push_escaped(&mut out, &text[e..]);
    out
}

// push_escaped: copies bytes; every '#' becomes "##".
```

`String::with_capacity(...)` with the precomputed cap is a **design requirement**, not an implementation detail — it must appear in the test plan and code reviews.

UTF-8: `find_match` returns offsets at char boundaries; the markup builder operates on byte slices that are always char-aligned. RTL correctness is delegated to LVGL's bidi pipeline.

## 7. Pagination, Debounce, and Inner-State Internals

### Pagination

Listen for `LV_EVENT_SCROLL` on `result_container`:

```
on scroll:
    bottom_dist = lv_obj_get_scroll_bottom(result_container)
    if bottom_dist <= threshold_px
       AND state == Results
       AND has_more
       AND !pending_load_more
       AND user_scroll_observed:
        emit_load_more()
```

`emit_load_more()` does:

```
pending_load_more = true
page_index += 1
show footer_loading_slot               // §10 invariant: loading visible during outstanding request
fire on_load_more(current_token, page_index)
```

- `threshold_px` derivation:
  - **Primary**: `load_more_threshold_px` (default 120 px) — pure pixel threshold against `lv_obj_get_scroll_bottom`. Always honored; works correctly with variable-height rows.
  - **Optional add-on**: if `load_more_threshold_rows > 0` AND `measured_row_height > 0`, the effective threshold is `max(load_more_threshold_px, load_more_threshold_rows * measured_row_height)`. Variable-height rows: `measured_row_height` is the **height of the most recently rendered row** (sampled in `set_results` / `append_results` after `lv_obj_update_layout`), which is a heuristic — callers with highly variable rows should rely on the pixel threshold and set `load_more_threshold_rows(0)`.
- `user_scroll_observed`: set the first time `lv_obj_get_scroll_top > 0` is observed. Prevents the "near-bottom always true" bug for short lists.
- `pending_load_more` cleared on `append_results`, `set_error(_, true)`, query change, or `cancel_pending_load_more()`.
- `page_index` is monotonic per query; resets to 0 on every generation bump. Caller may ignore it but it disambiguates concurrent paging if they care.
- `current_token` is the token associated with the active query.

### Auto-fill

After every `set_results(token, rows, has_more)` or `append_results(token, rows, has_more)` where the token matched and `has_more == true`:

- Run `lv_obj_update_layout(result_container)`.
- If `lv_obj_get_scroll_bottom(result_container) == 0` (content does not overflow viewport), call `emit_load_more()` once.
- This loop continues each time the caller calls `append_results` until either (a) `has_more=false`, (b) the viewport becomes full, (c) `max_rows` cap is hit (which forces `has_more=false`).

Divergence guard: if `append_results` adds zero new rows AND `has_more=true`, SearchBar issues exactly one more `emit_load_more` and then forces `has_more=false`, leaving `pending_load_more=true` until the caller resolves it. This is bounded by construction — at most one extra fire per "zero-progress" reply.

**Divergence-guard reset rules** (explicit):

- The "extra-fire used" flag is per-`(generation, page_index)` and resets to `false` on:
  - any generation bump (new query), OR
  - any `append_results` that adds **at least one** new row (progress made).
- The flag does **not** reset on `cancel_pending_load_more()` alone (which only clears the in-flight bit, not the divergence accounting).
- Within a single query/page, only one extra zero-progress fire is ever attempted; subsequent `append_results(_, [], has_more=true)` for the same page just force `has_more=false` immediately and do not re-fire.

### Debounce

Listen for `LV_EVENT_VALUE_CHANGED` on the inner TextArea:

```
on text changed:
    if debounce_ms == 0:
        fire_query_changed_now(text)              // subject to dedupe + min_query_len
    else:
        if no timer:  create one-shot lv_timer with period = debounce_ms, repeat_count = 1
        else:         lv_timer_set_period; lv_timer_reset; lv_timer_resume
```

`fire_query_changed_now` (uses the single canonical_query rule from §4):

```
let raw   = current_textarea_text();
let canon = canonical_query(raw, case_sensitive);
if canon.chars().count() < min_query_len { return; }      // no fire
if canon == last_fired_canonical          { return; }     // dedupe — token unchanged
generation += 1
page_index = 0
last_fired_canonical = canon.clone()
fire on_query_changed(RequestToken(generation), &raw)     // raw passed to callback
```

The callback receives the **raw** textarea text (so callers see exactly what the user typed). All internal comparisons use the canonical form.

- One-shot via `lv_timer_set_repeat_count(t, 1)`; LVGL auto-deletes after fire.
- Timer `user_data` = `*const Inner` (raw pointer; deref pattern below).
- Cancelled (paused / deleted) on `clear_query()`, on right-icon clear, when textarea becomes empty.

### Inner-state model: `RefCell<InnerState>` + bounded action queue (Model A — `apply` never fires user callbacks)

This replaces the v1 "raw `&mut Inner`" plan, which was UB under re-entrant LVGL events. **Model A** invariant (chosen over Model B for simplicity in single-threaded LVGL):

> **`apply_*` functions only mutate `InnerState` and enqueue any user-callback emissions as `Action::EmitCallback(...)` items in `state.queue`. User callbacks fire ONLY in step (d) of the drain loop, after the borrow has been dropped.**

Consequences:

- Single-threaded LVGL + this invariant ⇒ `RefCell::try_borrow_mut` cannot fail (no other thread, no nested `apply` on the same stack while the borrow is held).
- The `Err(_)` arm of `try_borrow_mut` is therefore **dead code in production**; the implementation `debug_assert!`s it never fires (= "we violated the Model A invariant somewhere"), accounts it, and returns safely.
- **There is no separate re-entry ring.** The single `state.queue: VecDeque<Action>` is the only enqueue channel; preallocated `with_capacity(QUEUE_CAP)`.
- Re-entrant pushes from a user callback (which runs with the borrow dropped) re-enter `dispatch`, succeed in `try_borrow_mut`, see `is_draining = true`, and push directly to `state.queue`. The drain loop picks them up FIFO.

```rust
pub(crate) struct Inner {
    root: *mut lv_obj_t,
    state: core::cell::RefCell<InnerState>,

    // Keyboard observer bookkeeping (outside RefCell because the
    // observer trampoline runs on the keyboard's DELETE event without
    // entering dispatch).
    attached_keyboard: core::cell::Cell<Option<*mut lv_obj_t>>,
    attached_keyboard_delete_token: core::cell::Cell<Option<EventCbHandle>>,

    // Liveness flag for the post-deletion guard (§7.liveness below).
    alive: core::cell::Cell<bool>,
}

pub(crate) struct InnerState {
    // configuration
    columns: Vec<ColumnConfig>,
    cb: Callbacks,
    debounce_ms: u32,
    case_sensitive: bool,
    min_query_len: usize,
    text_area_ptr: *mut lv_obj_t,
    // …

    // data
    rows: Vec<Row>,
    selected: Vec<bool>,
    row_ids: Vec<u64>,

    // tokens / pagination
    generation: u64,
    last_fired_canonical: String,
    page_index: u32,
    pending_load_more: bool,
    user_scroll_observed: bool,
    divergence_extra_used: bool,           // per-(generation, page_index)

    // Action queue (single source of FIFO truth)
    queue: VecDeque<Action>,                // VecDeque::with_capacity(QUEUE_CAP)
    queue_overflow: usize,                  // counter: action dropped because queue full
    queue_overflow_borrow: usize,           // counter: Err arm hit (Model A violation)
    is_draining: bool,                      // drain re-entry guard

    // misc
    stale_drop_count: usize,
    debounce_timer: Option<TimerHandle>,
    state_kind: State,
    measured_row_height: i32,
}

const QUEUE_CAP: usize = 16;

pub(crate) enum Action {
    // State-mutating actions (handled by apply()):
    SetResults  { token: RequestToken, rows: Vec<Row>, has_more: bool },
    AppendResults { token: RequestToken, rows: Vec<Row>, has_more: bool },
    SetLoading  { token: RequestToken, val: bool },
    SetError    { token: RequestToken, val: bool },
    ClearQuery,
    SetText     { text: alloc::string::String },
    ClearSelection,
    CancelPendingLoadMore,
    EmitRetry,                              // §3 emit_retry() public method

    // Pure callback emission (apply enqueues these instead of calling
    // user code synchronously). Drain step (d) invokes them.
    EmitCallback(CallbackEvent),
}

pub(crate) enum CallbackEvent {
    QueryChanged  { token: RequestToken, raw_text: alloc::string::String },
    QueryCleared,
    LoadMore      { token: RequestToken, page_index: u32 },
    Select        { row_id: u64, selected: bool },
    Retry         { token: RequestToken, raw_text: alloc::string::String },
}
```

Setter pattern (Model A — single drain loop, no ring):

```rust
fn set_results(&self, token: RequestToken, rows: &[Row], has_more: bool) {
    self.dispatch(Action::SetResults {
        token,
        rows: clone_rows(rows),     // owned upfront
        has_more,
    });
}

// Single entry point for every state-mutating operation.
fn dispatch(&self, action: Action) {
    let inner = self.inner();

    // §7.liveness — fast post-deletion guard (see below).
    if !inner.alive.get() { return; }

    let mut s = match inner.state.try_borrow_mut() {
        Ok(s) => s,
        Err(_) => {
            // MODEL A VIOLATION. apply_* must never call code that
            // re-enters dispatch while holding the borrow.
            // In release: count and return safely.
            // In debug: panic to catch it during development.
            debug_assert!(false,
                "RefCell held during dispatch — Model A invariant violated. \
                 apply_* must only mutate state + enqueue Action::EmitCallback");
            // Cell counter on Inner since we cannot enter the RefCell:
            // increment via separate Cell<usize> if needed for telemetry.
            return;
        }
    };

    if s.is_draining {
        // Re-entered from a user callback (which runs with borrow dropped);
        // we are between drain iterations, currently holding the borrow.
        // Just enqueue — the active drain will pick this up.
        push_or_overflow(&mut s, action);
        return;
    }

    s.is_draining = true;
    apply(&mut s, &action);

    // Drain loop: pop one → drop borrow → fire callback (if any) →
    // re-acquire → apply if state-mutating. Bounded by QUEUE_CAP per
    // re-entrant burst.
    loop {
        let next = s.queue.pop_front();
        match next {
            None => { s.is_draining = false; break; }
            Some(a) => {
                drop(s);                                  // (c)
                fire_callbacks_for(self, &a);             // (d)
                s = inner.state.try_borrow_mut()
                        .expect("Model A: borrow always free here");
                apply(&mut s, &a);                         // (no-op for EmitCallback)
            }
        }
    }
}

fn push_or_overflow(s: &mut InnerState, action: Action) {
    if s.queue.len() < QUEUE_CAP {
        s.queue.push_back(action);
    } else {
        s.queue_overflow += 1;
        debug_assert!(false, "queue overflow — raise QUEUE_CAP or audit re-entrancy");
    }
}

fn apply(s: &mut InnerState, a: &Action) {
    match a {
        Action::SetResults { token, rows, has_more } => {
            apply_set_results(s, *token, rows, *has_more);
            // Model A: any callback emission goes via the queue:
            // (set_results does NOT directly fire any user callback —
            // it is the result of a user response, not an event.)
        }
        Action::AppendResults { token, rows, has_more } => {
            apply_append_results(s, *token, rows, *has_more);
            // May enqueue Action::EmitCallback(LoadMore {…}) for auto-fill.
        }
        Action::SetLoading { token, val }    => apply_set_loading(s, *token, *val),
        Action::SetError   { token, val }    => apply_set_error(s, *token, *val),
        Action::ClearQuery                    => apply_clear_query(s),  // enqueues EmitCallback(QueryCleared)
        Action::SetText { text }              => apply_set_text(s, text),
        Action::ClearSelection                => apply_clear_selection(s),
        Action::CancelPendingLoadMore         => { s.pending_load_more = false; }
        Action::EmitRetry                     => apply_emit_retry(s),    // enqueues EmitCallback(Retry {…})
        Action::EmitCallback(_)               => { /* no state mutation; handled in drain step (d) */ }
    }
}
```

**Invariants (Model A):**

- `apply_*` functions are PURE state mutators + queue pushers. They MUST NOT call any function that re-enters `dispatch`.
- All user callbacks emit through `state.queue.push_back(Action::EmitCallback(...))`. Drain step (d) invokes the user code with the borrow dropped.
- `is_draining=true` is the ONLY legitimate state for a re-entrant `dispatch` call to find. It will succeed on `try_borrow_mut` (because the active drain dropped the borrow before firing the callback) and push to the queue.
- `Err(_)` from `try_borrow_mut` indicates a Model-A violation (some `apply_*` re-entered `dispatch` while holding the borrow). Debug builds panic; release builds count and return.
- `queue` is preallocated `with_capacity(QUEUE_CAP)`. Overflow drops the action and counts.
- Worst-case work per outer dispatch is bounded: each iteration consumes one action, and re-entrant pushes are capped at `QUEUE_CAP`. No livelock.

**§7.liveness — post-deletion guard.**

`Inner.alive: Cell<bool>` defaults `true`. `LV_EVENT_DELETE` step 0 (added before keyboard detach) sets `alive=false`. All public setter methods on `SearchBar` consult `inner.alive.get()` BEFORE acquiring the RefCell; if false, the setter is a no-op (with `stale_drop_count++` for visibility). This addresses late async responses that arrive after the SearchBar has been destroyed but the caller still holds a `&SearchBar` reference — common when the screen is unloaded mid-fetch.

Note: the `&SearchBar` reference itself can only outlive the underlying LVGL object if the caller violates the documented "lifetime = parent screen" rule. The `alive` flag is a defense-in-depth guard, not a license to use SearchBar after its parent is gone. **Crate-wide invariant** (documented in `DSL_REFERENCE.md`): all widget methods are invalid once the parent screen has been deleted. SearchBar's `alive` flag turns most such uses from UB into a safe no-op, but does not authorize them.

### Trampolines

Every event/timer callback follows this pattern:

```rust
unsafe extern "C" fn trampoline(e: *mut lv_event_t) {
    let user_data = lv_event_get_user_data(e);
    if user_data.is_null() { return; }
    // SAFETY: user_data was set to a Box<Inner>::into_raw and is alive
    // until LV_EVENT_DELETE NULLs it.
    let inner: &Inner = &*(user_data as *const Inner);
    inner.dispatch(EventKind::TextChanged);   // dispatch acquires RefCell
}
```

No `&mut Inner` is ever materialized through user_data. All mutation goes through `inner.state.try_borrow_mut()`.

### Lifetime / cleanup

`SearchBar::new` allocates `Box<Inner>` and stores `Box::into_raw(inner) as *mut c_void` on the LVGL root via `lv_obj_set_user_data`.

`attach_keyboard(&kb)` semantics:

- Stores `kb.raw_ptr()` into `Inner.attached_keyboard: Cell<Option<*mut lv_obj_t>>`.
- Calls `lv_keyboard_set_textarea(kb, self.text_area_ptr)`.
- **Registers a keyboard `LV_EVENT_DELETE` observer** on the keyboard object: `lv_obj_add_event_cb(kb, on_kb_deleted_trampoline, LV_EVENT_DELETE, inner_ptr)`. Trampoline body: `inner.attached_keyboard.set(None)` and clears `attached_keyboard_delete_token`. The handle is stored in `Inner.attached_keyboard_delete_token: Cell<Option<EventCbHandle>>`. This closes the keyboard-deleted-first dangling pointer hole.
- If a previous keyboard was attached: first remove the previous DELETE observer (`lv_obj_remove_event_cb_with_user_data(prev_kb, on_kb_deleted_trampoline, prev_inner_ptr)` — see §8 binding note); the previous keyboard's textarea is **not** automatically NULLed (the previous keyboard may now legitimately be bound to a different textarea by the new caller; SearchBar doesn't own the previous keyboard's binding). If `prev_kb == new_kb` (re-attach same keyboard), the observer is replaced with a fresh registration to keep semantics simple.
- `detach_keyboard()` is the explicit inverse and is idempotent: if `attached_keyboard.is_some()`, calls `lv_keyboard_set_textarea(kb, NULL)`, removes the DELETE observer, clears both Cells. No-op if `None`.

`LV_EVENT_DELETE` on the SearchBar root runs in this order:

1. **Keyboard detach** — if `attached_keyboard.get()` is `Some(kb)`:
   - The DELETE observer guarantees `kb` is still alive (it would have NULLed `attached_keyboard` on its own deletion).
   - Call `lv_keyboard_set_textarea(kb, ptr::null_mut())`.
   - `lv_obj_remove_event_cb_with_user_data(kb, on_kb_deleted_trampoline, inner_ptr)` to avoid the keyboard later firing into a freed `Inner`.
   - Clear `attached_keyboard` and `attached_keyboard_delete_token`.
2. `lv_timer_delete` if a debounce timer is alive (the spy-side "live timer set" must shrink to empty).
3. `lv_obj_set_user_data(root, ptr::null_mut())`.
4. `Box::from_raw(inner_ptr)` — drop.

LVGL deletes child objects automatically (and their event callbacks die with them); we do not iterate-and-detach. Trampolines on those children would not fire after their object is deleted in any case. Trampolines on the root guard on null user_data (step 3 above).

## 8. New C Bindings

Append to `src/lvgl/bindings.conf`:

```
# Timers (debounce)
lv_timer_create | lv_timer_set_period | lv_timer_reset
lv_timer_set_repeat_count | lv_timer_pause | lv_timer_resume | lv_timer_delete

# Scroll geometry & pagination
lv_obj_get_scroll_bottom | lv_obj_get_scroll_top
lv_obj_set_scrollbar_mode | lv_obj_scroll_to_view

# User data on objects (for Inner pointer)
lv_obj_set_user_data | lv_obj_get_user_data

# Long-text mode + recolor markup for cell labels (LVGL v9.2)
lv_label_set_long_mode
lv_label_set_recolor

# Slot child-management & visibility
lv_obj_add_flag | lv_obj_remove_flag
lv_obj_get_child_count

# Targeted event-cb removal (keyboard DELETE observer keyed by Inner*)
lv_obj_remove_event_cb_with_user_data

# Keyboard detach (NULL textarea on SearchBar delete) — already present, listed for clarity
lv_keyboard_set_textarea

# Focus management (auto-focus textarea)
lv_group_focus_obj
```

**Already-present dependencies (no new binding required, listed for traceability):**

| Symbol | Used for | Status |
|---|---|---|
| `lv_event_get_user_data` | trampoline pattern (recover `Inner*` from event) | already in bindings ✓ |
| `lv_textarea_get_text` | two-condition gate condition 2 + `query_text()` | already in bindings ✓ |
| `lv_obj_update_layout` | auto-fill viewport-fit check + first-render row-height measure | already in bindings ✓ |
| `lv_obj_send_event` | spy callback synthesis (no-op in production path) | already in bindings ✓ |
| `lv_obj_add_event_cb` | trampoline registration + keyboard DELETE observer | already in bindings ✓ |
| `lv_obj_remove_event_cb` | trampoline removal | already in bindings ✓ |
| `lv_obj_remove_event_cb_with_user_data` | targeted observer removal (keyboard DELETE observer keyed by Inner*) | **NOT YET in bindings — must be added in Step 1** |
| `lv_obj_delete` | not invoked by SearchBar; LVGL handles child cleanup | already in bindings ✓ |
| `lv_keyboard_set_textarea` | attach + detach + delete | already in bindings ✓ (used by `keyboard.rs`) |
| `lv_keyboard_set_textarea` | attach + detach + delete | **listed above** (verify presence; keyboard.rs uses it so it should already be allowlisted) |

If any "already-present" symbol is missing from `bindings.conf` at implementation time, append it; Step-1 spy tests reference each new and pre-existing symbol so a missing one fails compilation.

Plus matching desktop-sim `extern "C"` declarations and **enriched spy infrastructure** (see §10) added to `src/c_bindings.rs`.

## 9. File / Module Breakdown

```
src/lvgl/
├── mod.rs                 # add: pub mod searchbar; pub use searchbar::*;
├── prelude.rs             # add SearchBar + supporting types
├── bindings.conf          # bindings delta from §8
└── searchbar/
    ├── mod.rs             # public SearchBar struct + builder API; thin facade
    ├── inner.rs           # struct Inner — Box<Inner> wrapper + trampolines
    ├── state_cell.rs      # InnerState + RefCell pattern + try-borrow-or-queue helpers
    ├── action.rs          # Action enum + bounded VecDeque drain
    ├── token.rs           # RequestToken newtype
    ├── row.rs             # Row, ColumnConfig, SelectionMode, MatchMode, LongMode
    ├── highlight.rs       # find_match() + recolor-markup builder w/ full-text escape
    ├── debounce.rs        # DebounceTimer wrapper around lv_timer_*
    ├── pagination.rs      # scroll-end detection + auto-fill + page_index management
    ├── selection.rs       # selection state + indicator drawing
    └── state.rs           # State enum + pure transition() (visibility flips only)
```

Each module is < 300 lines with a single purpose; sibling APIs use `pub(super)`. `mod.rs` is the only file with the public surface.

`src/lvgl/searchbar/mod.rs` re-exports:

```rust
pub use self::row::{Row, ColumnConfig, SelectionMode, MatchMode, LongMode};
pub use self::token::RequestToken;
pub use self::highlight::find_match;
pub struct SearchBar { /* … */ }
```

Compile-time guarantees:

- `SearchBar: !Clone, !Copy` — enforced via inline `assert_not_impl_any!` (no external dep).
- `ColumnConfig` requires `&'static Font`.

## 10. Testing Strategy

### Step 0 — extend the spy infrastructure

The current spy in `src/c_bindings.rs`:

- Records `AddEventCb { obj, code }` only — no callback fn or user_data.
- `lv_obj_send_event` is a no-op.
- `lv_event_get_user_data` returns null.
- No timer registry.
- No per-object user_data store.

The SearchBar test plan requires driving callbacks synthetically. The first deliverable is therefore an extended desktop-sim spy with:

- **Thread-local isolation**: every new spy structure (event registry, user_data store, timer registry, scroll-injection fixtures, and the existing `LvCall` log) lives in `thread_local!` storage. `cargo test` runs with multiple threads by default; per-thread state is the only way to keep tests from corrupting each other. Per-test teardown must reset all thread-local state in the same `Drop`/`reset_obj_pool` helper.
- **Event registry** (`HashMap<*mut lv_obj_t, Vec<EventReg>>`) where `EventReg { code, cb_fn, user_data }`. `lv_obj_add_event_cb` appends; `lv_obj_remove_event_cb` removes. `spy_emit_event(obj, code)` looks up matching entries and invokes each `cb_fn` with a synthetic `lv_event_t` whose `code`, `target`, and `user_data` are recoverable through the existing `lv_event_get_*` accessors.
- **Per-object user_data store** (`HashMap<*mut lv_obj_t, *mut c_void>`). `lv_obj_set_user_data` writes; `lv_obj_get_user_data` reads.
- **Live timer registry** (`HashMap<TimerHandle, TimerReg>` where `TimerReg { period_ms, repeat_count: i32, cb_fn, user_data, paused }`). The `repeat_count` field is a signed `i32` mirroring LVGL v9.x's `lv_timer.repeat_count` (which is signed in upstream `lv_timer.h`). LVGL conventions:
  - `repeat_count == -1` (alias `LV_TIMER_INFINITY`) → fires forever; never auto-removes.
  - `repeat_count == 0` → no fires remaining; LVGL deletes the timer at next tick. The spy `spy_fire_timer` returns no-op when `repeat_count == 0`.
  - `repeat_count > 0` → that many fires remaining. After each fire, decrement; when it reaches 0, auto-remove from registry.
  - `lv_timer_set_repeat_count(t, n)` overwrites the field (cast as `i32`).
  - `lv_timer_set_repeat_count(t, 1)` is the SearchBar one-shot pattern: fires exactly once, then auto-removed.

  `spy_fire_timer(handle)` semantics:
  - If `paused == true` → no-op (fire suppressed; debug log records suppression).
  - Else if `repeat_count == 0` → no-op + auto-remove (mirrors LVGL's "stop and delete").
  - Else: invoke the callback exactly once.
    - If `repeat_count > 0`: decrement; if now 0, auto-remove.
    - If `repeat_count == -1`: leave unchanged.
  - `lv_timer_pause(h)` sets `paused=true`; `lv_timer_resume(h)` clears it.
  - `lv_timer_reset(h)` is recorded as `LvCall::TimerReset` but does not change registry state (period restart is irrelevant when fires are explicit; tests that care assert the call was recorded).
  - `lv_timer_set_period(h, ms)` updates the registered period and is recorded.
  - `lv_timer_delete(h)` removes the entry; subsequent `spy_fire_timer(h)` is a no-op + debug log.
- **Scroll injection**: `set_next_scroll_bottom(px)`, `set_next_scroll_top(px)`. The next call to `lv_obj_get_scroll_*` consumes the injected value (default 0). Per-thread.

This work lives in `src/c_bindings.rs` behind `#[cfg(any(test, all(no_zephyr, desktop_sim)))]` and is committed as Step 1 of the implementation order (§11).

**Panic-safe reset (mandatory).** Per-test reset must run **at test start**, not (only) at teardown — a panicking test would otherwise leak per-thread state into the next test scheduled on the same worker. The implementation provides a `SpyFixture` RAII struct:

```rust
pub struct SpyFixture(());
impl SpyFixture {
    pub fn new() -> Self { reset_all_thread_local_spy_state(); SpyFixture(()) }
}
impl Drop for SpyFixture {
    fn drop(&mut self) { reset_all_thread_local_spy_state(); }
}
```

Every SearchBar test starts with `let _fx = SpyFixture::new();` (also exposed via a `with_spy!` helper). The `Drop` runs even on panic (Rust unwind safety) so even fixed-up tests benefit; the `new()` reset guarantees clean state regardless of whether the previous test on this worker panicked.

### New `LvCall` variants

```
TimerCreate{period_ms, user_data, ret} / TimerSetPeriod / TimerReset
TimerSetRepeatCount / TimerPause / TimerResume / TimerDelete
ObjGetScrollBottom{obj, ret} / ObjGetScrollTop{obj, ret}
ObjSetScrollbarMode{obj, mode} / ObjScrollToView{obj, anim}
ObjSetUserData{obj, data} / ObjGetUserData{obj, ret}
LabelSetLongMode{label, mode}
LabelSetRecolor{label, en}
ObjAddFlag{obj, flag} / ObjRemoveFlag{obj, flag}
ObjGetChildCount{obj, ret}
KeyboardSetTextarea{kb, ta}
GroupFocusObj{obj}
```

### Test modules

1. **`searchbar::token::tests`** — equality, `Copy`, debug.
2. **`searchbar::row::tests`** — `cells` clones; `cells_owned` does not; `id()` builder; `ColumnConfig` defaults.
3. **`searchbar::highlight::tests`** — pure-Rust:
   - `find_match` substring / prefix / word-prefix.
   - Case-insensitive vs case-sensitive.
   - Empty query → `None`.
   - UTF-8: `"café"` inside `"Le café Paris"`, emoji, combining marks.
   - Markup builder:
     - Match in middle of plain text.
     - Cell text containing `#` in prefix, suffix, AND inside the match.
     - Empty prefix and empty suffix.
     - `String::with_capacity` invoked with the precomputed exact size (asserted via a small allocator-counter test).
4. **`searchbar::action::tests`** — drain order: pushing 3 actions inside an in-transition block applies them FIFO. Queue overflow at 16 increments `queue_overflow` and discards the 17th.
5. **`searchbar::debounce::tests`** — using the spy timer registry:
   - First keystroke: `TimerCreate{period: 250, repeat: 1, …}` recorded; firing the timer invokes the trampoline.
   - Second keystroke before fire: `TimerSetPeriod` + `TimerReset`; no second `TimerCreate`.
   - `debounce_ms == 0`: fires synchronously, no timer.
   - `clear_query()` calls `TimerPause` or removes timer.
   - **Token-timing regression test**: type "alice" (debounce fires, T1 issued); type "alicex"; backspace to "alice"; debounce fires; assert no second `on_query_changed` invocation, generation still 1, `T1` still equals `current_token()`.
6. **`searchbar::pagination::tests`** — spy + injected scroll values + spy event emission:
   - Above threshold → no fire.
   - Below threshold + `has_more` + `Results` + `user_scroll_observed` → exactly one `on_load_more` invocation with `(token, page_index=1)`.
   - Suppressed while `pending_load_more` set.
   - `append_results(matching_token, …)` clears `pending_load_more`.
   - **Auto-fill**: `set_results(t, rows, has_more=true)` with injected `scroll_bottom == 0` immediately calls `on_load_more(t, page_index=1)` AND `footer_loading_slot` becomes visible (`ObjRemoveFlag(HIDDEN)` recorded).
   - **First-render guard**: short list with `scroll_top == 0` does not fire on subsequent zero-progress scroll events.
   - **Divergence guard**: `append_results(t, [], has_more=true)` triggers exactly one extra `on_load_more` then no more. `has_more` is internally false; `pending_load_more` remains true.
   - **`max_rows` cap**: with `max_rows=10`, `set_results(t, 8 rows, has_more=true)` then `append_results(t, 5 rows, has_more=true)` → only 10 rows rendered, no further `on_load_more` fires.
   - `cancel_pending_load_more()` re-arms.
   - **`page_index` monotonicity**: 3 consecutive load-mores deliver page_index 1, 2, 3.
7. **`searchbar::selection::tests`** — using the spy event emission:
   - Single-mode: emit `LV_EVENT_CLICKED` on row 1, then row 2 → `on_select(id1, true)` then `on_select(id2, true)`; row 1's indicator repaints to "unselected".
   - Multi-mode: toggles independently.
   - `set_results` clears selection; `append_results` preserves; `clear_selection` clears.
   - Row IDs: caller-supplied ids are used in callbacks; auto-generated when not supplied.
8. **`searchbar::state::tests`** — pure: `transition()` produces correct visibility flag sequence for every state transition in §4.
9. **`searchbar::tests`** (composite integration) — using the full spy:
   - **Construction sequence**: assert exact LvCall ordering for the 9 sub-widgets created.
   - Builder chaining returns `&Self`.
   - **Stale-token drop**: `set_results(stale_token, …)` is a no-op; `stale_drop_count()` increments by 1; no row widgets created.
   - **Token round-trip**: token from `on_query_changed` matches the token accepted by `set_results`.
   - **Stale-render race (two-condition gate)**: type "alice" → debounce fires → T5; type "alicex" before debounce; `set_results(T5, rows)` arrives → DROPPED (`stale_drop_count==1`, no rows rendered, state unchanged); debounce fires for "alicex" → T6; `set_results(T6, rows2)` → accepted, rows2 rendered.
   - **Keyboard detach on delete**: `attach_keyboard(&kb)`; delete the SearchBar root; assert spy log shows `KeyboardSetTextarea(kb, NULL)` recorded **before** the `Box<Inner>` drop indicators (user_data NULL + timer delete).
   - **Explicit `detach_keyboard`**: idempotent (second call is a no-op); after detach, deleting SearchBar does not re-emit `KeyboardSetTextarea`.
   - **Selection-clear is silent**: select row in Single mode → `on_select(id, true)` recorded; call `set_results(t, new_rows, _)` → no `on_select(_, false)` recorded; `selected_row_ids().is_empty()`.
   - **`on_query_cleared` semantics**: clear button, programmatic `clear_query()`, and backspace-to-empty all fire `on_query_cleared` exactly once and **do not** fire `on_query_changed`.
   - **Empty + has_more=true normalization**: `set_results(t, [], true)` → state is `NoResults`, no `on_load_more` fires.
   - **Slot model**: caller creates a Label in `bar.footer_loading_slot()`; the Label's parent matches the slot's pointer; `set_loading(t, true)` toggles slot visibility (verified by `ObjRemoveFlag(HIDDEN)` on the slot).
   - **Loading + Error mutual exclusion** at both placements.
   - **`attach_keyboard` last-bind-wins**: attach to bar A, then to bar B → bar A's textarea is no longer the keyboard target (`KeyboardSetTextarea(kb, ta_b)` recorded after `KeyboardSetTextarea(kb, ta_a)`).
   - **`LV_EVENT_DELETE` cleanup**: live-timer set is empty after delete; root's user_data is null after delete; trampoline invoked with null user_data is a no-op (driven via spy emit).
   - **Re-entrancy**: `on_query_changed` callback that calls `set_loading` then `set_results` produces correct final visibility state (drained FIFO via the action queue).
   - **Model A invariant (apply never fires user callbacks)**: instrument a debug-only `apply_*` shim that tracks "currently inside apply" via a thread-local counter. From inside `on_query_changed`, call any setter that re-enters `dispatch`. Assert: (a) `try_borrow_mut()` always succeeds (no `Err` path taken), (b) `queue_overflow_borrow == 0`, (c) every user callback fires with the apply-counter at zero. A negative-control regression test injects a synthetic `apply_*` that intentionally invokes a user callback inline and asserts the `debug_assert!` trips.
10. **`c_bindings::tests`** — extend `reset_obj_pool` test to clear new spy state (event registry, user_data store, timer registry, injection fixtures). Reference each new bound symbol so a missing one fails compilation.

### Manual desktop-sim smoke binary

Spy-based unit tests cannot validate real LVGL layout, real scroll dynamics, real bidi, or real delete-order in a live screen swap. Add `examples/searchbar_demo.rs` (gated on the existing `desktop_sim` cfg) wiring SearchBar into a small SDL window with a fake async data source. Required pre-merge for any SearchBar change; documented in `DSL_REFERENCE.md`.

## 11. Implementation Order

Each step is independently buildable, testable, and committable:

1. **Extended spy + new C bindings** (`bindings.conf`, desktop-sim stubs, enriched event registry, user_data store, timer registry, scroll injection, new `LvCall` variants, `c_bindings::tests` updates).
2. `searchbar/token.rs` + `searchbar/row.rs` — pure data types.
3. `searchbar/highlight.rs` — `find_match` + recolor markup builder with full-text escape and capacity preallocation.
4. `searchbar/state.rs` — `State` enum + pure `transition()`.
5. `searchbar/action.rs` — `Action` enum + bounded `VecDeque` drain.
6. `searchbar/state_cell.rs` — `InnerState` + `try_borrow_mut`-or-queue helpers.
7. `searchbar/debounce.rs` — `DebounceTimer` (with the token-timing regression covered).
8. `searchbar/pagination.rs` — scroll-end handler + auto-fill + page_index.
9. `searchbar/selection.rs` — indicator draw + click handling.
10. `searchbar/inner.rs` — `Inner` aggregate + trampolines.
11. `searchbar/mod.rs` — public facade + composite integration tests.
12. `mod.rs` + `prelude.rs` exports + `DSL_REFERENCE.md` and `DSL_PLAYGROUND.html` updates.
13. `examples/searchbar_demo.rs` desktop-sim smoke binary.

## 12. Risks & Mitigations

| # | Risk | Severity | Mitigation | Verifiable by |
|---|---|---|---|---|
| 1 | **`lv_obj_set_user_data` collision** with caller code that wants user_data on the SearchBar root or sub-widgets. | High | API doc: "do not set user_data on SearchBar root or sub-widgets". Reserve `bar.set_payload(usize) / payload()` as the escape hatch (stored on `Inner`, not on the LVGL object). Trampolines null-check user_data and early-return. | Test: overwriting user_data post-construction → trampoline early-returns without panic. |
| 2 | **Re-entrancy & mutable aliasing** — naive `&mut Inner` deref in trampolines is UB if a callback synchronously triggers another SearchBar event. | Critical | `Inner.state: RefCell<InnerState>` + `try_borrow_mut`-or-queue pattern. Borrows are dropped before any user callback fires. Bounded `VecDeque<Action>` (cap 16) FIFO drain. No raw `&mut` is ever materialized through user_data. | Test: `on_query_changed` callback that calls `set_loading` then `set_results` produces correct final state in spy log; queue overflow at 17 actions increments `queue_overflow`. |
| 3 | **Timer fires after SearchBar deletion** (screen unload race). | High | `LV_EVENT_DELETE` order: (a) `lv_timer_delete`, (b) NULL user_data, (c) drop `Box<Inner>`. Trampolines guard on null user_data. LVGL auto-deletes child object callbacks. | Test: live-timer set empty after delete; trampoline with NULL user_data is a no-op. |
| 4 | **`Box<Inner>` heap address must never move** — moving SearchBar must not invalidate the LVGL user_data pointer. | High | `SearchBar: !Clone, !Copy` (compile-time `assert_not_impl_any!`); `Inner` is `Box::leak`ed so its heap address is fixed. | Compile-time + doc. |
| 5 | **Token-timing bug**: bumping generation on every keystroke + debounce-dedupe could strand an in-flight valid response. | Critical | Generation bumps **only** when `on_query_changed` actually fires (after dedupe + min_query_len). Type → backspace-to-previous → dedupe path keeps the prior token live. Covered by an explicit regression test. | Regression test in `searchbar::debounce::tests`. |
| 6 | **`pending_load_more` never cleared** — caller never responds. | Medium | Three escape hatches: `cancel_pending_load_more()`, `set_error(token, true)`, `set_results` with new token (after query change). | Test: `cancel_pending_load_more` re-arms pagination on next scroll. |
| 7 | **Stale `set_results` from previous query** drops fresh data. | High | `RequestToken` round-trip; setters silently no-op on mismatch and increment `stale_drop_count`. Caller may inspect `bar.stale_drop_count()` in debug. | Test: `set_results(stale_token, …)` is no-op + `stale_drop_count` increments. |
| 8 | **`on_load_more` page identity** — auto-fill reuses the same query token; caller can't distinguish page N vs N+1 from token alone. | Medium | `on_load_more(token, page_index)`; `page_index` is monotonic per query. SearchBar increments before each emit. | Test: 3 consecutive load-mores deliver page_index 1, 2, 3. |
| 9 | **`set_results(empty, has_more=true)` ambiguous against state machine** — `NoResults` that paginates. | High | Normalize: `rows.is_empty()` ⇒ force `has_more=false`; transition to `NoResults`; no auto-fill. | Test in §10.9. |
| 10 | **`max_rows` cap conflicts with auto-fill divergence guard** — wedge at cap. | High | When `total_rows >= max_rows`, force `has_more=false` and stop pagination. `cancel_pending_load_more` is unnecessary at the cap. | Test in §10.6 (`max_rows` cap). |
| 11 | **Auto-fill loop divergence** — `append_results(_, [], has_more=true)` infinitely. | Medium | At most one extra `emit_load_more` per "zero-progress" reply; then `has_more` forced false; `pending_load_more` left true so caller resolves explicitly. | Test in §10.6 (divergence guard). |
| 12 | **Auto-fill UI looks idle** during outstanding internal pagination request. | Medium | `emit_load_more()` shows `footer_loading_slot` before firing `on_load_more`. | Test: spy `ObjRemoveFlag(HIDDEN)` on `footer_loading_slot` recorded immediately before the callback fires. |
| 13 | **First-render row-height = 0** — `lv_obj_get_scroll_bottom` returns 0 before first layout. | Medium | After first `set_results`, call `lv_obj_update_layout(result_container)` before computing `measured_row_height`; cache; fall back to `load_more_threshold_px`. | Test with simulated zero-height first render. |
| 14 | **"Near bottom always true" for short lists** — pagination misfires repeatedly. | Medium | `user_scroll_observed` flag gates pagination until `scroll_top > 0` is observed once. Auto-fill loop fires deterministically when content fits viewport instead. | Test in §10.6 (first-render guard). |
| 15 | **Recolor escaping** — literal `#` in prefix/suffix corrupts markup. | High | Escape `# → ##` over the **entire** cell text (prefix + match + suffix). `String::with_capacity` precomputed including escape count. | Test: cell text with `#` in prefix, suffix, AND inside the match. |
| 16 | **Slot widget ownership** — earlier draft's `set_*_widget` setters required reparenting that the crate doesn't support. | High | Replaced with slot accessors; caller creates children directly with the slot as parent. SearchBar never reparents anything. | Test in §10.9 (slot model). |
| 17 | **`on_query_cleared` vs `on_query_changed("")` semantics conflated.** | Medium | One callback (`on_query_cleared`) for any path to empty; never fire `on_query_changed("")`. Document explicitly. | Test in §10.9. |
| 18 | **`attach_keyboard` last-bind-wins** behavior is implicit. | Low | Documented; verified with spy assertion when a second `attach_keyboard` is called. | Test in §10.9 (attach_keyboard). |
| 19 | **Spy infrastructure too thin** to actually drive the test plan (no callback registry, no event emission, no timer firing, no per-object user_data). | Critical | Step 1 of implementation order is the spy extension; SearchBar tests depend on it. | Step-1 standalone tests prove the spy works before any SearchBar code lands. |
| 20 | **Heap allocation failure on embedded** (`alloc` panics on OOM). | Medium | Documented allocation surface: `Box<Inner>` once + per-`set_results` `Vec<Row>` grow + per-cell `String` storage + per-highlight markup `String::with_capacity`. `max_rows(n)` cap enforces an upper bound. No allocation on hot paths beyond the recolor markup builder. | Doc + test: row count cap enforced. |
| 21 | **Single-threaded LVGL invariant** — caller invokes setters from a different thread / ISR. | Medium | Documented requirement; `RefCell` panic in debug surfaces accidental misuse via the standard `BorrowMutError`. | Doc + RefCell behavior. |
| 22 | **Action queue holds owned row clones** — pathological re-entrancy can spike allocation. | Low | `queue_cap` (default 16) bounds it; queue overflow is a `debug_assert!` + counter. Caller code that enters re-entrancy heavily is documented as misuse. | Test: 17th queued action increments `queue_overflow`. |
| 23 | **Font pointer lifetime** — `Font` references in `ColumnConfig` must outlive the SearchBar. | Medium | API takes `&'static Font` (LVGL fonts are linker symbols). | Compile-time enforcement. |
| 24 | **Row String ownership** — `Row::cells(&[&str])` borrows but rows must outlive the call. | Medium | `Row` stores `String` (owned); `cells` clones; `cells_owned(Vec<String>)` skips. | Test: rows survive after the input slice is dropped. |
| 25 | **Non-ASCII whitespace trim** in `min_query_len` / "is empty". | Low | `str::trim()` (Unicode-aware) + `chars().count()`. | Test with NBSP-only query. |
| 26 | **`bindings.conf` regex omits a required symbol.** | Low | Step-1 spy tests reference each new symbol; build fails fast. | Step-1 tests. |
| 27 | **LVGL `lv_obj_create` returns NULL** under OOM. | Low | `panic!` per existing crate convention. | N/A. |
| 28 | **Spy can't validate real LVGL layout / scroll dynamics / delete order in a real screen swap.** | Medium | `examples/searchbar_demo.rs` desktop-sim smoke binary is the manual gate; required pre-merge. | Manual run + screenshot. |
| 29 | **Stale-render race**: token still valid (debounce hasn't fired yet for newer input) but textarea content has changed since the token was minted — response would render under stale UI input. | Critical | Two-condition acceptance gate in §4: setters require `token == current_token AND last_fired_query == current_textarea_text_normalized`. On mismatch: drop + `stale_drop_count++`. | Test: type "alice"→fire→T5; type "alicex"→T5 response arrives→dropped, no rows rendered, `stale_drop_count==1`; debounce fires for "alicex"→T6→T6 accepted. |
| 30 | **Action-queue drain algorithm internally inconsistent** in v2 spec (drain-while-borrowed vs drain-after-drop conflated). | Critical | **v5 Model A**: precise single-loop algorithm in §7. `apply_*` only mutates state + enqueues `Action::EmitCallback(...)`; user callbacks fire ONLY in drain step (d) with borrow dropped. `is_draining` flag handles re-entrant `dispatch` calls (which always succeed in `try_borrow_mut` because the active drain dropped its borrow). | Test: nested `set_loading→set_results→set_error` from inside `on_query_changed` produces FIFO-correct final state. |
| 39 | **`VecDeque` action queue allocates on first re-entrant use** in `no_std + alloc` hot path. | Low | `queue: VecDeque::with_capacity(QUEUE_CAP)` initialized in `Inner::new`. (v4's separate ring removed in v5 — Model A makes it unnecessary.) | Doc + test: zero allocations recorded during forced re-entrant drain (allocator counter). |
| 40 | **Drain loop ignored `re_entry_ring`** — re-entrant pushes via `try_borrow_mut().Err` got stranded; `unreachable!` was unsound. | Critical | **v5 supersedes v4**: ring removed entirely. Model A invariant ("apply never fires user callbacks") guarantees `try_borrow_mut() == Ok` for every legitimate re-entrant call. The `Err(_)` arm becomes dead code that `debug_assert!`s a Model-A violation. | Test: `apply_*` audit + Model-A regression test. |
| 31 | **Keyboard outlives SearchBar with dead textarea pointer** — `lv_keyboard_set_textarea` was never NULLed on delete. | High | `Inner.attached_keyboard: Option<*mut lv_obj_t>` populated by `attach_keyboard`; `LV_EVENT_DELETE` step 1 calls `lv_keyboard_set_textarea(kb, NULL)`. Public `detach_keyboard()` is the explicit inverse. | Test: attach kb to bar, delete bar, then `lv_keyboard_set_textarea` recorded with NULL prior to bar's `Box::from_raw`. |
| 32 | **Wrong recolor symbol name** — v2 used `lv_label_set_recolor_enabled` which doesn't exist in LVGL v9.2. | High | LVGL pinned to v9.2.x at top of spec; bindgen symbol is `lv_label_set_recolor`; spec §6 and §8 corrected. Step-1 spy tests reference each symbol so a missing/renamed one fails compilation. | Compile-time. |
| 33 | **Selection-clear callback ambiguity** — does `set_results` emit `on_select(_, false)` for prior selections? | Medium | Explicit rule in §5: clearing is **silent** (no `on_select(false)` for cleared rows). Documented on `on_select`. Caller mirrors via `selected_row_ids()` if needed. | Test: select row, call `set_results` with new rows, assert no `on_select` callback fired during clear. |
| 34 | **Pagination divergence-guard reset rules unspecified** — could double-fire across pages. | Medium | Reset rules in §7: extra-fire flag is per-`(generation, page_index)`; resets on generation bump or any append that adds ≥1 row. Within same page, repeated zero-progress appends just force `has_more=false` immediately, no re-fire. | Test: zero-progress append fires extra once; second zero-progress append fires zero extra. |
| 35 | **Variable-row-height threshold math** — `rows × measured_row_height` is wrong with mixed heights. | Medium | Pixel threshold (`load_more_threshold_px`) is the primary; rows-based add-on is a `max()` overlay only when `measured_row_height > 0`. Callers with variable rows set `load_more_threshold_rows(0)`. Documented. | Doc + test with two rows of differing heights. |
| 36 | **Slot ownership boundary violation** — caller hides/deletes/reparents slot containers and breaks the state machine. | Medium | Explicit slot ownership table in §2 reserving `Hidden` flag, delete, user_data, reparent, position/size. Debug builds may `debug_assert!` slot visibility against expected per-state table. Future revision can return narrower `SlotParent` handles. | Doc; debug-assert in `transition()` checks slot visibility post-flip. |
| 37 | **Spy thread-safety in parallel `cargo test`** — naive globals corrupt across tests. | High | All spy state in `thread_local!` storage. `reset_obj_pool` (already test-fixture-invoked) extended to wipe per-thread event/timer/userdata/scroll registries. | Step-1 spy test runs `#[test] fn parallel_isolation()` that spawns 4 threads each constructing isolated obj graphs. |
| 38 | **Spy timer semantics under-modeled** — paused timers, repeat_count > 1, infinite repeat (0). | Medium | `spy_fire_timer` semantics fully specified in §10 Step 0: paused=no-op; repeat_count==1=remove; >1=decrement; 0=infinite-keep. | Step-1 spy tests cover each branch. |
| 41 | **Canonicalization contradiction** — dedupe used raw `==`, condition 2 used trim+casefold; `last_fired_query` not reset on clear (re-typing same string after clear was silently dedupe-suppressed). | Critical | Single `canonical_query(text, case_sensitive)` function used everywhere (dedupe, condition 2, min_query_len, empty-pivot). `last_fired_canonical` reset to `""` on every clear / generation bump from query→empty. | Tests: clear-then-retype-same-string fires `on_query_changed`; trailing-whitespace edit does not invalidate in-flight token. |
| 42 | **Keyboard deleted before SearchBar** — stored kb pointer becomes dangling; SearchBar's DELETE handler calls `lv_keyboard_set_textarea(dead_kb_ptr, NULL)` → UAF. | High | Register `LV_EVENT_DELETE` observer on the keyboard at `attach_keyboard` time; observer NULLs `Inner.attached_keyboard`. SearchBar DELETE step 1 only calls `lv_keyboard_set_textarea` if `attached_keyboard.is_some()`. Observer also removes itself on `detach_keyboard` and on SearchBar delete. | Test: attach kb, delete kb, then delete SearchBar → no `KeyboardSetTextarea` call recorded for the dead pointer. |
| 43 | **LVGL timer `repeat_count` semantics modeled wrong** in v3 spy spec (claimed 0=infinite). LVGL v9.x: `-1=infinity, 0=stop+delete, n>0=remaining`. | High | §10 Step 0 spy timer table fully rewritten using signed `i32` mirroring upstream `lv_timer.h`. SearchBar one-shot uses `lv_timer_set_repeat_count(t, 1)`. | Step-1 spy tests cover all four branches (-1, 0, 1, n>1). |
| 44 | **`append_results` from non-`Results` states** undefined. | Medium | Explicit table in normalization rules §4: from Loading/NoResults with non-empty rows → promotes to Results; from Empty/Error → drop + `stale_drop_count++`. State diagram updated to show `NoResults → Results`. | Tests: each transition explicitly covered. |
| 45 | **§8 bindings delta misleading** — listed only "new" symbols; reader could miss already-present dependencies (`lv_textarea_get_text`, `lv_obj_update_layout`, `lv_obj_send_event`). | Low | New "Already-present dependencies" table in §8 lists every symbol the design depends on with status. Step-1 tests reference each so missing ones fail compilation. | Step-1 spy tests. |
| 46 | **Slot default layout/styling unspecified** — caller has no idea what flex/padding/size to expect from slot containers. | Low | §2 specifies: `LV_LAYOUT_FLEX` column, center-aligned, `LV_PCT(100) × LV_SIZE_CONTENT`, no padding/border/background, hidden by default. Caller may override. | Doc. |
| 47 | **Spy reset not panic-safe** — a test that panics mid-flight leaks per-thread spy state into the next test on the same worker. | High | `SpyFixture` RAII struct: `new()` resets at start, `Drop` resets at end. Mandatory in every test (`with_spy!` macro). | Test: panicking test followed by clean test on same worker — clean test still passes. |
| 48 | **Callback / re-entry model was mixed (Model A vs Model B unclear)** — apply_* sometimes invoked user callbacks directly, which both *required* the re-entry ring and made every `apply_*` audit fragile. | Critical | **v5 picks Model A**: `apply_*` ONLY mutates `InnerState` and pushes `Action::EmitCallback(...)`. User callbacks fire ONLY in drain step (d) after the borrow is dropped. Ring eliminated. `try_borrow_mut().Err(_)` ⇒ `debug_assert!` (Model-A violation) + safe early return + `queue_overflow_borrow += 1`. | Tests: (a) audit test — every `apply_*` body is statically free of user-callback invocation (lint or doc + manual); (b) regression — synthetic `apply_*` that violates Model A trips the debug_assert. |
| 49 | **`min_query_len` transition undefined** — what happens when the canonical query has length `1..min_query_len`? | High | §4 defines three buckets: EMPTY (`canonical.is_empty()`), TOO_SHORT (`0 < len < min_query_len`), VALID (`len ≥ min_query_len`). TOO_SHORT is treated **identically to EMPTY**: drop rows, clear selection, fire `on_query_cleared`, bump generation, reset `last_fired_canonical=""`. `on_query_changed` never fires for TOO_SHORT. | Tests: (a) typing "ab" with min_query_len=3 produces zero `on_query_changed`, one `on_query_cleared`; (b) erasing from "abc" to "ab" fires `on_query_cleared` and bumps generation; (c) responses with the now-stale token are dropped. |
| 50 | **`on_retry` not implementable** — error UI lives in caller's slot child, but spec implied SearchBar would route a button click into the callback with no plumbing. | Medium | §3: caller invokes `bar.emit_retry()` from their own button handler inside the error slot. SearchBar fires `on_retry(&Bar, last_failed_query)`. SearchBar owns no error-button widget. "Retry ownership" paragraph added after `Row` API. | Test: caller installs `on_retry`, calls `emit_retry()`, callback fires once with the failed query string. |
| 51 | **`set_error(token, true)` from `Empty` / `NoResults` is ambiguous** — there is no in-flight request to fail, and `set_error(false)` has nothing meaningful to restore to. | Medium | §4 forbids `set_error(token, true)` from Empty/NoResults: it's a no-op + `stale_drop_count++`. Legal from Loading/Results/Error. `pre_error_state: Option<State>` records the source for deterministic `set_error(false)` restoration. | Tests: (a) `set_error(true)` from Empty → state unchanged + counter increments; (b) Loading → Error → false restores to Loading; (c) Results → Error → false restores to Results. |
| 52 | **Post-deletion API calls UAF the `Inner`** — caller may invoke `bar.set_results(...)` after the screen and bar were deleted. | High | `Inner.alive: Cell<bool>` set to `false` in `LV_EVENT_DELETE` step 0 (before any other cleanup). Every public setter checks `if !alive.get() { return; }` before touching the RefCell. SearchBar handle stores `*const Inner`; setters re-read `alive` first. | Tests: (a) setter called after `LV_EVENT_DELETE` is recorded as a no-op; (b) trampoline guards against re-entry on a half-torn-down inner. |
| 53 | **Stale spec facts vs actual repo state** — §8 originally claimed `lv_obj_remove_event_cb_with_user_data` was already in `bindings.conf`; it is not (only `lv_obj_remove_event_cb` is bound). Drift like this leaks into Step-1. | Low | §8 split into two tables; the `_with_user_data` variant is moved to the **new-bindings** allowlist with explicit "NOT YET in bindings — must be added in Step 1" annotation. Every claim in §8 carries a ✓ or "must add" marker. Step-1 spy tests reference each symbol so any missing one fails compilation. | Step-1: compile fails if any `must add` symbol is forgotten in `bindings.conf`. |
