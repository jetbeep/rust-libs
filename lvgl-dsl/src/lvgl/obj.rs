use crate::c_bindings;

use super::widget::{LvObj, Widget};

/// Generic LVGL container object (`lv_obj`).
///
/// A plain `lv_obj_create`-allocated widget with no built-in content.  Use
/// it as a transparent layout container or as a styled panel.  All layout,
/// style, event, and state methods are inherited from the [`Widget`] trait.
pub struct Obj {
    obj: LvObj,
}

impl Widget for Obj {
    fn lv_obj(&self) -> &LvObj {
        &self.obj
    }
}

impl Obj {
    /// Creates a new container object as a child of `parent`.
    ///
    /// # Panics
    /// Panics if LVGL returns a null pointer (out-of-memory).
    pub fn new(parent: &impl Widget) -> Obj {
        // SAFETY: `parent` wraps a non-null, valid LVGL object.
        let obj = unsafe { c_bindings::lv_obj_create(parent.lv_obj().raw()) };
        if obj.is_null() {
            panic!("lv_obj_create returned null");
        }
        Obj {
            obj: LvObj::from_raw(obj),
        }
    }

    /// Wraps a raw `*mut lv_obj_t` pointer in an `Obj` without taking ownership semantics.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - `ptr` is non-null and points to a valid `lv_obj_t` of generic-object kind.
    /// - The wrapper does not outlive the underlying C-owned LVGL object.
    /// - No other wrapper exists for the same pointer that would also attempt to free it
    ///   (LVGL retains ownership of the underlying object in typical usage).
    /// - The pointer is only used on the LVGL thread (LVGL is single-threaded).
    pub unsafe fn from_raw(ptr: *mut crate::c_bindings::lv_obj_t) -> Self {
        Obj {
            obj: LvObj::from_raw(ptr),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::c_bindings::reset_obj_pool;
    use crate::lvgl::obj::Obj;
    use crate::lvgl::screen::Screen;

    fn parent() -> Screen {
        reset_obj_pool();
        Screen::active()
    }

    #[test]
    fn new_does_not_panic() {
        let p = parent();
        let _ = Obj::new(&p);
    }
}

#[cfg(test)]
mod pub_from_raw_tests {
    use super::*;

    #[test]
    fn pub_from_raw_round_trips_pointer() {
        let p = 0xDEAD_BEEF_usize as *mut crate::c_bindings::lv_obj_t;
        let o = unsafe { Obj::from_raw(p) };
        assert_eq!(o.lv_obj().raw(), p);
        std::mem::forget(o);
    }
}
