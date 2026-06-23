use std::boxed::Box;
use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::cell::Cell;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// Non-blocking single-thread executor (same as Zephyr version, but using std).
pub fn run<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    let task = Box::new(Task::new(Box::pin(future)));
    let task_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(task)) };

    unsafe {
        Task::poll(task_ptr);
    }
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    ref_count: Cell<usize>,
    completed: Cell<bool>,
}

impl Task {
    fn new(future: Pin<Box<dyn Future<Output = ()> + 'static>>) -> Self {
        Self {
            future,
            ref_count: Cell::new(1),
            completed: Cell::new(false),
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
