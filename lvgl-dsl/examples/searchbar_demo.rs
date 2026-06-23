//! End-to-end smoke demo for the SearchBar widget.
//!
//! Runs under desktop-sim mock (no real LVGL frame buffer). Its purpose
//! is to exercise every public API path together and assert the order of
//! callbacks. If this binary panics, an integration regression has crept
//! in.
//!
//! Run with:
//!     cargo run --example searchbar_demo

extern crate alloc;

use lvgl_dsl::searchbar::row::SearchRow;
use lvgl_dsl::searchbar::{SearchBar, SearchBarConfig};
use lvgl_dsl::test_support::{SpyFixture, set_next_scroll_bottom};
use std::sync::{Arc, Mutex};

fn main() {
    let _fx = SpyFixture::new();
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let mut sb = unsafe {
        SearchBar::build(
            core::ptr::null_mut(),
            SearchBarConfig {
                width: 400,
                height: 300,
                case_insensitive: true,
                min_query_len: 2,
                debounce_ms: 100,
            },
        )
    };

    {
        let log = log.clone();
        sb.on_query_changed(move |t, q| {
            log.lock().unwrap().push(format!("query t={} q={}", t.0, q));
        });
    }
    {
        let log = log.clone();
        sb.on_load_more(move |t, p| {
            log.lock()
                .unwrap()
                .push(format!("load_more t={} page={}", t.0, p));
        });
    }
    {
        let log = log.clone();
        sb.on_select(move |id, on| {
            log.lock()
                .unwrap()
                .push(format!("select id={} on={}", id, on));
        });
    }
    {
        let log = log.clone();
        sb.on_query_cleared(move || {
            log.lock().unwrap().push("cleared".into());
        });
    }

    // 1) Type. set_text() kicks the debounce; tick_debounce() simulates the
    //    timer firing (we cannot wait for real wall-clock time in the mock).
    sb.set_text("pizza");
    sb.tick_debounce();
    let t = sb.current_token();
    println!("token after typing: {:?}", t);

    // 2) Reply with results.
    let accepted = sb.set_results(
        t,
        vec![
            SearchRow::new(1, "Pizza Hut"),
            SearchRow::new(2, "Domino's Pizza"),
            SearchRow::new(3, "Pizza Express"),
        ],
    );
    assert!(accepted, "set_results rejected");

    // 3) Select a row.
    sb.select(1);
    assert!(sb.is_selected_id(1));

    // 4) Scroll to bottom → load_more callback should fire.
    set_next_scroll_bottom(5);
    sb.check_scroll_for_load_more();

    // Reply to the load-more.
    let t2 = sb.current_token();
    let _ = sb.append_results(t2, vec![SearchRow::new(4, "Sbarro Pizza")]);

    // 5) Clear the query.
    sb.clear_query();

    let log = log.lock().unwrap();
    println!("--- callback log ({}) ---", log.len());
    for l in log.iter() {
        println!("  {}", l);
    }

    assert!(
        log.iter().any(|l| l.starts_with("query")),
        "on_query_changed never fired"
    );
    assert!(
        log.iter().any(|l| l.starts_with("select id=1 on=true")),
        "on_select never fired for row 1"
    );
    assert!(
        log.iter().any(|l| l.starts_with("load_more")),
        "on_load_more never fired"
    );
    assert!(
        log.iter().any(|l| l == "cleared"),
        "on_query_cleared never fired"
    );

    println!("searchbar_demo OK");
}
