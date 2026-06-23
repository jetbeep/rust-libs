use std::cell::RefCell;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::cmp::Ordering;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};
use futures::channel::oneshot;

pub type TaskId = u32;

pub const TASK_ID_INVALID: TaskId = 0xFFFFFFFF;

#[cfg(target_family = "unix")]
unsafe extern "C" {
    fn _exit(status: i32) -> !;
}

/// Terminate the process immediately, bypassing Rust/C atexit cleanup.
///
/// We use this for simulator shutdown to avoid TLS-destructor re-entrancy in
/// the work queue at process teardown.
pub fn terminate_now(status: i32) -> ! {
    #[cfg(target_family = "unix")]
    unsafe {
        _exit(status)
    }

    #[cfg(not(target_family = "unix"))]
    {
        std::process::exit(status)
    }
}

struct TaskEntry {
    due: Instant,
    task_id: TaskId,
    callback: Box<dyn FnOnce(TaskId)>,
}

impl PartialEq for TaskEntry {
    fn eq(&self, other: &Self) -> bool {
        self.due == other.due && self.task_id == other.task_id
    }
}

impl Eq for TaskEntry {}

// Min-heap: earliest due time has highest priority
impl Ord for TaskEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.due.cmp(&self.due)
            .then_with(|| other.task_id.cmp(&self.task_id))
    }
}

impl PartialOrd for TaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct WorkqInner {
    heap: BinaryHeap<TaskEntry>,
    next_id: u32,
    cancelled: HashSet<TaskId>,
}

impl WorkqInner {
    fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_id: 1,
            cancelled: HashSet::new(),
        }
    }

    fn alloc_id(&mut self) -> TaskId {
        loop {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
            if id != TASK_ID_INVALID {
                return id;
            }
        }
    }
}

// The main workq holds `!Send` closures (UI/LVGL callbacks frequently capture
// raw widget pointers and other thread-bound state via `dispatch_to`). It is
// therefore a `thread_local!` queue, only accessible to the thread that runs
// `run_loop()` — i.e. the UI thread.
//
// To deliver work onto the UI thread *from another thread* (notably from the
// `workq-bg` worker when bouncing async results back), use `post_to_main(...)`.
// That funnels through a process-global, thread-safe inbox that `run_loop()`
// drains every iteration. Calling `submit(...)` directly from a non-UI thread
// is a bug — it would push the entry onto the caller-thread's local queue,
// which nobody drains, and the callback would never run (this previously
// caused awaiting tasks to hang forever after the network response arrived).
thread_local! {
    static MAIN_WORKQ: RefCell<WorkqInner> = RefCell::new(WorkqInner::new());
}

pub fn restart<F: FnOnce(TaskId) + 'static>(
    task_id: TaskId,
    timeout: Duration,
    result_cb: F,
) -> TaskId {
    // `try_with` is critical during process/thread teardown: dropping pending
    // queue entries can run callback destructors that attempt to reschedule
    // work. At that point TLS may already be in destruction and `with(...)`
    // would panic with AccessError.
    let scheduled = MAIN_WORKQ.try_with(|wq| {
        let mut inner = wq.borrow_mut();

        if task_id != TASK_ID_INVALID {
            inner.cancelled.insert(task_id);
        }

        let new_id = inner.alloc_id();
        inner.heap.push(TaskEntry {
            due: Instant::now() + timeout,
            task_id: new_id,
            callback: Box::new(result_cb),
        });
        new_id
    });

    scheduled.unwrap_or(TASK_ID_INVALID)
}

pub fn submit<F: FnOnce(TaskId) + 'static>(timeout: Duration, result_cb: F) -> TaskId {
    restart(TASK_ID_INVALID, timeout, result_cb)
}

pub fn cancel(task_id: TaskId) {
    if task_id == TASK_ID_INVALID {
        return;
    }
    let _ = MAIN_WORKQ.try_with(|wq| {
        wq.borrow_mut().cancelled.insert(task_id);
    });
}

// ── Cross-thread "post to main" inbox ───────────────────────────────────────
//
// Process-global, thread-safe queue used to ferry work from non-UI threads
// (e.g. `workq-bg`) onto the UI thread. `run_loop()` drains this inbox on
// every iteration, executing each callback inline on the main thread.
//
// FIX: this exists because the main workq is `thread_local!`, so `submit()`
// from another thread cannot reach it. Without this inbox, async results
// produced on `workq-bg` (HTTP/file I/O completions) had no path back to wake
// the future awaiting them on the UI thread — `.await` hung forever even
// though the underlying I/O had finished.
struct MainInbox {
    queue: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
    condvar: Condvar,
}

static MAIN_INBOX: OnceLock<Arc<MainInbox>> = OnceLock::new();
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

fn main_inbox() -> &'static Arc<MainInbox> {
    MAIN_INBOX.get_or_init(|| {
        Arc::new(MainInbox {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
        })
    })
}

/// Schedule `cb` to run on the UI/main thread as soon as `run_loop()` next
/// drains its inbox. Safe to call from any thread.
///
/// This is the correct way to deliver async results from background workers
/// back to UI-thread state; it avoids the thread-local pitfall that calling
/// `submit()` from a non-UI thread would otherwise hit.
pub fn post_to_main<F: FnOnce() + Send + 'static>(cb: F) {
    let inbox = main_inbox();
    inbox.queue.lock().unwrap().push_back(Box::new(cb));
    inbox.condvar.notify_one();
}

/// Request a graceful process shutdown.
///
/// This is safe to call from any thread. The main workq loop exits from the
/// UI thread, avoiding abrupt `exit()` from foreign threads while thread-local
/// values are being torn down.
pub fn request_shutdown() {
    SHUTDOWN_REQUESTED.store(true, AtomicOrdering::SeqCst);

    if let Some(inbox) = MAIN_INBOX.get() {
        inbox.condvar.notify_all();
    }
}

/// Drain all pending inbox callbacks; runs each on the current (UI) thread.
fn drain_main_inbox() {
    loop {
        let cb_opt = {
            let mut q = main_inbox().queue.lock().unwrap();
            q.pop_front()
        };
        match cb_opt {
            Some(cb) => cb(),
            None => break,
        }
    }
}

// ── Background thread work queue ────────────────────────────────────────────
//
// Callbacks submitted here run on a dedicated OS thread ("workq-bg"), which
// allows blocking I/O (filesystem, network) without stalling the main workq.
// Results are typically forwarded back to the main workq via `submit()`.

/// Task entry for the background queue. The callback must be `Send` because
/// it will be executed on a different OS thread.
struct BgTaskEntry {
    due: Instant,
    task_id: TaskId,
    callback: Box<dyn FnOnce(TaskId) + Send>,
}

impl PartialEq for BgTaskEntry {
    fn eq(&self, other: &Self) -> bool {
        self.due == other.due && self.task_id == other.task_id
    }
}
impl Eq for BgTaskEntry {}

impl Ord for BgTaskEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.due.cmp(&self.due)
            .then_with(|| other.task_id.cmp(&self.task_id))
    }
}
impl PartialOrd for BgTaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct BgWorkqInner {
    heap: BinaryHeap<BgTaskEntry>,
    next_id: u32,
    cancelled: HashSet<TaskId>,
}

impl BgWorkqInner {
    fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_id: 1,
            cancelled: HashSet::new(),
        }
    }

    fn alloc_id(&mut self) -> TaskId {
        loop {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
            if id != TASK_ID_INVALID {
                return id;
            }
        }
    }
}

struct BgWorkq {
    inner: Mutex<BgWorkqInner>,
    condvar: Condvar,
}

static BG_WORKQ: OnceLock<Arc<BgWorkq>> = OnceLock::new();

/// Returns the singleton background workq, spawning the worker thread on
/// first call.
fn bg_workq() -> &'static Arc<BgWorkq> {
    BG_WORKQ.get_or_init(|| {
        let wq = Arc::new(BgWorkq {
            inner: Mutex::new(BgWorkqInner::new()),
            condvar: Condvar::new(),
        });
        let wq_clone = Arc::clone(&wq);
        std::thread::Builder::new()
            .name("workq-bg".into())
            .spawn(move || bg_run_loop(wq_clone))
            .expect("failed to spawn background workq thread");
        wq
    })
}

fn bg_run_loop(wq: Arc<BgWorkq>) {
    loop {
        let task: Option<BgTaskEntry> = {
            let mut guard = wq.inner.lock().unwrap();
            'wait: loop {
                // Drain cancelled entries from the top of the heap.
                loop {
                    let top_id = guard.heap.peek().map(|e| e.task_id);
                    match top_id {
                        Some(id) if guard.cancelled.contains(&id) => {
                            guard.heap.pop();
                            guard.cancelled.remove(&id);
                        }
                        _ => break,
                    }
                }

                let now = Instant::now();
                // Copy the due time so the borrow on `guard.heap` is released
                // before we potentially move `guard` into `wait_timeout`.
                let peek_due = guard.heap.peek().map(|e| e.due);
                match peek_due {
                    Some(due) if due <= now => {
                        break 'wait guard.heap.pop();
                    }
                    Some(due) => {
                        let wait_dur = due.saturating_duration_since(now);
                        let (new_guard, _) =
                            wq.condvar.wait_timeout(guard, wait_dur).unwrap();
                        guard = new_guard;
                    }
                    None => {
                        guard = wq.condvar.wait(guard).unwrap();
                    }
                }
            }
        };

        if let Some(entry) = task {
            // Execute callback with the lock released so the callback can
            // itself call submit_bg / cancel_bg without deadlocking.
            (entry.callback)(entry.task_id);
        }
    }
}

pub unsafe fn restart_bg<F: FnOnce(TaskId) + Send + 'static>(
    task_id: TaskId,
    timeout: Duration,
    result_cb: F,
) -> TaskId {
    let wq = bg_workq();
    let mut inner = wq.inner.lock().unwrap();

    if task_id != TASK_ID_INVALID {
        inner.cancelled.insert(task_id);
    }

    let new_id = inner.alloc_id();
    inner.heap.push(BgTaskEntry {
        due: Instant::now() + timeout,
        task_id: new_id,
        callback: Box::new(result_cb),
    });
    drop(inner); // release the lock before notifying
    wq.condvar.notify_one();
    new_id
}

pub unsafe fn submit_bg<F: FnOnce(TaskId) + Send + 'static>(
    timeout: Duration,
    result_cb: F,
) -> TaskId {
    restart_bg(TASK_ID_INVALID, timeout, result_cb)
}

pub unsafe fn cancel_bg(task_id: TaskId) {
    if task_id == TASK_ID_INVALID {
        return;
    }
    let wq = bg_workq();
    let mut inner = wq.inner.lock().unwrap();
    inner.cancelled.insert(task_id);
    drop(inner);
    wq.condvar.notify_one();
}

pub async fn delay(timeout: Duration) {
    let (sender, receiver) = oneshot::channel::<()>();
    submit(timeout, |_| {
        sender.send(()).ok();
    });
    receiver.await.ok();
}

/// Run the workq event loop on the current thread. Never returns.
///
/// This is the desktop equivalent of the Zephyr `rust_workq` thread:
/// one thread processes all tasks sequentially — app callbacks, async wakers,
/// and LVGL timer ticks.
///
/// On each iteration the loop:
///   1. Drains the cross-thread `MAIN_INBOX` (callbacks posted from
///      `workq-bg`, etc.), running each on this thread.
///   2. Runs any due entries from the thread-local heap.
///   3. Sleeps on the inbox's condvar (with a timeout matching the next
///      heap-due time) so cross-thread posts wake the loop immediately
///      instead of waiting for `thread::sleep` to expire.
pub fn run_loop() -> ! {
    let inbox = main_inbox();
    loop {
        if SHUTDOWN_REQUESTED.load(AtomicOrdering::SeqCst) {
            terminate_now(0);
        }

        // 1. Cross-thread inbox first — this is how async results from the
        //    background workq make it onto the UI thread.
        drain_main_inbox();

        // 2. Local heap: extract one ready task (borrow released before
        //    callback runs so the callback may itself submit/cancel tasks).
        let task = MAIN_WORKQ.with(|wq| {
            let mut inner = wq.borrow_mut();

            // Drain cancelled entries from the top of the heap.
            loop {
                let dominated_by_cancelled = inner
                    .heap
                    .peek()
                    .map(|e| e.task_id)
                    .is_some_and(|id| inner.cancelled.contains(&id));
                if dominated_by_cancelled {
                    let entry = inner.heap.pop().unwrap();
                    inner.cancelled.remove(&entry.task_id);
                } else {
                    break;
                }
            }

            if let Some(entry) = inner.heap.peek() {
                if entry.due <= Instant::now() {
                    inner.heap.pop()
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(entry) = task {
            (entry.callback)(entry.task_id);
            continue;
        }

        // 3. No work ready — wait until either a cross-thread post arrives
        //    or the next local heap entry becomes due (whichever is first).
        let sleep_dur = MAIN_WORKQ.with(|wq| {
            let inner = wq.borrow();
            inner
                .heap
                .peek()
                .map(|e| e.due.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::from_millis(100))
        });

        if sleep_dur > Duration::ZERO {
            // Wait on the inbox's condvar so `post_to_main` from another
            // thread wakes us immediately. We don't actually consume any
            // queue items here — `drain_main_inbox` at the top of the loop
            // does that on the next iteration.
            let guard = inbox.queue.lock().unwrap();
            if guard.is_empty() {
                let _ = inbox.condvar.wait_timeout(guard, sleep_dur).unwrap();
            }
        }
    }
}
