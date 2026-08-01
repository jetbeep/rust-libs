use alloc::ffi::CString;
use core::ffi::CStr;
use core::time::Duration;
use futures::channel::oneshot;
use enumset::{EnumSet, EnumSetType};
use alloc::{vec::Vec, vec, string::String};
use crate::error::Error;

use crate::workq::{submit, submit_bg};
use crate::c_bindings::*;

pub type FsOffset = usize;

const READ_ALL_CHUNK: usize = 4096;

pub struct File {
    inner: fs_file_t,
}

pub struct Dir {
    inner: fs_dir_t,
}

#[derive(Copy, Clone)]
pub struct DirEntry {
    inner: fs_dirent,
}

impl DirEntry {
    pub fn entry_type(&self) -> fs_dir_entry_type {
        self.inner.type_
    }

    pub fn is_file(&self) -> bool {
        self.inner.type_ == fs_dir_entry_type_FS_DIR_ENTRY_FILE
    }

    pub fn is_dir(&self) -> bool {
        self.inner.type_ == fs_dir_entry_type_FS_DIR_ENTRY_DIR
    }

    pub fn size(&self) -> usize {
        self.inner.size
    }

    pub fn name_cstr(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.inner.name.as_ptr()) }
    }

    pub fn name_bytes(&self) -> &[u8] {
        self.name_cstr().to_bytes()
    }

    pub fn name_str(&self) -> Option<&str> {
        core::str::from_utf8(self.name_cstr().to_bytes()).ok()
    }
}

/// Removes a file asynchronously.
///
/// The operation is executed in a workqueue context and the result is delivered
/// back on the main workqueue. This keeps the caller non-blocking.
///
/// # Parameters
/// - `path`: UTF-8 path without interior NUL bytes.
///
/// # Errors
/// Returns a negative Zephyr error code on failure.
pub async fn unlink(path: &str) -> Result<(), Error> {
    let c_path = CString::new(path).unwrap();
    let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

    unsafe {
        submit_bg(Duration::from_millis(0), move |_| {
            let result: i32;

            result = fs_unlink(c_path.as_ptr());

            submit(Duration::from_millis(0), move |_| {
                if result == 0 {
                    sender.send(Ok(())).ok();
                } else {
                    sender.send(Err(Error { code: result, message: String::from("fs_unlink") })).ok();
                }
            });
        });
    }

    receiver.await.unwrap()
}

/// Renames a file or directory asynchronously.
///
/// The operation is executed in a workqueue context and the result is delivered
/// back on the main workqueue. This keeps the caller non-blocking.
///
/// # Parameters
/// - `from`: Source path without interior NUL bytes.
/// - `to`: Destination path without interior NUL bytes.
///
/// # Errors
/// Returns a negative Zephyr error code on failure.
pub async fn rename(from: &str, to: &str) -> Result<(), Error> {
    let c_from = CString::new(from).unwrap();
    let c_to = CString::new(to).unwrap();
    let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

    unsafe {
        submit_bg(Duration::from_millis(0), move |_| {
            let result: i32;

            result = fs_rename(c_from.as_ptr(), c_to.as_ptr());

            submit(Duration::from_millis(0), move |_| {
                if result == 0 {
                    sender.send(Ok(())).ok();
                } else {
                    sender.send(Err(Error { code: result, message: String::from("fs_rename") })).ok();
                }
            });
        });
    }

    receiver.await.unwrap()
}

/// Creates a directory asynchronously.
///
/// The operation is executed in a workqueue context and the result is delivered
/// back on the main workqueue. This keeps the caller non-blocking.
///
/// # Parameters
/// - `path`: UTF-8 path without interior NUL bytes.
///
/// # Errors
/// Returns a negative Zephyr error code on failure.
pub async fn mkdir(path: &str) -> Result<(), Error> {
    let c_path = CString::new(path).unwrap();
    let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

    unsafe {
        submit_bg(Duration::from_millis(0), move |_| {
            let error = fs_tools_mkdir(c_path.as_ptr(), true);
            let result = if (*error).code == 0 {
                Ok(())
            } else {
                Err(crate::error::from_jb_error(error))
            };

            submit(Duration::from_millis(0), move |_| {
                sender.send(result).ok();
            });
        });
    }

    receiver.await.unwrap()
}

/// Opens a directory asynchronously.
///
/// The call is executed in a workqueue context and the result is delivered
/// back on the main workqueue. This keeps the caller non-blocking.
///
/// # Parameters
/// - `path`: UTF-8 path without interior NUL bytes.
///
/// # Errors
/// Returns a negative Zephyr error code on failure.
pub async fn opendir(path: &str) -> Result<Dir, Error> {
    let c_path = CString::new(path).unwrap();
    let (sender, receiver) = oneshot::channel::<Result<Dir, Error>>();

    unsafe {
        submit_bg(Duration::from_millis(0), move |_| {
            let mut dir_fd: fs_dir_t = core::mem::zeroed();
            let result: i32;

            fs_dir_t_init_shim(&mut dir_fd);
            result = fs_opendir(&mut dir_fd, c_path.as_ptr());

            submit(Duration::from_millis(0), move |_| {
                if result == 0 {
                    sender.send(Ok(Dir { inner: dir_fd })).ok();
                } else {
                    sender.send(Err(Error { code: result, message: String::from("fs_opendir") })).ok();
                }
            });
        });
    }

    receiver.await.unwrap()
}

pub(crate) async fn read_all(path: &str) -> Result<Vec<u8>, Error> {
    let c_path = CString::new(path).unwrap();
    let (sender, receiver) = oneshot::channel::<Result<Vec<u8>, Error>>();

    unsafe {
        submit_bg(Duration::from_millis(0), move |_| {
            let mut file_fd: fs_file_t = core::mem::zeroed();
            fs_file_t_init_shim(&mut file_fd);

            let open_result = fs_open(&mut file_fd, c_path.as_ptr(), FS_O_READ as u8);
            let result = if open_result != 0 {
                Err(Error {
                    code: open_result,
                    message: String::from("fs_open"),
                })
            } else {
                let mut bytes = Vec::new();
                let mut buffer = vec![0u8; READ_ALL_CHUNK];
                let mut read_error = None;

                loop {
                    let bytes_read = fs_read(
                        &mut file_fd,
                        buffer.as_mut_ptr() as *mut core::ffi::c_void,
                        buffer.len(),
                    );
                    if bytes_read < 0 {
                        read_error = Some(Error {
                            code: bytes_read as i32,
                            message: String::from("fs_read"),
                        });
                        break;
                    }
                    if bytes_read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..bytes_read as usize]);
                }

                let close_result = fs_close(&mut file_fd);
                if let Some(error) = read_error {
                    Err(error)
                } else if close_result != 0 {
                    Err(Error {
                        code: close_result,
                        message: String::from("fs_close"),
                    })
                } else {
                    Ok(bytes)
                }
            };

            submit(Duration::from_millis(0), move |_| {
                sender.send(result).ok();
            });
        });
    }

    receiver.await.unwrap()
}

impl File {
    /// Opens a file asynchronously on the background workqueue.
    ///
    /// The call is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Parameters
    /// - `path`: UTF-8 path without interior NUL bytes.
    /// - `flags`: Bitset of `OpenFlags`:
    ///   - `Read`
    ///   - `Write`
    ///   - `Truncate`
    ///   - `Create`
    ///   - `Append`
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn open(path: &str, flags: EnumSet<OpenFlags>) -> Result<File, Error> {
        let c_path = CString::new(path).unwrap();
        let (sender, receiver) = oneshot::channel::<Result<File, Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd: fs_file_t = core::mem::zeroed();
                let result: i32;
                let c_flags = to_c_flags(flags);

                fs_file_t_init_shim(&mut file_fd);
                result = fs_open(&mut file_fd, c_path.as_ptr(), c_flags);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        sender.send(Ok(File { inner: file_fd })).ok();
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_open") })).ok();
                    }
                });
            });
        }
        receiver.await.unwrap()
    }

    /// Closes the file asynchronously on the background workqueue.
    ///
    /// The close is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn close(self) -> Result<(), Error> {
        let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let result: i32;

                result = fs_close(&mut file_fd);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        sender.send(Ok(())).ok();
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_close") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Reads up to `size` bytes from the file asynchronously.
    ///
    /// The read is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Parameters
    /// - `size`: Maximum number of bytes to read.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn read(&mut self, size: usize) -> Result<Vec<u8>, Error> {
        let (sender, receiver) = oneshot::channel::<Result<Vec<u8>, Error>>();
        let mut buffer = vec![0u8; size];

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let bytes_read: isize;

                bytes_read = fs_read(&mut file_fd, buffer.as_mut_ptr() as *mut core::ffi::c_void, buffer.len());                

                submit(Duration::from_millis(0), move |_| {
                    if bytes_read >= 0 {
                        buffer.truncate(bytes_read as usize);
                        sender.send(Ok(buffer)).ok();
                    } else {
                        sender.send(Err(Error { code: bytes_read as i32, message: String::from("fs_read") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Writes the provided data asynchronously.
    ///
    /// The write is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Parameters
    /// - `data`: Bytes to write. The data is copied internally.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn write(&mut self, data: &[u8]) -> Result<usize, Error> {
        let (sender, receiver) = oneshot::channel::<Result<usize, Error>>();
        let buffer = data.to_vec();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let bytes_written: isize;

                bytes_written = fs_write(
                    &mut file_fd,
                    buffer.as_ptr() as *const core::ffi::c_void,
                    buffer.len(),
                );

                submit(Duration::from_millis(0), move |_| {
                    if bytes_written >= 0 {
                        sender.send(Ok(bytes_written as usize)).ok();
                    } else {
                        sender.send(Err(Error { code: bytes_written as i32, message: String::from("fs_write") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Writes the provided buffer asynchronously without copying.
    ///
    /// The write is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Parameters
    /// - `buffer`: Owned bytes to write.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn write_owned(&mut self, buffer: Vec<u8>) -> Result<usize, Error> {
        let (sender, receiver) = oneshot::channel::<Result<usize, Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let bytes_written: isize;
            
                bytes_written = fs_write(
                    &mut file_fd,
                    buffer.as_ptr() as *const core::ffi::c_void,
                    buffer.len(),
                );

                submit(Duration::from_millis(0), move |_| {
                    if bytes_written >= 0 {
                        sender.send(Ok(bytes_written as usize)).ok();
                    } else {
                        sender.send(Err(Error { code: bytes_written as i32, message: String::from("fs_write") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Seeks to a new position in the file asynchronously.
    ///
    /// The seek is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Parameters
    /// - `offset`: Byte offset relative to `whence`.
    /// - `whence`: One of `FS_SEEK_SET`, `FS_SEEK_CUR`, or `FS_SEEK_END`.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn seek(&mut self, offset: FsOffset, whence: i32) -> Result<(), Error> {
        let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let result: i32;

                result = fs_seek(&mut file_fd, offset as off_t, whence);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        sender.send(Ok(())).ok();
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_seek") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Returns the current file position asynchronously.
    ///
    /// The tell is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn tell(&mut self) -> Result<FsOffset, Error> {
        let (sender, receiver) = oneshot::channel::<Result<FsOffset, Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let result: off_t;

                result = fs_tell(&mut file_fd);

                submit(Duration::from_millis(0), move |_| {
                    if result >= 0 {
                        sender.send(Ok(result as FsOffset)).ok();
                    } else {
                        sender.send(Err(Error { code: result as i32, message: String::from("fs_tell") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Truncates or extends the file asynchronously.
    ///
    /// The truncate is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Parameters
    /// - `length`: New file size.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn truncate(&mut self, length: FsOffset) -> Result<(), Error> {
        let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let result: i32;

                result = fs_truncate(&mut file_fd, length as off_t);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        sender.send(Ok(())).ok();
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_truncate") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Flushes cached write data asynchronously.
    ///
    /// The sync is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn sync(&mut self) -> Result<(), Error> {
        let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut file_fd = self.inner;
                let result: i32;

                result = fs_sync(&mut file_fd);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        sender.send(Ok(())).ok();
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_sync") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }
}

impl Dir {
    /// Reads the next directory entry asynchronously.
    ///
    /// The read is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// Returns `Ok(None)` when the end of directory is reached.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn read(&mut self) -> Result<Option<DirEntry>, Error> {
        let (sender, receiver) = oneshot::channel::<Result<Option<DirEntry>, Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut dir_fd = self.inner;
                let mut entry: fs_dirent = core::mem::zeroed();
                let result: i32;

                result = fs_readdir(&mut dir_fd, &mut entry);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        if entry.name[0] == 0 {
                            sender.send(Ok(None)).ok();
                        } else {
                            sender.send(Ok(Some(DirEntry { inner: entry }))).ok();
                        }
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_readdir") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }

    /// Closes the directory asynchronously.
    ///
    /// The close is executed in a workqueue context and the result is delivered
    /// back on the main workqueue. This keeps the caller non-blocking.
    ///
    /// # Errors
    /// Returns a negative Zephyr error code on failure.
    pub async fn close(self) -> Result<(), Error> {
        let (sender, receiver) = oneshot::channel::<Result<(), Error>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let mut dir_fd = self.inner;
                let result: i32;

                result = fs_closedir(&mut dir_fd);

                submit(Duration::from_millis(0), move |_| {
                    if result == 0 {
                        sender.send(Ok(())).ok();
                    } else {
                        sender.send(Err(Error { code: result, message: String::from("fs_closedir") })).ok();
                    }
                });
            });
        }

        receiver.await.unwrap()
    }
}

#[derive(EnumSetType)]
pub enum OpenFlags {
   Read,
   Write,
   Truncate,
   Create,
   Append,
}

fn to_c_flags(flags: EnumSet<OpenFlags>) -> u8 {
    let mut c_flags: u8 = 0;
    if flags.contains(OpenFlags::Read) {
        c_flags |= FS_O_READ as u8;
    }
    if flags.contains(OpenFlags::Write) {
        c_flags |= FS_O_WRITE as u8;
    }
    if flags.contains(OpenFlags::Truncate) {
        c_flags |= FS_O_TRUNC as u8;
    }
    if flags.contains(OpenFlags::Create) {
        c_flags |= FS_O_CREATE as u8;
    }
    if flags.contains(OpenFlags::Append) {
        c_flags |= FS_O_APPEND as u8;
    }
    c_flags
}

