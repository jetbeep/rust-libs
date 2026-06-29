//! Pure-Rust, `no_std` on-demand LVGL font cache.
//!
//! Each font is exposed as a permanent **proxy** `lv_font_t` (a stable heap
//! pointer, created once per font and cached forever). Widgets/styles hold the
//! proxy pointer; the actual glyph data is loaded lazily on first render via
//! `lv_binfont_create` and freed again under an LRU byte budget. Because the
//! proxy never moves or dies, eviction never dangles — the next render simply
//! reloads. No reference counting, single-threaded (LVGL thread) only.
//!
//! The crate FFI-references LVGL (`lv_binfont_*`, `lv_fs_*`); those symbols are
//! resolved by the final binary's LVGL on every target. It mirrors only the
//! **public** `lv_font_t` layout (`lv_font.h`), guarded by `size_of` asserts.

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ffi::{c_char, c_void};
use core::ptr::{null, null_mut};

// ---------------------------------------------------------------------------
// ABI mirror of the PUBLIC `struct _lv_font_t` (lvgl/src/font/lv_font.h).
// Only public fields; the proxy forwards everything else to the real font.
// ---------------------------------------------------------------------------

type GlyphDscCb = unsafe extern "C" fn(*const LvFont, *mut c_void, u32, u32) -> bool;
type GlyphBitmapCb = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_void;
type ReleaseGlyphCb = unsafe extern "C" fn(*const LvFont, *mut c_void);

#[repr(C)]
struct LvFont {
    get_glyph_dsc: Option<GlyphDscCb>,
    get_glyph_bitmap: Option<GlyphBitmapCb>,
    release_glyph: Option<ReleaseGlyphCb>,
    line_height: i32,
    base_line: i32,
    /// Packed bitfield byte: `subpx:2, kerning:1, static_bitmap:1`. We only
    /// ever write 0 (subpx none, kerning normal, non-static), and never read it.
    bits: u8,
    underline_position: i8,
    underline_thickness: i8,
    dsc: *const c_void,
    fallback: *const LvFont,
    user_data: *mut c_void,
}

// Guard the mirror against the real struct size (3 ptrs + 2*i32 + 3*i8 + 3 ptrs).
#[cfg(target_pointer_width = "64")]
const _: () = assert!(core::mem::size_of::<LvFont>() == 64);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<LvFont>() == 36);

impl LvFont {
    const fn zeroed() -> Self {
        LvFont {
            get_glyph_dsc: None,
            get_glyph_bitmap: None,
            release_glyph: None,
            line_height: 0,
            base_line: 0,
            bits: 0,
            underline_position: 0,
            underline_thickness: 0,
            dsc: null(),
            fallback: null(),
            user_data: null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// LVGL FFI
// ---------------------------------------------------------------------------

type LvFsRes = i32;
const LV_FS_RES_OK: LvFsRes = 0;
const LV_FS_MODE_RD: i32 = 0x02;
const LV_FS_SEEK_END: i32 = 2;

extern "C" {
    fn lv_binfont_create(path: *const c_char) -> *mut LvFont;
    fn lv_binfont_destroy(font: *mut LvFont);
    fn lv_fs_open(file: *mut c_void, path: *const c_char, mode: i32) -> LvFsRes;
    fn lv_fs_close(file: *mut c_void) -> LvFsRes;
    fn lv_fs_read(file: *mut c_void, buf: *mut c_void, btr: u32, br: *mut u32) -> LvFsRes;
    fn lv_fs_seek(file: *mut c_void, pos: u32, whence: i32) -> LvFsRes;
    fn lv_fs_tell(file: *mut c_void, pos: *mut u32) -> LvFsRes;
}

/// Over-sized, 8-byte-aligned backing buffer for `lv_fs_file_t` (real size is
/// ~24-32 bytes; 96 is a safe upper bound across configs).
#[repr(C, align(8))]
struct LvFsFile([u8; 96]);
impl LvFsFile {
    const fn new() -> Self {
        LvFsFile([0u8; 96])
    }
    fn as_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
}

/// Mirror of LVGL's on-disk binary-font `head` payload (lv_binfont_loader.c).
/// Read directly with `size_of`, exactly as the loader does (no packing).
#[repr(C)]
struct FontHeaderBin {
    version: u32,
    tables_count: u16,
    font_size: u16,
    ascent: u16,
    descent: i16,
    typo_ascent: u16,
    typo_descent: i16,
    typo_line_gap: u16,
    min_y: i16,
    max_y: i16,
    default_advance_width: u16,
    kerning_scale: u16,
    index_to_loc_format: u8,
    glyph_id_format: u8,
    advance_width_format: u8,
    bits_per_pixel: u8,
    xy_bits: u8,
    wh_bits: u8,
    advance_width_bits: u8,
    compression_id: u8,
    subpixels_mode: u8,
    padding: u8,
    underline_position: i16,
    underline_thickness: u16,
}

// ---------------------------------------------------------------------------
// Cache state (single-threaded; accessed only on the LVGL thread)
// ---------------------------------------------------------------------------

struct Entry {
    /// The proxy font handed to LVGL. Lives inside a `Box<Entry>`, so its
    /// address is stable for the program's lifetime.
    proxy: LvFont,
    name: String,
    path: CString,
    fallback: *const LvFont,
    /// Loaded font, the bound fallback, or null when evicted.
    real: *mut LvFont,
    is_fallback: bool,
    weight: usize,
    last_used: u64,
}

struct Cache {
    base: String,
    budget: usize,
    loaded: usize,
    clock: u64,
    entries: Vec<Box<Entry>>,
}

struct SyncCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncCell<T> {}

static CACHE: SyncCell<Option<Cache>> = SyncCell(UnsafeCell::new(None));

#[inline]
unsafe fn cache() -> &'static mut Cache {
    let slot = &mut *CACHE.0.get();
    if slot.is_none() {
        // Lazy default init so accessors work even if `init()` was never called
        // (e.g. in unit tests, or before explicit configuration). `init()` may
        // still be called later to override the base path / budget.
        *slot = Some(Cache {
            base: String::from("J:fonts/"),
            budget: 256 * 1024,
            loaded: 0,
            clock: 0,
            entries: Vec::new(),
        });
    }
    slot.as_mut().unwrap()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialise the cache. `base_dir` is the LVGL path prefix where fonts live
/// (e.g. `"J:fonts/"`); `budget` is the resident glyph-data byte budget.
pub fn init(base_dir: &str, budget: usize) {
    unsafe {
        let c = cache();
        c.base = String::from(base_dir);
        c.budget = budget;
        maybe_evict(null_mut());
    }
}

/// Set the resident byte budget; evicts immediately if over.
pub fn set_budget(bytes: usize) {
    unsafe {
        cache().budget = bytes;
        maybe_evict(null_mut());
    }
}

/// Bytes of glyph data currently resident.
pub fn loaded_bytes() -> usize {
    unsafe { cache().loaded }
}

/// Get a stable proxy `lv_font_t*` for `name` (file `<base><name>.bin`).
/// Creates the proxy on first call (reading the file header for metrics) and
/// returns the same pointer thereafter. `fallback` is a real `lv_font_t*`
/// used when the file can't be loaded. The returned pointer is suitable for
/// `lv_obj_set_style_text_font` / `Font::from_raw`.
pub fn get(name: &str, fallback: *const c_void) -> *const c_void {
    unsafe {
        let c = cache();
        for e in c.entries.iter() {
            if e.name == name {
                return (&e.proxy) as *const LvFont as *const c_void;
            }
        }

        // Build "<base><name>.bin".
        let mut path = c.base.clone();
        path.push_str(name);
        path.push_str(".bin");
        let cpath = CString::new(path).unwrap_or_else(|_| CString::new("").unwrap());

        let mut boxed = Box::new(Entry {
            proxy: LvFont::zeroed(),
            name: String::from(name),
            path: cpath,
            fallback: fallback as *const LvFont,
            real: null_mut(),
            is_fallback: false,
            weight: 0,
            last_used: 0,
        });

        // Metrics: from the file header, else from the fallback font.
        let (lh, bl, up, ut, weight) = read_header(boxed.path.as_ptr())
            .unwrap_or_else(|| fallback_metrics(fallback as *const LvFont));
        boxed.weight = weight;

        let ep: *mut Entry = &mut *boxed;
        boxed.proxy = LvFont {
            get_glyph_dsc: Some(cb_get_glyph_dsc),
            get_glyph_bitmap: Some(cb_get_glyph_bitmap),
            release_glyph: Some(cb_release_glyph),
            line_height: lh,
            base_line: bl,
            bits: 0,
            underline_position: up,
            underline_thickness: ut,
            dsc: null(),
            fallback: null(),
            user_data: ep as *mut c_void,
        };

        let proxy_ptr = (&boxed.proxy) as *const LvFont as *const c_void;
        c.entries.push(boxed);
        proxy_ptr
    }
}

// ---------------------------------------------------------------------------
// Proxy callbacks
// ---------------------------------------------------------------------------

unsafe extern "C" fn cb_get_glyph_dsc(
    font: *const LvFont,
    g_dsc: *mut c_void,
    letter: u32,
    letter_next: u32,
) -> bool {
    let e = &mut *((*font).user_data as *mut Entry);
    if !ensure_loaded(e) {
        return false;
    }
    e.last_used = next_clock();
    ((*e.real).get_glyph_dsc.unwrap())(e.real, g_dsc, letter, letter_next)
}

unsafe extern "C" fn cb_get_glyph_bitmap(g_dsc: *mut c_void, draw_buf: *mut c_void) -> *const c_void {
    // `resolved_font` is field 0 of lv_font_glyph_dsc_t, and LVGL set it to our
    // proxy before calling. Read it without mirroring the whole struct.
    let proxy = *(g_dsc as *const *const LvFont);
    let e = &mut *((*proxy).user_data as *mut Entry);
    if !ensure_loaded(e) {
        return null();
    }
    e.last_used = next_clock();
    ((*e.real).get_glyph_bitmap.unwrap())(g_dsc, draw_buf)
}

unsafe extern "C" fn cb_release_glyph(font: *const LvFont, g_dsc: *mut c_void) {
    let e = &mut *((*font).user_data as *mut Entry);
    if !e.real.is_null() {
        if let Some(release) = (*e.real).release_glyph {
            release(e.real, g_dsc);
        }
    }
}

// ---------------------------------------------------------------------------
// Load / evict
// ---------------------------------------------------------------------------

unsafe fn ensure_loaded(e: &mut Entry) -> bool {
    if !e.real.is_null() {
        return true;
    }

    let f = lv_binfont_create(e.path.as_ptr());
    if !f.is_null() {
        e.real = f;
        e.is_fallback = false;
        e.proxy.dsc = (*f).dsc;
        cache().loaded += e.weight;
        log::debug!("fonts-cache: loaded {} ({} B, resident {} B)", e.name, e.weight, cache().loaded);
        maybe_evict(e as *mut Entry);
        return true;
    }

    if !e.fallback.is_null() {
        e.real = e.fallback as *mut LvFont;
        e.is_fallback = true;
        e.proxy.dsc = (*e.fallback).dsc;
        log::warn!("fonts-cache: load failed for {}, using fallback font", e.name);
        return true;
    }

    false
}

unsafe fn maybe_evict(keep: *mut Entry) {
    let c = cache();
    while c.loaded > c.budget {
        let mut victim: *mut Entry = null_mut();
        let mut oldest = u64::MAX;
        for boxed in c.entries.iter_mut() {
            let ep: *mut Entry = &mut **boxed;
            if ep == keep {
                continue;
            }
            let e = &mut *ep;
            if e.real.is_null() || e.is_fallback || e.weight == 0 {
                continue;
            }
            if e.last_used < oldest {
                oldest = e.last_used;
                victim = ep;
            }
        }
        if victim.is_null() {
            break;
        }
        let v = &mut *victim;
        lv_binfont_destroy(v.real);
        v.real = null_mut();
        v.proxy.dsc = null();
        c.loaded = c.loaded.saturating_sub(v.weight);
        log::debug!("fonts-cache: evicted {} (resident {} B)", v.name, c.loaded);
    }
}

#[inline]
unsafe fn next_clock() -> u64 {
    let c = cache();
    c.clock += 1;
    c.clock
}

// ---------------------------------------------------------------------------
// Header metrics
// ---------------------------------------------------------------------------

unsafe fn read_header(path: *const c_char) -> Option<(i32, i32, i8, i8, usize)> {
    let mut file = LvFsFile::new();
    if lv_fs_open(file.as_ptr(), path, LV_FS_MODE_RD) != LV_FS_RES_OK {
        return None;
    }

    let mut result = None;
    let mut len: u32 = 0;
    let mut label = [0u8; 4];
    let mut hdr = core::mem::MaybeUninit::<FontHeaderBin>::zeroed();

    let read_ok = lv_fs_read(file.as_ptr(), &mut len as *mut u32 as *mut c_void, 4, null_mut())
        == LV_FS_RES_OK
        && lv_fs_read(file.as_ptr(), label.as_mut_ptr() as *mut c_void, 4, null_mut())
            == LV_FS_RES_OK
        && &label == b"head"
        && lv_fs_read(
            file.as_ptr(),
            hdr.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<FontHeaderBin>() as u32,
            null_mut(),
        ) == LV_FS_RES_OK;

    if read_ok {
        let hdr = hdr.assume_init();
        let line_height = hdr.ascent as i32 - hdr.descent as i32;
        let base_line = -(hdr.descent as i32);
        let mut size: u32 = 0;
        if lv_fs_seek(file.as_ptr(), 0, LV_FS_SEEK_END) == LV_FS_RES_OK
            && lv_fs_tell(file.as_ptr(), &mut size) == LV_FS_RES_OK
        {
            result = Some((
                line_height,
                base_line,
                hdr.underline_position as i8,
                hdr.underline_thickness as i8,
                size as usize,
            ));
        }
    }

    lv_fs_close(file.as_ptr());
    result
}

unsafe fn fallback_metrics(fb: *const LvFont) -> (i32, i32, i8, i8, usize) {
    if fb.is_null() {
        return (0, 0, 0, 0, 0);
    }
    (
        (*fb).line_height,
        (*fb).base_line,
        (*fb).underline_position,
        (*fb).underline_thickness,
        0,
    )
}
