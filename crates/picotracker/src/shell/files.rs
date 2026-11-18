//! Carries out the App core's file requests on the real file system.

use picotracker_core::app::{DirectoryEntry, FileRequest, FileResponse};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Does what the app asked for, and says how it went.
pub fn carry_out(request: FileRequest) -> FileResponse {
    match request {
        FileRequest::List { directory } => match list(&directory) {
            Ok(entries) => FileResponse::Listed { directory, entries },
            Err(error) => failed(directory, &error),
        },
        FileRequest::Read { path } => match fs::read_to_string(&path) {
            Ok(contents) => FileResponse::Read { path, contents },
            Err(error) => failed(path, &error),
        },
        FileRequest::Write { path, contents } => match replace(&path, &contents) {
            Ok(()) => FileResponse::Written { path },
            Err(error) => failed(path, &error),
        },
    }
}

fn failed(path: PathBuf, error: &io::Error) -> FileResponse {
    FileResponse::Failed {
        path,
        reason: error.to_string(),
    }
}

fn list(directory: &Path) -> io::Result<Vec<DirectoryEntry>> {
    fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            Ok(DirectoryEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_directory: entry.path().is_dir(),
            })
        })
        .collect()
}

/// Writes beside the file and then swaps it in, so that a failed write leaves the old file whole.
fn replace(path: &Path, contents: &str) -> io::Result<()> {
    let mut temporary: OsString = path.as_os_str().to_owned();
    temporary.push(".tmp");
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path).inspect_err(|_| {
        // The rename's own error is the one worth reporting.
        let _ = fs::remove_file(&temporary);
    })
}
