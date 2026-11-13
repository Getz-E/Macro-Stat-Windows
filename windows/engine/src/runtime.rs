// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! Long-running trigger loop: hook events -> [`TriggerEngine`] -> action
//! workers, with IPC reload/stop/status, parent watching and clean shutdown.
//!
//! Thread model (mirrors the Linux engine):
//!
//! * the main thread owns [`TriggerEngine`] and the hook receiver; it never
//!   runs an action sequence itself, so the hook queue is drained promptly;
//! * every trigger command runs in a short-lived worker thread; one-shots do
//!   not release keys when they finish (Linux parity), while loops and
//!   `WHILE_HELD` workers release every injected key at exit;
//! * [`ActionGate`] serializes sequences so two workers cannot interleave
//!   `KEY_DOWN`/`KEY_UP` state;
//! * a missing parent (`--parent-pid`) ends the loop like Linux's
//!   `getppid() == 1`, and the panic key ends it after releasing keys.
//!
//! Only compiled on Windows: the hook and `SendInput` layer are required.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::actions::{
    execute_actions, release_all_keys, ActionContext, ActionGate, Cancel, Diagnostics, Injector,
    RandJitter,
};
use crate::hook::{self, HookError, InputEvent};
use crate::inject::{InjectionStats, SleepWaiter, WindowsInjector};
use crate::ipc::{ControlState, StatusSnapshot};
use crate::keymap::{keys_are_equivalent, Key};
use crate::log::{Level, Logger};
use crate::profile::{self, MacroAction, ProfileConfig, MAX_PROFILE_BYTES};
use crate::trigger::{CompiledMacro, TriggerCommand, TriggerEngine};
use crate::winutil;

/// Hook queue size; a few hundred milliseconds of typing. Overflow is
/// reported through [`hook::dropped_events`] and never blocks the hook.
const HOOK_QUEUE: usize = 4096;
/// Event-receive timeout, bounding how often lifecycle flags are checked.
const RECV_TIMEOUT_MS: u64 = 20;
/// How often the cached status snapshot is refreshed.
const STATUS_REFRESH_MS: u64 = 250;
/// Pause between loop iterations, matching Linux's loop/cancellable sleep.
const LOOP_PAUSE_MS: u64 = 5;
/// Granularity of the long-press watchdog.
const HOLD_POLL_MS: u64 = 10;

/// `runtime.json` document consumed by the GUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub pid: u32,
    pub creation_time: u64,
    pub exe_path: String,
    pub sid: String,
    pub pipe_name: String,
    pub log_path: String,
}

/// `runtime.json` en fazla bu kadar bayt olabilir (GUI ile aynı sınır).
pub const MAX_RUNTIME_BYTES: u64 = 64 * 1024;

/// Yeniden ayrıştırma noktası (symlink/junction) ise hata döndürür.
fn reject_reparse(path: &Path) -> std::io::Result<()> {
    crate::winutil::reject_reparse(path)
}

/// `runtime.json` dosyasına yalnızca geçerli kullanıcının erişebildiği
/// korumalı bir DACL uygular (en iyi çaba; başarısızlık ölümcül değildir).
fn protect_runtime_acl(path: &Path, sid: &str) -> bool {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        SetFileSecurityW, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR,
    };

    if !crate::ipc::sid_is_safe(sid) {
        return false;
    }
    let sddl = crate::winutil::wide(&format!("D:P(A;;FA;;;{sid})"));
    unsafe {
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return false;
        }
        let name = crate::winutil::wide(&path.display().to_string());
        let ok = SetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        ) != 0;
        LocalFree(descriptor);
        ok
    }
}

/// Atomically write `runtime.json` (unique temp file then rename).
///
/// The temp name is unique per process and created with `create_new`, so a
/// pre-existing file (including a symlink/junction plant) is never followed
/// or truncated. The final file gets a protected user-only DACL.
pub fn write_runtime_file(path: &Path, info: &RuntimeInfo) -> std::io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    reject_reparse(path)?;
    let data = serde_json::to_vec(info).map_err(std::io::Error::other)?;
    let mut tmp = None;
    let mut file = None;
    for attempt in 0..8u32 {
        let candidate = path.with_extension(format!("tmp.{}.{}", std::process::id(), attempt));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&candidate)
        {
            Ok(handle) => {
                tmp = Some(candidate);
                file = Some(handle);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (tmp, mut file) = match (tmp, file) {
        (Some(tmp), Some(file)) => (tmp, file),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "geçici runtime.json adı üretilemedi",
            ))
        }
    };
    let write_result = file.write_all(&data).and_then(|()| file.sync_all());
    drop(file);
    let result = write_result
        .and_then(|()| reject_reparse(path))
        .and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return result;
    }
    let _ = protect_runtime_acl(path, &info.sid);
    Ok(())
}

/// Read `runtime.json` with the same 64 KiB bound as the GUI.
pub fn read_runtime_file(path: &Path) -> std::io::Result<RuntimeInfo> {
    use std::io::Read;

    reject_reparse(path)?;
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_RUNTIME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "runtime.json çok büyük",
        ));
    }
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(MAX_RUNTIME_BYTES + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_RUNTIME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "runtime.json çok büyük",
        ));
    }
    serde_json::from_str(&text)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// Validate a `runtime.json` document before using it to reach the engine.
///
/// The engine's own `--ipc` client must not execute whatever path the file
/// names: a tampered file could point at a UNC share (outbound SMB/NTLM) or
/// at another process's pipe. Only the current user's live engine is
/// accepted.
pub fn validate_runtime_info(info: &RuntimeInfo) -> Result<(), String> {
    let current_sid = crate::winutil::current_user_sid_string()
        .ok_or_else(|| "geçerli kullanıcı SID'i okunamadı".to_string())?;
    if !info.sid.eq_ignore_ascii_case(&current_sid) {
        return Err("runtime.json başka bir kullanıcıya ait".to_string());
    }
    let expected_prefix = format!(r"\\.\pipe\macrostat-{}-", info.sid);
    if !info.pipe_name.starts_with(&expected_prefix) {
        return Err("runtime.json kanal adı beklenen biçimde değil".to_string());
    }
    if info.pid == 0 {
        return Err("runtime.json PID geçersiz".to_string());
    }
    let created = crate::winutil::process_creation_time(info.pid)
        .ok_or_else(|| "motor süreci çalışmıyor".to_string())?;
    if created != info.creation_time {
        return Err("runtime.json süreç kimliği eşleşmiyor".to_string());
    }
    Ok(())
}

/// Read and parse a profile with the same size limit as the Linux engine.
///
/// `None` means the built-in default profile; a missing or invalid document
/// is an error so the caller can keep the current profile on reload.
pub fn load_profile(path: Option<&Path>) -> Result<ProfileConfig, String> {
    use crate::winutil::ReadTextError;

    let Some(path) = path else {
        return Ok(profile::default_profile());
    };
    // Profil kullanıcı tarafından düzenlenebilir; bağlantı noktasına
    // yönlendirilmiş ya da sınırı aşan bir belge motoru beslemez.
    crate::winutil::reject_reparse(path)
        .map_err(|error| format!("Profil okunamadı ({}): {error}", path.display()))?;
    let contents = match crate::winutil::read_text_limited(path, MAX_PROFILE_BYTES) {
        Ok(contents) => contents,
        Err(ReadTextError::TooLarge(limit)) => {
            return Err(format!(
                "Profil çok büyük ({}): sınır {} bayt",
                path.display(),
                limit
            ))
        }
        Err(ReadTextError::Io(error)) => {
            return Err(format!("Profil okunamadı ({}): {error}", path.display()))
        }
    };
    profile::parse_profile(&contents)
        .ok_or_else(|| format!("Profil geçersiz JSON ya da şema dışı ({})", path.display()))
}

/// Injected-state shared by every worker.
#[derive(Clone)]
struct SharedInjector {
    inner: Arc<Mutex<SharedState>>,
}

struct SharedState {
    injector: WindowsInjector,
    pressed: HashSet<Key>,
}

impl SharedInjector {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedState {
                injector: WindowsInjector::new(),
                pressed: HashSet::new(),
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, SharedState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn stats(&self) -> InjectionStats {
        self.lock().injector.stats()
    }

    /// Release everything any worker injected (panic, stop, reload, exit).
    fn release_all(&self) {
        let mut guard = self.lock();
        let state = &mut *guard;
        release_all_keys(&mut state.injector, &mut state.pressed);
    }
}

/// [`Injector`] adapter for a worker. It mirrors key transitions into the
/// shared pressed set so any thread can release everything on shutdown.
struct WorkerInjector {
    shared: SharedInjector,
}

impl Injector for WorkerInjector {
    fn key(&mut self, key: Key, down: bool) {
        let mut state = self.shared.lock();
        state.injector.key(key, down);
        if down {
            state.pressed.insert(key);
        } else {
            state.pressed.remove(&key);
        }
    }

    fn mouse_move(&mut self, dx: i32, dy: i32) {
        self.shared.lock().injector.mouse_move(dx, dy);
    }

    fn mouse_wheel(&mut self, delta: i32) {
        self.shared.lock().injector.mouse_wheel(delta);
    }

    fn mouse_absolute(&mut self, x: i32, y: i32) {
        self.shared.lock().injector.mouse_absolute(x, y);
    }

    fn text_char(&mut self, ch: char) -> bool {
        self.shared.lock().injector.text_char(ch)
    }
}

/// Cancellation for a worker: process-wide shutdown OR its own stop flag.
struct WorkerCancel {
    stopping: Arc<AtomicBool>,
    local: Option<Arc<AtomicBool>>,
}

impl Cancel for WorkerCancel {
    fn is_cancelled(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
            || self
                .local
                .as_ref()
                .is_some_and(|flag| !flag.load(Ordering::SeqCst))
    }
}

/// Diagnostics that also land in the rotating engine log.
struct LogDiagnostics {
    logger: Arc<Logger>,
}

impl Diagnostics for LogDiagnostics {
    fn warn(&mut self, message: String) {
        self.logger.log(Level::Warn, &message);
    }
}

/// Worker exit notification so the main thread can drop stale loop entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerDone {
    Loop(usize),
    WhileHeld(usize),
}

/// Everything the main loop needs that is not owned by [`TriggerEngine`].
pub struct RuntimeOptions {
    pub profile: ProfileConfig,
    pub profile_path: Option<PathBuf>,
    pub state: Arc<ControlState>,
    pub logger: Arc<Logger>,
    pub parent_pid: Option<u32>,
}

/// Outcome of a clean engine run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeSummary {
    pub events: u64,
    pub triggers: usize,
    pub injected: InjectionStats,
    pub duration_ms: u64,
}

/// Why the long-running runtime could not start or run.
#[derive(Debug)]
pub enum RuntimeError {
    Hook(HookError),
    Io(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Hook(error) => write!(f, "{error}"),
            RuntimeError::Io(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Run until stop/panic/parent exit, then release all injected keys.
pub fn run(options: RuntimeOptions) -> Result<RuntimeSummary, RuntimeError> {
    let started = Instant::now();
    let RuntimeOptions {
        mut profile,
        profile_path,
        state,
        logger,
        parent_pid,
    } = options;

    let (handle, receiver) = hook::start(HOOK_QUEUE).map_err(RuntimeError::Hook)?;
    let (engine, compiled) = TriggerEngine::new(&profile);
    let mut engine = engine;
    // The hook latches the panic key-down before the bounded queue is tried,
    // so a saturated queue cannot swallow the emergency stop.
    hook::set_panic_key(engine.panic_key());
    for warning in &compiled.warnings {
        logger.log(
            Level::Warn,
            &format!(
                "Geçersiz tetikleyici '{}' (makro: '{}', token: '{}')",
                warning.trigger_key, warning.macro_name, warning.token
            ),
        );
    }
    logger.log(
        Level::Info,
        &format!(
            "Motor başladı: '{}' profili, {} etkin tetikleyici, panik tuşu {}",
            profile.name,
            compiled.macros.len(),
            engine.panic_key()
        ),
    );

    let shared = SharedInjector::new();
    let gate = ActionGate::new();
    let stopping = Arc::new(AtomicBool::new(false));
    let (done_tx, done_rx): (Sender<WorkerDone>, Receiver<WorkerDone>) = mpsc::channel();

    if let Some(pid) = parent_pid {
        spawn_parent_watcher(pid, Arc::clone(&state), Arc::clone(&logger));
    }

    update_status(&state, &engine, &profile, profile_path.as_deref(), &shared);

    let mut events = 0u64;
    let mut last_status = Instant::now();
    let mut last_drops = hook::dropped_events();

    loop {
        let now = Instant::now();
        if state.stop_requested() || stopping.load(Ordering::SeqCst) {
            break;
        }
        if state.take_reload() {
            reload(
                &mut profile,
                &mut engine,
                profile_path.as_deref(),
                &shared,
                &logger,
            );
            update_status(&state, &engine, &profile, profile_path.as_deref(), &shared);
            last_status = Instant::now();
        }

        // Panic key-down latched inside the hook callback. This covers the
        // case where the event itself was dropped from the full queue; the
        // normal in-band `TriggerCommand::Panic` path stays as it was.
        if hook::take_panic_press() {
            logger.log(
                Level::Warn,
                "Acil durdurma tetiklendi; tüm makrolar sonlandırılıyor.",
            );
            stopping.store(true, Ordering::SeqCst);
            shared.release_all();
            break;
        }

        match receiver.recv_timeout(Duration::from_millis(RECV_TIMEOUT_MS)) {
            Ok(InputEvent::Key { key, down, .. })
            | Ok(InputEvent::MouseButton { key, down, .. }) => {
                // Both arms feed the trigger engine: Linux delivers mouse
                // buttons as `EV_KEY` (`BTN_*`), so Windows must too.
                events += 1;
                let commands = engine.on_key(key, down, now);
                for command in commands {
                    dispatch(
                        command,
                        &profile,
                        &mut engine,
                        &shared,
                        &stopping,
                        &gate,
                        &logger,
                        &done_tx,
                    );
                }
            }
            Ok(_) => {
                events += 1;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                logger.log(Level::Error, "Kanca olay kanalı kapandı; motor duruyor.");
                break;
            }
        }

        while let Ok(done) = done_rx.try_recv() {
            match done {
                WorkerDone::Loop(macro_index) => engine.loop_finished(macro_index),
                WorkerDone::WhileHeld(macro_index) => engine.while_held_finished(macro_index),
            }
        }

        if last_status.elapsed() >= Duration::from_millis(STATUS_REFRESH_MS) {
            let drops = hook::dropped_events();
            if drops != last_drops {
                logger.log(
                    Level::Warn,
                    &format!("Kanca kuyruğu taştı; toplam {} olay düşürüldü", drops),
                );
                last_drops = drops;
            }
            update_status(&state, &engine, &profile, profile_path.as_deref(), &shared);
            last_status = Instant::now();
        }
    }

    stopping.store(true, Ordering::SeqCst);
    engine.reset_transient();
    {
        // Kapatma yarışı: iptal bayrağını görmeden `key(down)` göndermiş bir
        // işçi olabilir. Kapıyı kilitleyerek devam eden dilimin bitmesini
        // bekle, sonra bırak — böylece son enjeksiyon bizim bırakmamızdan
        // önce tamamlanır.
        let _guard = gate.lock();
        drop(handle);
        shared.release_all();
        // İşçilerin kendi dizilerini bırakmasına fırsat ver, sonra tekrar
        // bırak: kapı tutulurken işçi başlayamaz.
        thread::sleep(Duration::from_millis(50));
        shared.release_all();
    }

    let stats = shared.stats();
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    logger.log(
        Level::Info,
        &format!(
            "Motor durdu: {} olay, {} deneme, {} eklendi, {} eksik, {} ms",
            events, stats.attempts, stats.sent, stats.shortfalls, duration_ms
        ),
    );
    update_status(&state, &engine, &profile, profile_path.as_deref(), &shared);

    Ok(RuntimeSummary {
        events,
        triggers: engine.macros().len(),
        injected: stats,
        duration_ms,
    })
}

fn spawn_parent_watcher(pid: u32, state: Arc<ControlState>, logger: Arc<Logger>) {
    let worker_logger = Arc::clone(&logger);
    let spawned = thread::Builder::new()
        .name("macrostat-parent-watch".to_string())
        .spawn(move || {
            winutil::wait_for_process_exit(pid);
            worker_logger.log(
                Level::Warn,
                "Ana uygulama sonlandırıldı; motor temiz kapanıyor.",
            );
            state.request_stop();
        });
    if spawned.is_err() {
        logger.log(
            Level::Warn,
            "--parent-pid izleyicisi başlatılamadı; Job Object yedek olarak kullanılmalı.",
        );
    }
}

fn reload(
    profile: &mut ProfileConfig,
    engine: &mut TriggerEngine,
    path: Option<&Path>,
    shared: &SharedInjector,
    logger: &Arc<Logger>,
) {
    match load_profile(path) {
        Ok(new_profile) => {
            shared.release_all();
            let compiled = engine.reset_with(&new_profile);
            for warning in &compiled.warnings {
                logger.log(
                    Level::Warn,
                    &format!(
                        "Geçersiz tetikleyici '{}' (makro: '{}', token: '{}')",
                        warning.trigger_key, warning.macro_name, warning.token
                    ),
                );
            }
            logger.log(
                Level::Info,
                &format!(
                    "Profil yeniden yüklendi: '{}' ({} etkin tetikleyici)",
                    new_profile.name,
                    compiled.macros.len()
                ),
            );
            hook::set_panic_key(engine.panic_key());
            *profile = new_profile;
        }
        Err(error) => logger.log(
            Level::Warn,
            &format!("{error}; mevcut yapılandırma korunuyor."),
        ),
    }
}

fn update_status(
    state: &ControlState,
    engine: &TriggerEngine,
    profile: &ProfileConfig,
    path: Option<&Path>,
    shared: &SharedInjector,
) {
    let stats = shared.stats();
    state.set_status(StatusSnapshot {
        profile: profile.name.clone(),
        profile_path: path.map(|path| path.display().to_string()),
        macros: profile.macros.len(),
        triggers: engine.macros().len(),
        loops: engine.running_loops(),
        hook_dropped: hook::dropped_events(),
        injected_attempts: stats.attempts,
        injected_sent: stats.sent,
        injected_shortfalls: stats.shortfalls,
    });
}

#[allow(clippy::too_many_arguments)]
fn dispatch(
    command: TriggerCommand,
    profile: &ProfileConfig,
    engine: &mut TriggerEngine,
    shared: &SharedInjector,
    stopping: &Arc<AtomicBool>,
    gate: &ActionGate,
    logger: &Arc<Logger>,
    done_tx: &Sender<WorkerDone>,
) {
    match command {
        TriggerCommand::Panic => {
            logger.log(
                Level::Warn,
                "Acil durdurma tetiklendi; tüm makrolar sonlandırılıyor.",
            );
            stopping.store(true, Ordering::SeqCst);
            shared.release_all();
        }
        TriggerCommand::RunOnce { macro_index } => {
            let Some(actions) = actions_of(profile, engine, macro_index) else {
                return;
            };
            spawn_once(actions, macro_index, shared.clone(), stopping, gate, logger);
        }
        TriggerCommand::RunReleaseActions { macro_index } => {
            let Some(source_index) = engine.macro_at(macro_index).map(|m| m.source_index) else {
                return;
            };
            let actions = profile
                .macros
                .get(source_index)
                .map(|config| config.release_actions.clone())
                .unwrap_or_default();
            if actions.is_empty() {
                return;
            }
            spawn_once(actions, macro_index, shared.clone(), stopping, gate, logger);
        }
        TriggerCommand::StartLoop {
            macro_index,
            repeat,
            flag,
        } => {
            let Some(actions) = actions_of(profile, engine, macro_index) else {
                return;
            };
            let Some(compiled) = engine.macro_at(macro_index).cloned() else {
                return;
            };
            spawn_loop(
                actions,
                compiled,
                macro_index,
                repeat,
                flag,
                shared.clone(),
                Arc::clone(stopping),
                gate.clone(),
                Arc::clone(logger),
                done_tx.clone(),
            );
        }
        TriggerCommand::StopLoop { macro_index } => {
            logger.log(
                Level::Info,
                &format!("Döngü kapatıldı (makro #{macro_index}); tuşlar bırakılıyor."),
            );
            shared.release_all();
        }
        TriggerCommand::StartWhileHeld { macro_index, flag } => {
            let Some(actions) = actions_of(profile, engine, macro_index) else {
                return;
            };
            let Some(compiled) = engine.macro_at(macro_index).cloned() else {
                return;
            };
            spawn_while_held(
                actions,
                compiled,
                macro_index,
                flag,
                engine.held_handle(),
                shared.clone(),
                Arc::clone(stopping),
                gate.clone(),
                Arc::clone(logger),
                done_tx.clone(),
            );
        }
        TriggerCommand::StartHold {
            macro_index,
            key,
            hold_ms,
        } => {
            let Some(actions) = actions_of(profile, engine, macro_index) else {
                return;
            };
            spawn_hold(
                actions,
                key,
                hold_ms,
                engine.held_handle(),
                shared.clone(),
                Arc::clone(stopping),
                gate.clone(),
                logger,
            );
        }
    }
}

fn actions_of(
    profile: &ProfileConfig,
    engine: &TriggerEngine,
    macro_index: usize,
) -> Option<Vec<MacroAction>> {
    let source_index = engine.macro_at(macro_index)?.source_index;
    Some(profile.macros.get(source_index)?.actions.clone())
}

fn execute(
    actions: &[MacroAction],
    shared: &SharedInjector,
    stopping: &Arc<AtomicBool>,
    local: Option<&Arc<AtomicBool>>,
    gate: &ActionGate,
    logger: &Arc<Logger>,
) -> bool {
    let _guard = gate.lock();
    let cancel = WorkerCancel {
        stopping: Arc::clone(stopping),
        local: local.map(Arc::clone),
    };
    let mut injector = WorkerInjector {
        shared: shared.clone(),
    };
    let mut pressed = HashSet::new();
    let mut waiter = SleepWaiter;
    let mut jitter = RandJitter;
    let mut diagnostics = LogDiagnostics {
        logger: Arc::clone(logger),
    };
    let mut context = ActionContext {
        injector: &mut injector,
        pressed: &mut pressed,
        waiter: &mut waiter,
        cancel: &cancel,
        jitter: &mut jitter,
        diagnostics: &mut diagnostics,
    };
    execute_actions(actions, &mut context)
}

fn spawn_once(
    actions: Vec<MacroAction>,
    macro_index: usize,
    shared: SharedInjector,
    stopping: &Arc<AtomicBool>,
    gate: &ActionGate,
    logger: &Arc<Logger>,
) {
    let stopping = Arc::clone(stopping);
    let gate = gate.clone();
    let worker_logger = Arc::clone(logger);
    let spawned = thread::Builder::new()
        .name(format!("macrostat-once-{macro_index}"))
        .spawn(move || {
            execute(&actions, &shared, &stopping, None, &gate, &worker_logger);
        });
    if let Err(error) = spawned {
        logger.log(
            Level::Error,
            &format!("Tek seferlik iş parçacığı başlatılamadı: {error}"),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_loop(
    actions: Vec<MacroAction>,
    compiled: CompiledMacro,
    macro_index: usize,
    repeat: Option<u32>,
    flag: Arc<AtomicBool>,
    shared: SharedInjector,
    stopping: Arc<AtomicBool>,
    gate: ActionGate,
    logger: Arc<Logger>,
    done_tx: Sender<WorkerDone>,
) {
    let worker_logger = Arc::clone(&logger);
    let spawned = thread::Builder::new()
        .name(format!("macrostat-loop-{macro_index}"))
        .spawn(move || {
            let mut repetitions = 0u32;
            while flag.load(Ordering::SeqCst) && !stopping.load(Ordering::SeqCst) {
                if !execute(
                    &actions,
                    &shared,
                    &stopping,
                    Some(&flag),
                    &gate,
                    &worker_logger,
                ) {
                    break;
                }
                repetitions += 1;
                if repeat.is_some_and(|total| repetitions >= total) {
                    break;
                }
                thread::sleep(Duration::from_millis(LOOP_PAUSE_MS));
            }
            shared.release_all();
            let _ = done_tx.send(WorkerDone::Loop(macro_index));
            worker_logger.log(
                Level::Info,
                &format!(
                    "Döngü sonlandı: '{}' ({} tekrar)",
                    compiled.name, repetitions
                ),
            );
        });
    if let Err(error) = spawned {
        logger.log(
            Level::Error,
            &format!("Döngü iş parçacığı başlatılamadı: {error}"),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_while_held(
    actions: Vec<MacroAction>,
    compiled: CompiledMacro,
    macro_index: usize,
    flag: Arc<AtomicBool>,
    held: Arc<RwLock<HashSet<Key>>>,
    shared: SharedInjector,
    stopping: Arc<AtomicBool>,
    gate: ActionGate,
    logger: Arc<Logger>,
    done_tx: Sender<WorkerDone>,
) {
    let required = compiled.required_keys();
    let name = compiled.name.clone();
    let worker_logger = Arc::clone(&logger);
    let spawned = thread::Builder::new()
        .name(format!("macrostat-held-{macro_index}"))
        .spawn(move || {
            while flag.load(Ordering::SeqCst)
                && !stopping.load(Ordering::SeqCst)
                && all_held(&held, &required)
            {
                if !execute(
                    &actions,
                    &shared,
                    &stopping,
                    Some(&flag),
                    &gate,
                    &worker_logger,
                ) {
                    break;
                }
                thread::sleep(Duration::from_millis(LOOP_PAUSE_MS));
            }
            shared.release_all();
            let _ = done_tx.send(WorkerDone::WhileHeld(macro_index));
            worker_logger.log(Level::Info, &format!("Basılı tutma bırakıldı: '{name}'"));
        });
    if let Err(error) = spawned {
        logger.log(
            Level::Error,
            &format!("Basılı tutma iş parçacığı başlatılamadı: {error}"),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_hold(
    actions: Vec<MacroAction>,
    key: Key,
    hold_ms: u64,
    held: Arc<RwLock<HashSet<Key>>>,
    shared: SharedInjector,
    stopping: Arc<AtomicBool>,
    gate: ActionGate,
    logger: &Arc<Logger>,
) {
    let worker_logger = Arc::clone(logger);
    let spawned = thread::Builder::new()
        .name("macrostat-hold".to_string())
        .spawn(move || {
            let started = Instant::now();
            let target = Duration::from_millis(hold_ms);
            while started.elapsed() < target {
                if stopping.load(Ordering::SeqCst) || !all_held(&held, &[key]) {
                    return;
                }
                let remaining = target.saturating_sub(started.elapsed());
                thread::sleep(remaining.min(Duration::from_millis(HOLD_POLL_MS)));
            }
            if !stopping.load(Ordering::SeqCst) && all_held(&held, &[key]) {
                worker_logger.log(
                    Level::Info,
                    &format!("Uzun basım eşiği aşıldı ({hold_ms} ms); makro çalıştırılıyor."),
                );
                execute(&actions, &shared, &stopping, None, &gate, &worker_logger);
            }
        });
    if let Err(error) = spawned {
        logger.log(
            Level::Error,
            &format!("Uzun basım iş parçacığı başlatılamadı: {error}"),
        );
    }
}

fn all_held(held: &Arc<RwLock<HashSet<Key>>>, required: &[Key]) -> bool {
    let held = held.read().unwrap_or_else(|poisoned| poisoned.into_inner());
    required
        .iter()
        .all(|required| held.iter().any(|key| keys_are_equivalent(*required, *key)))
}
