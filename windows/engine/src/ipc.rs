// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! Named-pipe control channel: protocol, server and a small client.
//!
//! Security model (see `REPORT.md` §7.2):
//!
//! * the pipe name embeds the current user SID and a random 128-bit token;
//! * the DACL grants generic-all to that SID only (`D:P(A;;GA;;;<SID>)`) and
//!   `PIPE_REJECT_REMOTE_CLIENTS` refuses SMB clients;
//! * messages are newline-delimited JSON with a hard length limit and an idle
//!   timeout, so a stuck client cannot pin a server thread forever;
//! * if the SID cannot be resolved or the DACL cannot be built, the server
//!   refuses to start — fail closed rather than open the channel to everyone.
//!
//! The protocol and line assembler are platform neutral and unit-tested on
//! Linux; only [`start`] and [`send_command`] call Windows APIs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Maximum length of one request line (bytes, excluding the newline).
pub const MAX_LINE_BYTES: usize = 4_096;
/// A connected client that sends nothing for this long is disconnected.
pub const CLIENT_IDLE_TIMEOUT_MS: u64 = 5_000;
/// Pipe input/output buffer size.
pub const PIPE_BUFFER_BYTES: u32 = 8_192;

/// Supported control commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Ping,
    Reload,
    Stop,
    Status,
}

impl Command {
    pub fn as_str(self) -> &'static str {
        match self {
            Command::Ping => "ping",
            Command::Reload => "reload",
            Command::Stop => "stop",
            Command::Status => "status",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ping" => Some(Command::Ping),
            "reload" => Some(Command::Reload),
            "stop" => Some(Command::Stop),
            "status" => Some(Command::Status),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CommandEnvelope {
    #[serde(alias = "command")]
    cmd: String,
}

/// Parse one request line. The returned error is safe to send back to the
/// client: it never echoes the input.
pub fn parse_command(line: &str) -> Result<Command, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("empty request".to_string());
    }
    let envelope: CommandEnvelope =
        serde_json::from_str(trimmed).map_err(|_| "request is not valid JSON".to_string())?;
    Command::parse(&envelope.cmd).ok_or_else(|| "unknown command".to_string())
}

/// Runtime status published by the trigger loop and read by `status`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusSnapshot {
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_path: Option<String>,
    pub macros: usize,
    pub triggers: usize,
    pub loops: usize,
    pub hook_dropped: u64,
    pub injected_attempts: u64,
    pub injected_sent: u64,
    pub injected_shortfalls: u64,
}

/// Shared control flags and status, used by the main loop, workers and IPC.
pub struct ControlState {
    stop: AtomicBool,
    reload: AtomicBool,
    status: Mutex<StatusSnapshot>,
    pub pid: u32,
    pub pipe_name: String,
    pub log_path: String,
    pub elevated: bool,
    pub sid: String,
    started: Instant,
}

impl ControlState {
    pub fn new(pid: u32, pipe_name: String, log_path: String, elevated: bool, sid: String) -> Self {
        Self {
            stop: AtomicBool::new(false),
            reload: AtomicBool::new(false),
            status: Mutex::new(StatusSnapshot::default()),
            pid,
            pipe_name,
            log_path,
            elevated,
            sid,
            started: Instant::now(),
        }
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn request_reload(&self) {
        self.reload.store(true, Ordering::SeqCst);
    }

    /// Consume a pending reload request.
    pub fn take_reload(&self) -> bool {
        self.reload.swap(false, Ordering::SeqCst)
    }

    pub fn set_status(&self, snapshot: StatusSnapshot) {
        *self
            .status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = snapshot;
    }

    pub fn status(&self) -> StatusSnapshot {
        self.status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn uptime_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Full `status` response line (newline included).
    pub fn status_line(&self) -> String {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct StatusReply<'a> {
            ok: bool,
            cmd: &'a str,
            pid: u32,
            uptime_ms: u64,
            pipe: &'a str,
            log: &'a str,
            elevated: bool,
            #[serde(flatten)]
            status: StatusSnapshot,
        }

        let reply = StatusReply {
            ok: true,
            cmd: "status",
            pid: self.pid,
            uptime_ms: self.uptime_ms(),
            pipe: &self.pipe_name,
            log: &self.log_path,
            elevated: self.elevated,
            status: self.status(),
        };
        let mut line = serde_json::to_string(&reply).unwrap_or_else(|_| {
            r#"{"ok":false,"error":"status serialization failed"}"#.to_string()
        });
        line.push('\n');
        line
    }
}

pub fn ok_line(command: Command) -> String {
    let mut line = serde_json::json!({"ok": true, "cmd": command.as_str()}).to_string();
    line.push('\n');
    line
}

pub fn error_line(message: &str) -> String {
    let mut line = serde_json::json!({"ok": false, "error": message}).to_string();
    line.push('\n');
    line
}

/// Build the response for one client line and whether the server should
/// finish the connection afterwards (only `stop` does).
pub fn reply_for(state: &ControlState, line: &str) -> (String, bool) {
    match parse_command(line) {
        Ok(Command::Ping) => (ok_line(Command::Ping), false),
        Ok(Command::Status) => (state.status_line(), false),
        Ok(Command::Reload) => {
            state.request_reload();
            (ok_line(Command::Reload), false)
        }
        Ok(Command::Stop) => {
            state.request_stop();
            (ok_line(Command::Stop), true)
        }
        Err(error) => (error_line(&error), false),
    }
}

/// Accumulates pipe bytes into newline-delimited request lines.
#[derive(Debug, Default)]
pub struct LineAssembler {
    buffer: Vec<u8>,
}

impl LineAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one read chunk and return every complete line it contained.
    ///
    /// A line longer than [`MAX_LINE_BYTES`] aborts the connection: the buffer
    /// is cleared and `Err` is returned, so the server can answer without
    /// retaining unbounded memory.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        for &byte in chunk {
            if byte == b'\n' {
                if self.buffer.last() == Some(&b'\r') {
                    self.buffer.pop();
                }
                lines.push(String::from_utf8_lossy(&self.buffer).into_owned());
                self.buffer.clear();
            } else {
                self.buffer.push(byte);
                if self.buffer.len() > MAX_LINE_BYTES {
                    self.buffer.clear();
                    return Err(format!("request longer than {MAX_LINE_BYTES} bytes"));
                }
            }
        }
        Ok(lines)
    }
}

/// Pipe name shared by server and client: `\\.\pipe\macrostat-<SID>-<token>`.
pub fn pipe_name(sid: &str, token: &str) -> String {
    format!(r"\\.\pipe\macrostat-{sid}-{token}")
}

/// True when `sid` is safe to interpolate into an SDDL string.
///
/// A SID string is ASCII letters/digits and `-` only. Rejecting everything
/// else keeps commas, semicolons and parentheses out of
/// `D:P(A;;GA;;;<sid>)`, where they would change the parsed ACL instead of
/// naming a trustee. Empty input is refused as well.
pub fn sid_is_safe(sid: &str) -> bool {
    !sid.is_empty() && sid.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// 128-bit random token as lowercase hex.
pub fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut out = String::with_capacity(32);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    UnsupportedPlatform,
    MissingSid,
    InvalidSid,
    SecurityDescriptor(u32),
    CreatePipe(u32),
    Thread(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::UnsupportedPlatform => f.write_str("named-pipe IPC requires Windows"),
            IpcError::MissingSid => {
                f.write_str("current user SID unavailable; refusing to open the pipe")
            }
            IpcError::InvalidSid => {
                f.write_str("current user SID has an unexpected format; refusing to open the pipe")
            }
            IpcError::SecurityDescriptor(code) => {
                write!(f, "security descriptor error {code}")
            }
            IpcError::CreatePipe(code) => write!(f, "CreateNamedPipeW error {code}"),
            IpcError::Thread(message) => write!(f, "IPC thread failed: {message}"),
        }
    }
}

impl std::error::Error for IpcError {}

/// Detached server thread handle. Dropping it leaves the thread running; the
/// OS tears the pipe down when the process exits, which is the intended
/// fail-fast shutdown path.
pub struct IpcServer {
    #[cfg(windows)]
    _thread: std::thread::JoinHandle<()>,
}

/// Owned pipe handle; only the IPC thread touches it after `start` returns.
#[cfg(windows)]
struct PipeHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for PipeHandle {}

#[cfg(windows)]
pub fn start(state: std::sync::Arc<ControlState>) -> Result<IpcServer, IpcError> {
    use windows_sys::Win32::Foundation::{GetLastError, LocalFree, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    };

    if !sid_is_safe(&state.sid) {
        return Err(IpcError::InvalidSid);
    }
    let sddl = format!("D:P(A;;GA;;;{})", state.sid);
    let name = crate::winutil::wide(&state.pipe_name);
    let sddl_wide = crate::winutil::wide(&sddl);

    let handle = unsafe {
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(IpcError::SecurityDescriptor(GetLastError()));
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(0),
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let handle = CreateNamedPipeW(
            name.as_ptr(),
            // İlk örnek bayrağı: ad başka bir süreç tarafından önceden
            // bağlanmışsa oluşturma başarısız olur. Böylece aynı adı taşıyan
            // sahte bir sunucuya istemci yönlendirilemez.
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            0,
            &attributes,
        );
        let error = GetLastError();
        LocalFree(descriptor);
        // Win32 başarısızlıkta NULL değil INVALID_HANDLE_VALUE döndürür.
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(IpcError::CreatePipe(error));
        }
        PipeHandle(handle)
    };

    let thread = std::thread::Builder::new()
        .name("macrostat-ipc".to_string())
        .spawn(move || server_loop(handle, state))
        .map_err(|error| IpcError::Thread(error.to_string()))?;
    Ok(IpcServer { _thread: thread })
}

#[cfg(not(windows))]
pub fn start(_state: std::sync::Arc<ControlState>) -> Result<IpcServer, IpcError> {
    Err(IpcError::UnsupportedPlatform)
}

#[cfg(windows)]
fn server_loop(handle: PipeHandle, state: std::sync::Arc<ControlState>) {
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_PIPE_CONNECTED};
    use windows_sys::Win32::System::Pipes::{ConnectNamedPipe, DisconnectNamedPipe};

    let PipeHandle(pipe) = handle;

    loop {
        let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) };
        if connected == 0 {
            let error = unsafe { GetLastError() };
            if error != ERROR_PIPE_CONNECTED {
                if state.stop_requested() {
                    return;
                }
                // Transient accept failure (client vanished between the
                // connect and the call); retry without spinning.
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        }
        serve_connection(pipe, &state);
        unsafe {
            DisconnectNamedPipe(pipe);
        }
        if state.stop_requested() {
            return;
        }
    }
}

#[cfg(windows)]
fn serve_connection(pipe: windows_sys::Win32::Foundation::HANDLE, state: &ControlState) {
    use std::time::Duration;
    use windows_sys::Win32::Storage::FileSystem::ReadFile;
    use windows_sys::Win32::System::Pipes::PeekNamedPipe;

    let mut assembler = LineAssembler::new();
    let mut last_activity = Instant::now();
    loop {
        let mut available = 0u32;
        let peek = unsafe {
            PeekNamedPipe(
                pipe,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        };
        if peek == 0 {
            return;
        }
        if available == 0 {
            if last_activity.elapsed().as_millis() >= u128::from(CLIENT_IDLE_TIMEOUT_MS) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }

        let mut buffer = [0u8; 512];
        let mut read = 0u32;
        let ok = unsafe {
            ReadFile(
                pipe,
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).unwrap_or(0),
                &mut read,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || read == 0 {
            return;
        }
        last_activity = Instant::now();

        let lines = match assembler.push(&buffer[..read as usize]) {
            Ok(lines) => lines,
            Err(error) => {
                write_all(pipe, error_line(&error).as_bytes());
                return;
            }
        };
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let (reply, stop_after) = reply_for(state, &line);
            if !write_all(pipe, reply.as_bytes()) {
                return;
            }
            if stop_after {
                return;
            }
        }
    }
}

#[cfg(windows)]
fn write_all(pipe: windows_sys::Win32::Foundation::HANDLE, data: &[u8]) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{FlushFileBuffers, WriteFile};

    let mut offset = 0usize;
    while offset < data.len() {
        let chunk = u32::try_from(data.len() - offset).unwrap_or(u32::MAX);
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                pipe,
                data[offset..].as_ptr(),
                chunk,
                &mut written,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || written == 0 {
            return false;
        }
        offset += written as usize;
    }
    unsafe { FlushFileBuffers(pipe) != 0 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    UnsupportedPlatform,
    Open(u32),
    Io(String),
    Timeout,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::UnsupportedPlatform => f.write_str("named-pipe IPC requires Windows"),
            ClientError::Open(code) => write!(f, "CreateFileW error {code}"),
            ClientError::Io(message) => f.write_str(message),
            ClientError::Timeout => f.write_str("timed out waiting for the engine"),
        }
    }
}

impl std::error::Error for ClientError {}

/// Connect to the engine pipe, send one command and return the response line.
///
/// Used by `--ipc <cmd>` for acceptance tests and by the GUI's diagnostics.
#[cfg(windows)]
pub fn send_command(pipe_name: &str, command: &str) -> Result<String, ClientError> {
    use std::time::Duration;
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, ReadFile, WriteFile, OPEN_EXISTING, SECURITY_IDENTIFICATION,
        SECURITY_SQOS_PRESENT,
    };
    use windows_sys::Win32::System::Pipes::{PeekNamedPipe, WaitNamedPipeW};

    Command::parse(command).ok_or_else(|| ClientError::Io("unknown command".to_string()))?;
    let request = format!("{{\"cmd\":\"{}\"}}\n", command.trim().to_ascii_lowercase());
    let name = crate::winutil::wide(pipe_name);

    // SAHTE SUNUCU KORUMASI: `SECURITY_IDENTIFICATION`, sunucunun istemcinin
    // kimliğine bürünmesini engeller. Varsayılan (SecurityImpersonation) ile
    // aynı adı taşıyan kötü niyetli bir sunucu, yükseltilmiş GUI/`--ipc`
    // istemcisini taklit edebilirdi.
    let mut pipe = INVALID_HANDLE_VALUE;
    let mut last_error = 0u32;
    for attempt in 0..5u32 {
        pipe = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                std::ptr::null_mut(),
            )
        };
        if pipe != INVALID_HANDLE_VALUE {
            break;
        }
        // Sunucu tek bağlantı işler; kısa süreli meşguliyet normaldir.
        last_error = unsafe { GetLastError() };
        if last_error != ERROR_PIPE_BUSY {
            break;
        }
        unsafe {
            WaitNamedPipeW(name.as_ptr(), 200);
        }
        std::thread::sleep(Duration::from_millis(20 * u64::from(attempt)));
    }
    // Win32 başarısızlıkta INVALID_HANDLE_VALUE döndürür; NULL denetimi
    // hatayı gizleyip geçersiz tutamacı kullanıma sokuyordu.
    if pipe == INVALID_HANDLE_VALUE || pipe.is_null() {
        return Err(ClientError::Open(last_error));
    }

    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let pipe = Handle(pipe);

    let mut offset = 0usize;
    while offset < request.len() {
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                pipe.0,
                request[offset..].as_ptr(),
                u32::try_from(request.len() - offset).unwrap_or(u32::MAX),
                &mut written,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || written == 0 {
            return Err(ClientError::Io("write failed".to_string()));
        }
        offset += written as usize;
    }
    unsafe {
        FlushFileBuffers(pipe.0);
    }

    let deadline = Instant::now() + Duration::from_millis(CLIENT_IDLE_TIMEOUT_MS);
    let mut buffer = Vec::new();
    loop {
        let mut available = 0u32;
        let peek = unsafe {
            PeekNamedPipe(
                pipe.0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        };
        if peek == 0 {
            return Err(ClientError::Io("engine closed the pipe".to_string()));
        }
        if available == 0 {
            if Instant::now() >= deadline {
                return Err(ClientError::Timeout);
            }
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        let mut chunk = [0u8; 512];
        let mut read = 0u32;
        let ok = unsafe {
            ReadFile(
                pipe.0,
                chunk.as_mut_ptr(),
                u32::try_from(chunk.len()).unwrap_or(0),
                &mut read,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || read == 0 {
            return Err(ClientError::Io("engine closed the pipe".to_string()));
        }
        buffer.extend_from_slice(&chunk[..read as usize]);
        if buffer.contains(&b'\n') {
            return Ok(String::from_utf8_lossy(&buffer).trim().to_string());
        }
        if buffer.len() > MAX_LINE_BYTES {
            return Err(ClientError::Io("response too long".to_string()));
        }
    }
}

#[cfg(not(windows))]
pub fn send_command(_pipe_name: &str, _command: &str) -> Result<String, ClientError> {
    Err(ClientError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ControlState {
        ControlState::new(
            42,
            pipe_name("S-1-5-21-1", "abc"),
            "engine.log".to_string(),
            false,
            "S-1-5-21-1".to_string(),
        )
    }

    #[test]
    fn parses_all_commands() {
        assert_eq!(parse_command(r#"{"cmd":"ping"}"#), Ok(Command::Ping));
        assert_eq!(
            parse_command(r#"{"command":"STATUS"}"#),
            Ok(Command::Status)
        );
        assert_eq!(parse_command(r#"{"cmd":"stop"}"#), Ok(Command::Stop));
        assert_eq!(parse_command(r#"{"cmd":"reload"}"#), Ok(Command::Reload));
        assert!(parse_command(r#"{"cmd":"format"}"#).is_err());
        assert!(parse_command("").is_err());
        assert!(parse_command("not json").is_err());
    }

    #[test]
    fn assembler_handles_partial_and_multiple_lines() {
        let mut assembler = LineAssembler::new();
        assert!(assembler.push(br#"{"cmd":"pi"#).unwrap().is_empty());
        let lines = assembler.push(b"ng\"}\n{\"cmd\":\"status\"}\r\n").unwrap();
        assert_eq!(
            lines,
            vec![
                r#"{"cmd":"ping"}"#.to_string(),
                r#"{"cmd":"status"}"#.to_string()
            ]
        );
    }

    #[test]
    fn assembler_rejects_overlong_lines() {
        let mut assembler = LineAssembler::new();
        let chunk = vec![b'x'; MAX_LINE_BYTES + 1];
        assert!(assembler.push(&chunk).is_err());
        // The buffer is reset, so a following valid line still works.
        assert_eq!(assembler.push(b"ok\n").unwrap(), vec!["ok".to_string()]);
    }

    #[test]
    fn control_state_serializes_status() {
        let state = state();
        state.set_status(StatusSnapshot {
            profile: "Profil".to_string(),
            profile_path: Some("C:\\p.json".to_string()),
            macros: 2,
            triggers: 3,
            loops: 1,
            hook_dropped: 4,
            injected_attempts: 5,
            injected_sent: 6,
            injected_shortfalls: 7,
        });
        let line = state.status_line();
        assert!(line.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["cmd"], "status");
        assert_eq!(value["pid"], 42);
        assert_eq!(value["profile"], "Profil");
        assert_eq!(value["triggers"], 3);
        assert_eq!(value["injectedShortfalls"], 7);
        assert_eq!(value["pipe"], pipe_name("S-1-5-21-1", "abc"));
    }

    #[test]
    fn replies_drive_control_flags() {
        let state = state();
        let (reply, stop) = reply_for(&state, r#"{"cmd":"ping"}"#);
        assert!(reply.contains("\"ok\":true"));
        assert!(!stop);

        let (_, stop) = reply_for(&state, r#"{"cmd":"stop"}"#);
        assert!(stop);
        assert!(state.stop_requested());

        assert!(!state.take_reload());
        let (_, stop) = reply_for(&state, r#"{"cmd":"reload"}"#);
        assert!(!stop);
        assert!(state.take_reload());
        assert!(!state.take_reload());

        let (reply, stop) = reply_for(&state, r#"{"cmd":"nope"}"#);
        assert!(reply.contains("\"ok\":false"));
        assert!(!stop);
    }

    #[test]
    fn eager_commands_are_recognized() {
        // Guards the `Command::parse` table against a typo in one arm.
        for name in ["ping", "reload", "stop", "status"] {
            assert_eq!(Command::parse(name).map(Command::as_str), Some(name));
        }
    }

    #[test]
    fn sddl_interpolation_rejects_unsafe_sids() {
        assert!(sid_is_safe("S-1-5-21-1004336348-1177238915-682003330-1001"));
        assert!(sid_is_safe("S-1-5-18"));
        assert!(!sid_is_safe(""));
        assert!(!sid_is_safe("S-1-5-21-1)"));
        assert!(!sid_is_safe("S-1-5-21-1(A;;GA;;;WD"));
        assert!(!sid_is_safe("S-1-5-21-1;"));
        assert!(!sid_is_safe("S-1-5-21-1,WD"));
        assert!(!sid_is_safe("S-1-5-21-1 "));
        assert!(!sid_is_safe("S-1-5-21-1\n"));
    }

    #[test]
    fn pipe_names_and_tokens_have_expected_shape() {
        let name = pipe_name("S-1-5-21-99", "0123abcd");
        assert!(name.starts_with(r"\\.\pipe\macrostat-"));
        assert!(name.ends_with("-S-1-5-21-99-0123abcd"));
        let token = random_token();
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_ne!(token, random_token());
    }
}
