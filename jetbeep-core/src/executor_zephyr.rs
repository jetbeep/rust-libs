use alloc::boxed::Box;
use alloc::vec::Vec;
use core::{
	cell::{Cell, UnsafeCell},
	future::Future,
	pin::Pin,
	ptr::NonNull,
	task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

/// Registry of live tasks, used to cancel tasks belonging to a soft-killed
/// app generation. All executor operations run on the single LVGL/workq
/// thread, so plain interior mutability is sufficient.
struct Registry(UnsafeCell<Vec<NonNull<Task>>>);

// SAFETY: the executor is single-threaded (main workq thread only).
unsafe impl Sync for Registry {}

static REGISTRY: Registry = Registry(UnsafeCell::new(Vec::new()));

fn registry_push(task_ptr: NonNull<Task>) {
	unsafe { (*REGISTRY.0.get()).push(task_ptr) };
}

fn registry_remove(task_ptr: NonNull<Task>) {
	let vec = unsafe { &mut *REGISTRY.0.get() };
	if let Some(pos) = vec.iter().position(|p| *p == task_ptr) {
		vec.swap_remove(pos);
	}
}

/// Cancel every task that does not belong to the current app generation.
///
/// Must be called right after `generation::bump()` when soft-killing an app,
/// before any stale workq closure is dropped: cancelled tasks ignore all
/// subsequent wakes (e.g. from dropped oneshot senders), so old-app code
/// never resumes.
pub fn cancel_stale() {
	let current = crate::generation::current();
	let vec = unsafe { &mut *REGISTRY.0.get() };
	let mut i = 0;
	while i < vec.len() {
		let task_ptr = vec[i];
		let stale = unsafe { task_ptr.as_ref().generation != current };
		if stale {
			unsafe { task_ptr.as_ref().completed.set(true) };
			vec.swap_remove(i);
			Task::dec_ref(task_ptr);
		} else {
			i += 1;
		}
	}
}

/// Non-blocking single-thread executor that relies on `wake()` to continue polling.
///
/// Safety assumption: `wake()` is never called while `poll()` is executing.
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
