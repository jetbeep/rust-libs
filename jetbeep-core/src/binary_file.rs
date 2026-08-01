use alloc::format;
use alloc::vec::Vec;

use crate::error::Error;
use crate::fs::{self, File, OpenFlags};

const EIO: i32 = 5;

pub(crate) async fn read(path: &str) -> Result<Vec<u8>, Error> {
    fs::read_all(path).await
}

pub(crate) async fn write(path: &str, bytes: &[u8]) -> Result<(), Error> {
    let flags = OpenFlags::Write | OpenFlags::Create | OpenFlags::Truncate;
    let mut file = File::open(path, flags).await?;
    let mut written = 0usize;
    while written < bytes.len() {
        let count = match file.write(&bytes[written..]).await {
            Ok(count) => count,
            Err(error) => {
                let _ = file.close().await;
                return Err(error);
            }
        };
        if count == 0 {
            let _ = file.close().await;
            return Err(Error {
                code: -EIO,
                message: format!(
                    "binary_file::write: short write at {} of {} bytes",
                    written,
                    bytes.len()
                ),
            });
        }
        written += count;
    }
    if let Err(error) = file.sync().await {
        let _ = file.close().await;
        return Err(error);
    }
    let _ = file.close().await;
    Ok(())
}