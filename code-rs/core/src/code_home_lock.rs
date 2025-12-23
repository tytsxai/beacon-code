use fs2::FileExt;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;

const LOCK_FILE_NAME: &str = "code-home.lock";

#[derive(Debug)]
pub struct CodeHomeLock {
    _file: std::fs::File,
}

pub fn try_acquire_code_home_lock(code_home: &Path) -> io::Result<Option<CodeHomeLock>> {
    std::fs::create_dir_all(code_home)?;
    let lock_path = code_home.join(LOCK_FILE_NAME);
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)?;

    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(CodeHomeLock { _file: file })),
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(err) => Err(err),
    }
}
