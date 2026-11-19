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
        FileRequest::ReadBytes { path } => match fs::read(&path) {
            Ok(contents) => FileResponse::BytesRead { path, contents },
            Err(error) => failed(path, &error),
        },
        FileRequest::Write { path, contents } => written(path, contents.as_bytes()),
        FileRequest::WriteBytes { path, contents } => written(path, &contents),
        FileRequest::Rename { from, to } => match fs::rename(&from, &to) {
            Ok(()) => FileResponse::Renamed { path: to },
            Err(error) => failed(from, &error),
        },
    }
}

/// Where `state.ron` lives: under `$XDG_STATE_HOME` on Linux, or the platform's per-user folder.
pub fn state_file() -> Option<PathBuf> {
    Some(state_folder()?.join("picotracker").join("state.ron"))
}

#[cfg(target_os = "windows")]
fn state_folder() -> Option<PathBuf> {
    absolute_from_env("LOCALAPPDATA")
}

#[cfg(target_os = "macos")]
fn state_folder() -> Option<PathBuf> {
    Some(absolute_from_env("HOME")?.join("Library/Application Support"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn state_folder() -> Option<PathBuf> {
    absolute_from_env("XDG_STATE_HOME")
        .or_else(|| Some(absolute_from_env("HOME")?.join(".local/state")))
}

/// A folder named by an environment variable. The XDG rules ignore relative paths.
fn absolute_from_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn written(path: PathBuf, contents: &[u8]) -> FileResponse {
    match replace(&path, contents) {
        Ok(()) => FileResponse::Written { path },
        Err(error) => failed(path, &error),
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
/// Creates the file's folder first, which `state.ron` needs on first launch.
fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        fs::create_dir_all(folder)?;
    }
    let mut temporary: OsString = path.as_os_str().to_owned();
    temporary.push(".tmp");
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path).inspect_err(|_| {
        // The rename's own error is the one worth reporting.
        let _ = fs::remove_file(&temporary);
    })
}
