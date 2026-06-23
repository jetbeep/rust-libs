//! Desktop filesystem API backed by host files and mounted to a configurable root.

use crate::error::Error;
use crate::workq::{post_to_main, submit_bg};
use futures::channel::oneshot;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const EPERM: i32 = 1;
const ENOENT: i32 = 2;
const EIO: i32 = 5;
const EACCES: i32 = 13;
const EEXIST: i32 = 17;
const ENOTDIR: i32 = 20;
const EISDIR: i32 = 21;
const EINVAL: i32 = 22;
const EBUSY: i32 = 16;
const ENOTSUP: i32 = 95;

pub type FsOffset = usize;

struct MountConfig {
    mount_root: PathBuf,
    canonical_root: PathBuf,
}

static MOUNT_CONFIG: OnceLock<MountConfig> = OnceLock::new();

pub struct File {
    inner: Arc<Mutex<std::fs::File>>,
}

struct DirState {
    entries: Vec<DirEntry>,
    index: usize,
}

pub struct Dir {
    inner: Arc<Mutex<DirState>>,
}

#[derive(Clone, Debug)]
pub struct DirEntry {
    is_file: bool,
    size: usize,
    name: Vec<u8>,
}

impl DirEntry {
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_dir(&self) -> bool {
        !self.is_file
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn name_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.name).ok()
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name
    }
}

// Re-export OpenFlags compatible with the Zephyr API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags(u8);

impl OpenFlags {
    pub const Read: OpenFlags = OpenFlags(0x01);
    pub const Write: OpenFlags = OpenFlags(0x02);
    pub const Create: OpenFlags = OpenFlags(0x04);
    pub const Append: OpenFlags = OpenFlags(0x08);
    pub const Truncate: OpenFlags = OpenFlags(0x10);

    fn contains(self, flag: OpenFlags) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl std::ops::BitOr for OpenFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        OpenFlags(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for OpenFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

fn fs_error(code: i32, op: &str, message: impl Into<String>) -> Error {
    Error {
        code: -code.abs(),
        message: format!("fs::{}: {}", op, message.into()),
    }
}

fn map_io_error(op: &str, err: std::io::Error) -> Error {
    if let Some(raw) = err.raw_os_error() {
        return Error {
            code: -raw.saturating_abs(),
            message: format!("fs::{}: {}", op, err),
        };
    }

    let code = match err.kind() {
        std::io::ErrorKind::NotFound => ENOENT,
        std::io::ErrorKind::PermissionDenied => EACCES,
        std::io::ErrorKind::AlreadyExists => EEXIST,
        std::io::ErrorKind::InvalidInput => EINVAL,
        std::io::ErrorKind::InvalidData => EINVAL,
        std::io::ErrorKind::WouldBlock => EBUSY,
        std::io::ErrorKind::Unsupported => ENOTSUP,
        _ => EIO,
    };

    fs_error(code, op, err.to_string())
}

fn map_lock_error(op: &str) -> Error {
    fs_error(EBUSY, op, "internal lock poisoned")
}

fn parse_virtual_rel_path(path: &str) -> Result<PathBuf, Error> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(PathBuf::new());
    }

    let mut rel = PathBuf::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Prefix(_) => {
                return Err(fs_error(EINVAL, "path", format!("invalid path: {}", path)));
            }
            Component::RootDir => {
                // Treat leading '/' as virtual mount root.
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(fs_error(EPERM, "path", format!("path escapes mount root: {}", path)));
            }
            Component::Normal(seg) => rel.push(seg),
        }
    }

    Ok(rel)
}

fn resolve_mount_config(fs_root: Option<&str>) -> Result<MountConfig, Error> {
    let root = if let Some(arg) = fs_root {
        let provided = PathBuf::from(arg);
        if provided.is_absolute() {
            provided
        } else {
            let cwd = std::env::current_dir()
                .map_err(|e| map_io_error("mount", e))?;
            cwd.join(provided)
        }
    } else {
        let exe_path = std::env::current_exe()
            .map_err(|e| map_io_error("mount", e))?;
        let exe_dir = exe_path
            .parent()
            .ok_or_else(|| fs_error(EINVAL, "mount", "failed to resolve executable directory"))?;
        exe_dir.join("fs")
    };

    let metadata = fs::metadata(&root).map_err(|e| map_io_error("mount", e))?;
    if !metadata.is_dir() {
        return Err(fs_error(ENOTDIR, "mount", format!("not a directory: {}", root.display())));
    }

    let canonical_root = root.canonicalize().map_err(|e| map_io_error("mount", e))?;
    Ok(MountConfig {
        mount_root: root,
        canonical_root,
    })
}

fn mount_config() -> Result<&'static MountConfig, Error> {
    if let Some(cfg) = MOUNT_CONFIG.get() {
        return Ok(cfg);
    }

    let cfg = resolve_mount_config(None)?;
    let _ = MOUNT_CONFIG.set(cfg);
    MOUNT_CONFIG
        .get()
        .ok_or_else(|| fs_error(EIO, "mount", "failed to initialize mount config"))
}

pub fn configure_mount_root(fs_root: Option<&str>) -> Result<(), Error> {
    let cfg = resolve_mount_config(fs_root)?;
    match MOUNT_CONFIG.set(cfg) {
        Ok(()) => {
            let cfg = MOUNT_CONFIG.get().expect("mount config just set");
            log::info!("desktop fs mount root: {}", cfg.mount_root.display());
            Ok(())
        }
        Err(_) => {
            if let Some(existing) = MOUNT_CONFIG.get() {
                log::warn!(
                    "desktop fs mount root already configured: {}",
                    existing.mount_root.display()
                );
            }
            Ok(())
        }
    }
}

fn ensure_within_root(candidate: &Path, cfg: &MountConfig, op: &str) -> Result<(), Error> {
    let mut probe = Some(candidate);
    while let Some(path) = probe {
        if let Ok(canonical) = path.canonicalize() {
            if canonical.starts_with(&cfg.canonical_root) {
                return Ok(());
            }
            return Err(fs_error(
                EPERM,
                op,
                format!("path escapes mount root: {}", candidate.display()),
            ));
        }
        probe = path.parent();
    }

    Err(fs_error(
        ENOENT,
        op,
        format!("unable to resolve path: {}", candidate.display()),
    ))
}

fn resolve_virtual_path(path: &str, allow_root: bool, op: &str) -> Result<PathBuf, Error> {
    let cfg = mount_config()?;
    let rel = parse_virtual_rel_path(path)?;
    if rel.as_os_str().is_empty() && !allow_root {
        return Err(fs_error(EINVAL, op, "empty path is not allowed"));
    }

    let full = if rel.as_os_str().is_empty() {
        cfg.mount_root.clone()
    } else {
        cfg.mount_root.join(rel)
    };

    ensure_within_root(&full, cfg, op)?;
    Ok(full)
}

async fn run_bg<T: Send + 'static, F>(f: F) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, Error> + Send + 'static,
{
    let (sender, receiver) = oneshot::channel::<Result<T, Error>>();

    unsafe {
        submit_bg(Duration::from_millis(0), move |_| {
            let result = f();
            // Bounce back to the UI thread via the cross-thread inbox.
            // Using `submit` here would push onto the bg thread's local
            // workq (thread_local!) where the result would never be
            // delivered — see workq.rs / post_to_main.
            post_to_main(move || {
                sender.send(result).ok();
            });
        });
    }

    receiver
        .await
        .map_err(|_| fs_error(EIO, "workq", "failed to receive background result"))?
}

impl File {
    pub async fn open(path: &str, flags: OpenFlags) -> Result<File, Error> {
        let host_path = resolve_virtual_path(path, false, "open")?;

        run_bg(move || {
            let read = flags.contains(OpenFlags::Read);
            let write = flags.contains(OpenFlags::Write);
            let append = flags.contains(OpenFlags::Append);
            let truncate = flags.contains(OpenFlags::Truncate);

            if !read && !write && !append {
                return Err(fs_error(EINVAL, "open", "one of Read/Write/Append must be set"));
            }

            let mut opts = OpenOptions::new();
            opts.read(read);
            opts.write(write || append || truncate);
            opts.append(append);
            opts.create(flags.contains(OpenFlags::Create));
            opts.truncate(truncate);

            let file = opts.open(&host_path).map_err(|e| map_io_error("open", e))?;
            Ok(File {
                inner: Arc::new(Mutex::new(file)),
            })
        })
        .await
    }

    pub async fn close(self) -> Result<(), Error> {
        run_bg(move || {
            drop(self);
            Ok(())
        })
        .await
    }

    pub async fn read(&mut self, size: usize) -> Result<Vec<u8>, Error> {
        let inner = Arc::clone(&self.inner);
        run_bg(move || {
            let mut tmp = vec![0u8; size];
            let mut guard = inner.lock().map_err(|_| map_lock_error("read"))?;
            let bytes = guard.read(&mut tmp).map_err(|e| map_io_error("read", e))?;
            tmp.truncate(bytes);
            Ok(tmp)
        })
        .await
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<usize, Error> {
        let inner = Arc::clone(&self.inner);
        let payload = data.to_vec();
        run_bg(move || {
            let mut guard = inner.lock().map_err(|_| map_lock_error("write"))?;
            let bytes = guard.write(&payload).map_err(|e| map_io_error("write", e))?;
            Ok(bytes)
        })
        .await
    }

    pub async fn seek(&mut self, offset: FsOffset) -> Result<(), Error> {
        let inner = Arc::clone(&self.inner);
        run_bg(move || {
            let mut guard = inner.lock().map_err(|_| map_lock_error("seek"))?;
            guard
                .seek(SeekFrom::Start(offset as u64))
                .map_err(|e| map_io_error("seek", e))?;
            Ok(())
        })
        .await
    }

    pub async fn tell(&self) -> Result<FsOffset, Error> {
        let inner = Arc::clone(&self.inner);
        run_bg(move || {
            let mut guard = inner.lock().map_err(|_| map_lock_error("tell"))?;
            let pos = guard
                .stream_position()
                .map_err(|e| map_io_error("tell", e))?;
            Ok(pos as FsOffset)
        })
        .await
    }

    pub async fn truncate(&mut self, length: FsOffset) -> Result<(), Error> {
        let inner = Arc::clone(&self.inner);
        run_bg(move || {
            let guard = inner.lock().map_err(|_| map_lock_error("truncate"))?;
            guard
                .set_len(length as u64)
                .map_err(|e| map_io_error("truncate", e))?;
            Ok(())
        })
        .await
    }

    pub async fn sync(&self) -> Result<(), Error> {
        let inner = Arc::clone(&self.inner);
        run_bg(move || {
            let guard = inner.lock().map_err(|_| map_lock_error("sync"))?;
            guard.sync_all().map_err(|e| map_io_error("sync", e))?;
            Ok(())
        })
        .await
    }
}

impl Dir {
    pub async fn open(path: &str) -> Result<Dir, Error> {
        let host_path = resolve_virtual_path(path, true, "opendir")?;

        run_bg(move || {
            let mut entries = Vec::new();
            let iter = fs::read_dir(&host_path).map_err(|e| map_io_error("opendir", e))?;

            for entry in iter {
                let entry = entry.map_err(|e| map_io_error("readdir", e))?;
                let metadata = entry.metadata().map_err(|e| map_io_error("readdir", e))?;
                let name = entry
                    .file_name()
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec();

                entries.push(DirEntry {
                    is_file: metadata.is_file(),
                    size: metadata.len() as usize,
                    name,
                });
            }

            Ok(Dir {
                inner: Arc::new(Mutex::new(DirState { entries, index: 0 })),
            })
        })
        .await
    }

    pub async fn read(&mut self) -> Result<Option<DirEntry>, Error> {
        let inner = Arc::clone(&self.inner);
        run_bg(move || {
            let mut guard = inner.lock().map_err(|_| map_lock_error("readdir"))?;
            if guard.index >= guard.entries.len() {
                return Ok(None);
            }
            let entry = guard.entries[guard.index].clone();
            guard.index += 1;
            Ok(Some(entry))
        })
        .await
    }

    pub async fn close(self) -> Result<(), Error> {
        run_bg(move || {
            drop(self);
            Ok(())
        })
        .await
    }
}

pub async fn unlink(path: &str) -> Result<(), Error> {
    let host_path = resolve_virtual_path(path, false, "unlink")?;
    run_bg(move || {
        let metadata = fs::metadata(&host_path).map_err(|e| map_io_error("unlink", e))?;
        if metadata.is_dir() {
            return Err(fs_error(EISDIR, "unlink", "path is a directory"));
        }
        fs::remove_file(&host_path).map_err(|e| map_io_error("unlink", e))?;
        Ok(())
    })
    .await
}

pub async fn rename(from: &str, to: &str) -> Result<(), Error> {
    let from_path = resolve_virtual_path(from, false, "rename")?;
    let to_path = resolve_virtual_path(to, false, "rename")?;
    run_bg(move || {
        fs::rename(&from_path, &to_path).map_err(|e| map_io_error("rename", e))?;
        Ok(())
    })
    .await
}

pub async fn mkdir(path: &str) -> Result<(), Error> {
    let host_path = resolve_virtual_path(path, false, "mkdir")?;
    run_bg(move || {
        fs::create_dir(&host_path).map_err(|e| map_io_error("mkdir", e))?;
        Ok(())
    })
    .await
}
