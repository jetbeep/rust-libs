use std::boxed::Box;
use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::cell::{Cell, RefCell};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// Registry of live tasks on the UI thread, used to cancel tasks belonging to
// a soft-killed app generation (mirrors executor_zephyr.rs).
thread_local! {
    static REGISTRY: RefCell<Vec<NonNull<Task>>> = RefCell::new(Vec::new());
}

fn registry_push(task_ptr: NonNull<Task>) {
    let _ = REGISTRY.try_with(|r| r.borrow_mut().push(task_ptr));
}

fn registry_remove(task_ptr: NonNull<Task>) {
    let _ = REGISTRY.try_with(|r| {
        let mut vec = r.borrow_mut();
        if let Some(pos) = vec.iter().position(|p| *p == task_ptr) {
            vec.swap_remove(pos);
        }
    });
}

/// Cancel every task that does not belong to the current app generation.
/// See executor_zephyr.rs for the full contract.
pub fn cancel_stale() {
    let current = crate::generation::current();
    let stale: Vec<NonNull<Task>> = REGISTRY
        .try_with(|r| {
            let mut vec = r.borrow_mut();
            let mut stale = Vec::new();
            let mut i = 0;
            while i < vec.len() {
                let task_ptr = vec[i];
                if unsafe { task_ptr.as_ref().generation != current } {
                    vec.swap_remove(i);
                    stale.push(task_ptr);
                } else {
                    i += 1;
                }
            }
            stale
        })
        .unwrap_or_default();

    for task_ptr in stale {
        unsafe { task_ptr.as_ref().completed.set(true) };
        Task::dec_ref(task_ptr);
    }
}

/// Non-blocking single-thread executor (same as Zephyr version, but using std).
pub fn run<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    let task = Box::new(Task::new(Box::pin(future)));
    let task_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(task)) };

    registry_push(task_ptr);

    unsafe {
        Task::poll(task_ptr);
    }
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    ref_count: Cell<usize>,
    completed: Cell<bool>,
    generation: u32,
}

impl Task {
    fn new(future: Pin<Box<dyn Future<Output = ()> + 'static>>) -> Self {
        Self {
            future,
            ref_count: Cell::new(1),
            completed: Cell::new(false),
            generation: crate::generation::current(),
        }
    }

    unsafe fn poll(task_ptr: NonNull<Task>) {
        let task = task_ptr.as_ptr();

        if unsafe { (*task).completed.get() } {
            return;
        }

        let waker = Task::make_waker(task_ptr);
        let mut cx = Context::from_waker(&waker);
        let poll = unsafe { (*task).future.as_mut().poll(&mut cx) };
        drop(waker);

        if poll == Poll::Ready(()) {
            unsafe { (*task).completed.set(true) };
            registry_remove(task_ptr);
            Task::dec_ref(task_ptr);
        }
    }

    fn make_waker(task_ptr: NonNull<Task>) -> Waker {
        Task::inc_ref(task_ptr);
        unsafe { Waker::from_raw(Task::raw_waker(task_ptr)) }
    }

    fn raw_waker(task_ptr: NonNull<Task>) -> RawWaker {
        RawWaker::new(task_ptr.as_ptr().cast::<()>(), &TASK_WAKER_VTABLE)
    }

    fn inc_ref(task_ptr: NonNull<Task>) {
        let task = unsafe { task_ptr.as_ref() };
        let count = task.ref_count.get();
        task.ref_count.set(count + 1);
    }

    fn dec_ref(task_ptr: NonNull<Task>) {
        let task = unsafe { task_ptr.as_ref() };
        let count = task.ref_count.get();
        debug_assert!(count > 0);
        let next = count - 1;
        task.ref_count.set(next);

        if next == 0 {
            unsafe {
                drop(Box::from_raw(task_ptr.as_ptr()));
            }
        }
    }
}

unsafe fn waker_clone(data: *const ()) -> RawWaker {
    let task_ptr = unsafe { NonNull::new_unchecked(data as *mut Task) };
    Task::inc_ref(task_ptr);
    Task::raw_waker(task_ptr)
}

unsafe fn waker_wake(data: *const ()) {
    let task_ptr = unsafe { NonNull::new_unchecked(data as *mut Task) };
    unsafe { Task::poll(task_ptr) };
    Task::dec_ref(task_ptr);
}

unsafe fn waker_wake_by_ref(data: *const ()) {
    let task_ptr = unsafe { NonNull::new_unchecked(data as *mut Task) };
    unsafe { Task::poll(task_ptr) };
}

unsafe fn waker_drop(data: *const ()) {
    let task_ptr = unsafe { NonNull::new_unchecked(data as *mut Task) };
    Task::dec_ref(task_ptr);
}

static TASK_WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);
