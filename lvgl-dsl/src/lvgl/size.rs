use crate::c_bindings;

// LV_SIZE_CONTENT expands to LV_COORD_SET_SPEC(LV_COORD_MAX) = (1<<29)-1 | (1<<29) = 0x3FFF_FFFF.
// LV_EXPORT_CONST_INT is a no-op by default so bindgen cannot see this macro — define manually.
const LV_SIZE_CONTENT: i32 = 1_073_741_823; // 0x3FFF_FFFF

#[derive(Debug, Copy, Clone)]
pub enum Size {
    Px(i32),
    Pct(i32),
    Content,
}

impl Size {
    pub(super) fn to_lv(self) -> i32 {
        match self {
            Size::Px(n) => n,
            // SAFETY: lv_pct() is a pure arithmetic C function — no pointers involved.
            Size::Pct(n) => unsafe { c_bindings::lv_pct(n) },
            Size::Content => LV_SIZE_CONTENT,
        }
    }
}
