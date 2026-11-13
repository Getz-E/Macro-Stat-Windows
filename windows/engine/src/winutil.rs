// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! Small Windows helpers shared by the input backend: wide strings, the
//! injection signature, DPI awareness, integrity level and timer resolution.
//!
//! Everything that can be pure is kept outside `#[cfg(windows)]` so Linux CI
//! exercises it. The FFI functions return `Option`/`bool` and never panic:
//! the engine treats "could not measure" as an unknown, not as a crash.

use std::fmt;
use std::path::{Path, PathBuf};

/// `dwExtraInfo` marker written into every synthetic event.
///
/// The low-level hook compares against this value and ignores our own
/// injection, so a macro cannot re-trigger itself. The value spells "MSTA"
/// in little-endian ASCII and is not a documented Windows constant; collisions
/// with third-party software are possible but harmless (their events would be
/// ignored like ours).
pub const INJECTION_SIGNATURE: usize = 0x4154_534D;

/// Windows mandatory integrity level of a process/token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrityLevel {
    Untrusted,
    Low,
    Medium,
    High,
    System,
    Protected,
}

impl IntegrityLevel {
    /// Classify the last sub-authority of a mandatory label SID.
    ///
    /// Windows allocates fixed bases (0x0000, 0x1000 … 0x5000); values between
    /// the bases are rounded down, so a custom level still lands in a bucket.
    pub fn from_rid(rid: u32) -> Self {
        match rid & 0xF000 {
            0x0000 => IntegrityLevel::Untrusted,
            0x1000 => IntegrityLevel::Low,
            0x2000 => IntegrityLevel::Medium,
            0x3000 => IntegrityLevel::High,
            0x4000 => IntegrityLevel::System,
            _ => IntegrityLevel::Protected,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            IntegrityLevel::Untrusted => "untrusted",
            IntegrityLevel::Low => "low",
            IntegrityLevel::Medium => "medium",
            IntegrityLevel::High => "high",
            IntegrityLevel::System => "system",
            IntegrityLevel::Protected => "protected",
        }
    }

    /// True when a process at this level should be able to `SendInput` into
    /// a window at `target` (UIPI blocks lower -> higher).
    pub fn can_drive(self, target: IntegrityLevel) -> bool {
        self >= target
    }
}

impl fmt::Display for IntegrityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// NUL-terminated UTF-16 buffer for `*W` APIs.
///
/// Interior NULs are dropped (Windows would truncate at them anyway) and a
/// terminating NUL is always present.
pub fn wide(text: &str) -> Vec<u16> {
    let mut out: Vec<u16> = text.encode_utf16().take_while(|unit| *unit != 0).collect();
    out.push(0);
    out
}

/// Windows `FILE_ATTRIBUTE_REPARSE_POINT` (symlink, junction, mount point).
pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// True when `path` itself is a symlink or (on Windows) any reparse point.
///
/// `symlink_metadata` is used so a link is reported even when its target is
/// missing. Windows junctions and mount points are reparse points without the
/// legacy "symlink" file type, which is why the attribute is checked as well;
/// Rust's `is_symlink()` alone would let a junction through.
pub fn is_reparse_point(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata_is_reparse(&metadata),
        Err(_) => false,
    }
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// Reject a path that is (or has become) a reparse point.
pub fn reject_reparse(path: &Path) -> std::io::Result<()> {
    if is_reparse_point(path) {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "yeniden ayrıştırma noktası reddedildi",
        ))
    } else {
        Ok(())
    }
}

/// Why [`read_text_limited`] failed.
#[derive(Debug)]
pub enum ReadTextError {
    /// The file is larger than the caller's limit.
    TooLarge(u64),
    /// Open/read failed, or the bytes are not valid UTF-8.
    Io(std::io::Error),
}

impl fmt::Display for ReadTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadTextError::TooLarge(limit) => {
                write!(f, "dosya boyut sınırını aşıyor (sınır {limit} bayt)")
            }
            ReadTextError::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ReadTextError {}

impl From<std::io::Error> for ReadTextError {
    fn from(error: std::io::Error) -> Self {
        ReadTextError::Io(error)
    }
}

/// Read at most `max_bytes` of UTF-8 text from a single open file handle.
///
/// The bound is enforced both from the handle's metadata and again on the
/// bytes actually read (`take(max + 1)`), so a file that grows after the
/// metadata check is still refused. The error never contains file content.
pub fn read_text_limited(path: &Path, max_bytes: u64) -> Result<String, ReadTextError> {
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() > max_bytes {
        return Err(ReadTextError::TooLarge(max_bytes));
    }
    let mut text = String::new();
    file.take(max_bytes + 1).read_to_string(&mut text)?;
    if text.len() as u64 > max_bytes {
        return Err(ReadTextError::TooLarge(max_bytes));
    }
    Ok(text)
}

/// Numeric description of the last Win32 error (no locale-dependent text).
#[cfg(windows)]
pub fn last_error() -> String {
    use windows_sys::Win32::Foundation::GetLastError;
    format!("Windows error {}", unsafe { GetLastError() })
}

/// Turn on Per-Monitor-V2 DPI awareness for this process.
///
/// Must run before any window is created. Returns `false` when the API is
/// unavailable (older Windows) or the manifest already set a different mode.
#[cfg(windows)]
pub fn set_per_monitor_v2() -> bool {
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0 }
}

#[cfg(not(windows))]
pub fn set_per_monitor_v2() -> bool {
    false
}

/// Restrict DLL search to the application directory and System32.
///
/// Without this, `LoadLibrary`/implicit imports may search the current
/// working directory and directories on `PATH`. The GUI starts the engine
/// with `cwd` set to the install directory, but a user-writable `%PATH%`
/// entry or a relocated install could still plant a DLL that Windows loads
/// into this process — which runs at the user's integrity level and injects
/// input. `SetDefaultDllDirectories` removes CWD and `PATH` from the search
/// order for every subsequent load. Must be called at the very start of
/// `main`, before any dynamic load happens.
///
/// Returns `true` when the hardening call succeeded. Windows 7 without the
/// KB2533623 update lacks the API; MacroStat targets Windows 10 22H2+, so a
/// failure is treated as a logged anomaly rather than a fatal error.
#[cfg(windows)]
pub fn harden_dll_search() -> bool {
    use windows_sys::Win32::System::LibraryLoader::{
        SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_APPLICATION_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };
    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32)
            != 0
    }
}

#[cfg(not(windows))]
pub fn harden_dll_search() -> bool {
    false
}

/// Keep the system timer at `ms` resolution for the lifetime of the value.
///
/// `timeBeginPeriod(1)` is what makes `Sleep(1)`-driven action timing close to
/// the Linux engine's millisecond delays. Windows 10 2004+ may ignore it for
/// background processes, which is documented rather than worked around.
#[cfg(windows)]
pub struct TimerResolution {
    period: u32,
}

#[cfg(windows)]
impl TimerResolution {
    pub fn new(ms: u32) -> Self {
        use windows_sys::Win32::Media::timeBeginPeriod;
        let period = ms.max(1);
        unsafe {
            timeBeginPeriod(period);
        }
        Self { period }
    }
}

#[cfg(windows)]
impl Drop for TimerResolution {
    fn drop(&mut self) {
        use windows_sys::Win32::Media::timeEndPeriod;
        unsafe {
            timeEndPeriod(self.period);
        }
    }
}

/// Integrity level of an arbitrary process.
#[cfg(windows)]
pub fn process_integrity_level(pid: u32) -> Option<IntegrityLevel> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
        TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    unsafe {
        let process = Handle(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid));
        if process.0.is_null() {
            return None;
        }
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(process.0, TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let token = Handle(token);

        let mut needed: u32 = 0;
        GetTokenInformation(
            token.0,
            TokenIntegrityLevel,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
        if needed == 0 {
            return None;
        }
        let mut buffer = vec![0u8; needed as usize];
        if GetTokenInformation(
            token.0,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        ) == 0
        {
            return None;
        }

        // `Vec<u8>` hizalaması 1'dir; doğrudan referans oluşturmak UB olurdu.
        if buffer.len() < std::mem::size_of::<TOKEN_MANDATORY_LABEL>() {
            return None;
        }
        let label = std::ptr::read_unaligned(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
        let sid = label.Label.Sid;
        if sid.is_null() {
            return None;
        }
        let count = *GetSidSubAuthorityCount(sid);
        if count == 0 {
            return None;
        }
        let rid = *GetSidSubAuthority(sid, u32::from(count - 1));
        Some(IntegrityLevel::from_rid(rid))
    }
}

#[cfg(not(windows))]
pub fn process_integrity_level(_pid: u32) -> Option<IntegrityLevel> {
    None
}

/// Integrity level of the foreground window's process, if any.
#[cfg(windows)]
pub fn foreground_integrity_level() -> Option<IntegrityLevel> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        process_integrity_level(pid)
    }
}

#[cfg(not(windows))]
pub fn foreground_integrity_level() -> Option<IntegrityLevel> {
    None
}

/// Whether the current process token is elevated.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut needed = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut needed,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Per-user data directory (`%LOCALAPPDATA%\MacroStat`).
///
/// `%LOCALAPPDATA%` is per-user and not roamed by default, which matches the
/// runtime-file/log requirements. When the variable is missing (service
/// accounts, stripped environments) the temporary directory is used so the
/// engine still logs instead of failing to start.
pub fn data_dir() -> PathBuf {
    match std::env::var_os("LOCALAPPDATA") {
        Some(local) => PathBuf::from(local).join("MacroStat"),
        None => std::env::temp_dir().join("MacroStat"),
    }
}

/// String form (`S-1-5-21-…`) of the current process token's user SID.
///
/// Used for the named-pipe DACL, the single-instance mutex name and
/// `runtime.json`. Returns `None` when the token cannot be queried; callers
/// must then refuse to start the IPC server (fail closed).
#[cfg(windows)]
pub fn current_user_sid_string() -> Option<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE};
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut needed: u32 = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed);
        if needed == 0 {
            CloseHandle(token);
            return None;
        }
        let mut buffer = vec![0u8; needed as usize];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        if buffer.len() < std::mem::size_of::<TOKEN_USER>() {
            return None;
        }
        let user = std::ptr::read_unaligned(buffer.as_ptr() as *const TOKEN_USER);
        if user.User.Sid.is_null() {
            return None;
        }
        let mut string_ptr: *mut u16 = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut string_ptr) == 0 || string_ptr.is_null() {
            return None;
        }
        let mut length = 0usize;
        while *string_ptr.add(length) != 0 {
            length += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(string_ptr, length));
        LocalFree(string_ptr.cast());
        Some(sid)
    }
}

#[cfg(not(windows))]
pub fn current_user_sid_string() -> Option<String> {
    None
}

/// Creation time of an arbitrary process as Windows FILETIME (100 ns ticks).
///
/// `runtime.json` stores it so the GUI can distinguish the engine it started
/// from a recycled PID.
#[cfg(windows)]
pub fn process_creation_time(pid: u32) -> Option<u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut creation = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut user = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let ok = GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        Some((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
    }
}

#[cfg(not(windows))]
pub fn process_creation_time(_pid: u32) -> Option<u64> {
    None
}

/// Block until process `pid` exits (best effort).
///
/// Uses `OpenProcess(SYNCHRONIZE)`; a process that cannot be opened (already
/// gone, or access denied) is treated as exited. Called from a detached
/// watcher thread so a clean engine shutdown is never blocked by the parent
/// still running.
#[cfg(windows)]
pub fn wait_for_process_exit(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE;
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, INFINITE};

    unsafe {
        let process = OpenProcess(SYNCHRONIZE, 0, pid);
        if process.is_null() {
            return;
        }
        WaitForSingleObject(process, INFINITE);
        CloseHandle(process);
    }
}

#[cfg(not(windows))]
pub fn wait_for_process_exit(_pid: u32) {}

/// Why a single-instance mutex could not be acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleInstanceError {
    /// Another engine already owns the mutex.
    AlreadyRunning,
    /// `CreateMutexW` failed; carries the Win32 error code.
    CreateFailed(u32),
}

impl fmt::Display for SingleInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleInstanceError::AlreadyRunning => f.write_str("already running"),
            SingleInstanceError::CreateFailed(code) => write!(f, "CreateMutexW error {code}"),
        }
    }
}

/// Owned named mutex; releasing the value releases the mutex.
#[cfg(windows)]
pub struct SingleInstance {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl SingleInstance {
    /// Create or open `name` and report whether this process created it.
    pub fn acquire(name: &str) -> Result<Self, SingleInstanceError> {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        let name = wide(name);
        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
            if handle.is_null() {
                return Err(SingleInstanceError::CreateFailed(GetLastError()));
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return Err(SingleInstanceError::AlreadyRunning);
            }
            Ok(Self { handle })
        }
    }
}

#[cfg(windows)]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_rids_map_to_levels() {
        assert_eq!(IntegrityLevel::from_rid(0x0000), IntegrityLevel::Untrusted);
        assert_eq!(IntegrityLevel::from_rid(0x1000), IntegrityLevel::Low);
        assert_eq!(IntegrityLevel::from_rid(0x2000), IntegrityLevel::Medium);
        assert_eq!(IntegrityLevel::from_rid(0x2100), IntegrityLevel::Medium);
        assert_eq!(IntegrityLevel::from_rid(0x3000), IntegrityLevel::High);
        assert_eq!(IntegrityLevel::from_rid(0x4000), IntegrityLevel::System);
        assert_eq!(IntegrityLevel::from_rid(0x5000), IntegrityLevel::Protected);
    }

    #[test]
    fn uipi_direction_is_lower_cannot_drive_higher() {
        assert!(IntegrityLevel::High.can_drive(IntegrityLevel::Medium));
        assert!(IntegrityLevel::Medium.can_drive(IntegrityLevel::Medium));
        assert!(!IntegrityLevel::Medium.can_drive(IntegrityLevel::High));
    }

    #[test]
    fn wide_strings_are_nul_terminated_and_nul_free() {
        assert_eq!(wide("abc"), vec![0x61, 0x62, 0x63, 0]);
        assert_eq!(wide("a\0b"), vec![0x61, 0]);
        assert_eq!(
            wide("ş"),
            vec![0x015F, 0],
            "UTF-16 encoding must keep one code unit"
        );
        assert_eq!(wide("😀"), vec![0xD83D, 0xDE00, 0]);
        assert_eq!(wide(""), vec![0]);
    }

    #[test]
    fn injection_signature_spells_msta() {
        assert_eq!(
            (INJECTION_SIGNATURE as u32).to_le_bytes(),
            [b'M', b'S', b'T', b'A']
        );
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("macrostat-winutil-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn reparse_helpers_detect_links_but_allow_regular_files() {
        let dir = scratch_dir("reparse");
        let real = dir.join("real.json");
        std::fs::write(&real, b"{\"ok\":true}").expect("write");

        assert!(!is_reparse_point(&real));
        assert!(!is_reparse_point(&dir.join("missing.json")));
        assert!(reject_reparse(&real).is_ok());

        #[cfg(unix)]
        {
            let link = dir.join("link.json");
            std::os::unix::fs::symlink(&real, &link).expect("symlink");
            assert!(is_reparse_point(&link));
            let error = reject_reparse(&link).expect_err("symlink must be refused");
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_reader_enforces_the_byte_limit() {
        let dir = scratch_dir("bounded");
        let path = dir.join("document.json");
        std::fs::write(&path, b"12345678").expect("write");

        assert_eq!(read_text_limited(&path, 8).expect("fits"), "12345678");
        match read_text_limited(&path, 7) {
            Err(ReadTextError::TooLarge(limit)) => assert_eq!(limit, 7),
            other => panic!("unexpected result: {other:?}"),
        }
        assert!(matches!(
            read_text_limited(&dir.join("missing.json"), 8),
            Err(ReadTextError::Io(_))
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
