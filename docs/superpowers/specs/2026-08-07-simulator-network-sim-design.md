# Simulator network connectivity simulation

**Date:** 2026-08-07
**Scope:** `jetbeep-core` simulator only (`feature = "simulator"`).

## Goal

Let the locker simulator emulate different network conditions for outbound
server requests, on the same Settings modal "Timing" tab that already hosts the
physical-world door-open / cell-status latency controls. This is independent of
the physical latency: it wraps a different call (`server_request`), not the
lock/status bus calls.

## Where it applies

`server_request` is the only outbound server call. In the simulator build,
`bus::server_request_ex` → `server_request_impl` → `modem_request_json` hits the
real agent client. The network simulation wraps `server_request_impl` in the
`#[cfg(feature = "simulator")]` path only. The real-hardware / Zephyr build
(`#[cfg(not(feature = "simulator"))]`) is unchanged and does not compile any of
this code.

## Controls (Timing tab)

| Control | Values | Effect |
|---|---|---|
| **Network mode** (dropdown) | `Normal`, `Mobile`, `Slow/2G`, `Offline` | Base latency profile before each request. `Offline` fails every request immediately. |
| **Failure rate %** (stepper, step 10, 0–100) | 0–100 | Independent fault injection: randomly fails that fraction of requests in any non-offline mode. |
| **Failure kind** (dropdown) | `Timeout`, `Server error (5xx)`, `Bad request (4xx)` | Which error a failed request returns, so different error-handling paths can be exercised. |
| **Extra latency (ms)** (stepper, step 100, 0–10000) | 0–10000 | Fixed delay added on top of the mode latency. |

### Mode latency (approximate, with jitter)

| Mode | Base delay |
|---|---|
| Normal | 0 ms |
| Mobile | ~600 ms ± 400 |
| Slow/2G | ~2500 ms ± 1000 |
| Offline | n/a (instant failure) |

Total applied delay = mode base (± jitter) + extra latency.

## Data model (`simulator/state.rs`)

```rust
enum NetworkMode { Normal, Mobile, Slow, Offline }
enum FailureKind { Timeout, ServerError, BadRequest }
struct NetworkSim {
    mode: NetworkMode,
    failure_rate: u32,     // 0..=100
    failure_kind: FailureKind,
    extra_latency_ms: u32, // 0..=10000
}
```

- Default: `Normal`, `failure_rate = 0`, `Timeout`, `extra_latency_ms = 0`.
- Stored in its own thread-local (survives layout re-init, like `PHYSICAL_TIMING`).
- `get_network_sim()` / `set_network_sim(sim)` accessors.
- `network_sim_outcome() -> (u32 /*delay_ms*/, Option<Error>)`:
  - `Offline` → `(0, Some(connectivity error))`.
  - else compute base delay for mode with jitter + `extra_latency_ms`; roll
    `failure_rate`; on hit return the `Error` matching `failure_kind`.

### Error mapping

| FailureKind | Error |
|---|---|
| Timeout | code `-2`, "network timeout (simulated)" |
| ServerError | code `-1`, "server error 500 (simulated)" |
| BadRequest | code `-1`, "bad request 400 (simulated)" |
| Offline | code `-1`, "network offline (simulated)" |

### Randomness

No `rand` dependency is in scope. Use a tiny thread-local `u32` xorshift PRNG
seeded from a monotonic counter, used for both jitter and the failure roll.

## `bus.rs` changes

- In the `#[cfg(feature = "simulator")]` `server_request_impl` path (or a thin
  wrapper it calls), before performing the request:
  ```rust
  let (delay_ms, fail) = state::network_sim_outcome();
  if delay_ms > 0 { workq::delay(Duration::from_millis(delay_ms as u64)).await; }
  if let Some(err) = fail { return Err(err); }
  ```
- Add `get_network_sim()` / `set_network_sim(..)` passthroughs mirroring the
  existing `get_physical_sim_timing` / `set_physical_sim_timing`.
- Re-export `NetworkSim` / `NetworkMode` / `FailureKind` under
  `#[cfg(feature = "simulator")]`.

## UI changes (`simulator/ui.rs`)

Extend the existing Timing tab so the new widgets show/hide with it:

- Add a "Network simulation" sub-title label.
- Mode dropdown (reuse `lv_dropdown_*`).
- Failure-rate stepper (reuse `make_stepper_row` + `adjust_stepper`, clamp 0–100,
  step 10).
- Failure-kind dropdown.
- Extra-latency stepper (existing `PHYS_MS_STEP` / `PHYS_MS_MAX`).
- Store live selections in `ModalUi` cells; add all new widgets to the
  `timing_widgets` array in `set_tab_visibility`.
- `modal_save_cb` commits the network selections via `state::set_network_sim`
  alongside the existing physical-timing save.

## Testing

Unit tests in `simulator/state.rs`:
- `Offline` mode always returns a failure and 0 delay.
- `failure_rate = 0` never fails; `failure_rate = 100` always fails.
- `Normal` mode base delay is 0 (delay equals `extra_latency_ms`).
- Each `FailureKind` maps to the expected error code/message.

## Non-goals

- No per-endpoint configuration; applies to all `server_request` calls.
- No persistence across process restarts (matches `PhysicalTiming`).
- No effect on real-hardware builds.
