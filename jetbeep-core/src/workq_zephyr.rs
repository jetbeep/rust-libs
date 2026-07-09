use alloc::boxed::Box;
use core::{ffi::c_void, time::Duration};
use futures::channel::oneshot;

pub type TaskId = u32;

pub const TASK_ID_INVALID: TaskId = 0xFFFFFFFF;

type CWorkCallback = unsafe extern "C" fn(task_id: TaskId, user_data: *mut c_void);
enum WorkQType {
    Rust = 0,
    Background = 1,
}

unsafe extern "C" {
    fn rust_workq_restart(
        workq_type: i32,
        callback: CWorkCallback,
        user_data: *mut c_void,
        timeout_microseconds: u64,
        out_task_id: *mut TaskId,
    );
    fn rust_workq_cancel(workq_type: i32, task_id: TaskId);
}

fn get_callback<F>(_closure: &F) -> CWorkCallback
where
    F: FnMut(TaskId),
{
    task_done::<F>
}

unsafe extern "C" fn task_done<F>(task_id: TaskId, user_data: *mut c_void)
where
    F: FnMut(TaskId),
{
    unsafe {
        let user_data = Box::from_raw(user_data as *mut F);
        let mut cb = *(user_data);
        cb(task_id);
    }
}
// https://www.reddit.com/r/rust/comments/1h01z1s/just_learning_rust_and_have_immediately_stepped/
pub fn fn_mut_from_fn_once<A, R>(fn_once: impl FnOnce(A) -> R) -> impl FnMut(A) -> R {
    let mut fn_once = Some(fn_once);
    move |arg| {
        let fn_once = fn_once.take().unwrap();
        fn_once(arg)
    }
}

fn restart_internal<F: FnOnce(TaskId)>(workq_type: WorkQType, task_id: TaskId, timeout: Duration, result_cb: F) -> TaskId {
    unsafe {
        let mut id = task_id;
        // Main-workq closures are tagged with the app generation at submission
        // time and silently dropped when stale, so completions belonging to a
        // soft-killed app never run. Executor tasks of the old generation are
        // cancelled before stale closures are dropped (see executor::cancel_stale),
        // which makes dropping closures that own oneshot senders safe: the wake
        // triggered by the sender drop hits an already-completed task.
        let gate = match workq_type {
            WorkQType::Rust => Some(crate::generation::current()),
            WorkQType::Background => None,
        };
        let gated_cb = move |task_id: TaskId| {
            if let Some(gen) = gate {
                if gen != crate::generation::current() {
                    return;
                }
            }
            result_cb(task_id);
        };
        let closure = fn_mut_from_fn_once(gated_cb);
        let callback = get_callback(&closure);
        let boxed_closure = Box::new(closure);
        rust_workq_restart(
            workq_type as i32,
            callback,
            Box::into_raw(boxed_closure) as *mut c_void,
            timeout.as_micros() as u64,
            &mut id,
        );
        id
    }
}

pub fn restart<F: FnOnce(TaskId)>(task_id: TaskId, timeout: Duration, result_cb: F) -> TaskId {
    restart_internal(WorkQType::Rust, task_id, timeout, result_cb)
}

#[allow(dead_code)]
pub fn submit<F: FnOnce(TaskId)>(timeout: Duration, result_cb: F) -> TaskId {
    restart(TASK_ID_INVALID, timeout, result_cb)
}

#[allow(dead_code)]
pub fn cancel(task_id: TaskId) {
    unsafe {
        rust_workq_cancel(WorkQType::Rust as i32, task_id);
    }
}

pub unsafe fn restart_bg<F: FnOnce(TaskId)>(task_id: TaskId, timeout: Duration, result_cb: F) -> TaskId {
    restart_internal(WorkQType::Background, task_id, timeout, result_cb)
}

pub unsafe fn submit_bg<F: FnOnce(TaskId)>(timeout: Duration, result_cb: F) -> TaskId {
    unsafe { restart_bg(TASK_ID_INVALID, timeout, result_cb) }
}

pub unsafe fn cancel_bg(task_id: TaskId) {
    unsafe {
        rust_workq_cancel(WorkQType::Background as i32, task_id);
    }
}

pub async fn delay(timeout: Duration) {
    let (sender, receiver) = oneshot::channel::<()>();
    submit(timeout, |_| {
        sender.send(()).ok();
    });
    receiver.await.ok();
}
