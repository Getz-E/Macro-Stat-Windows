// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! MacroStat Windows engine binary.
//!
//! Phase 2 wires the input backend to two operations that can be verified
//! without IPC:
//!
//! * `--test-macro <id|name>` — run one macro's actions once through
//!   `SendInput` (the GUI's "test macro" button);
//! * `--record-mouse <file>` — record pointer movement and buttons with the
//!   low-level hook, in the Linux-compatible JSON schema.
//!
//! The long-running trigger/worker loop arrives with IPC and lifecycle in
//! Phase 3; this binary currently prints the compiled trigger state instead.

use std::process::ExitCode;

#[cfg(windows)]
use macrostat_engine::keymap::{self, Key};
use macrostat_engine::profile::{self, ProfileConfig, MAX_PROFILE_BYTES};
#[cfg(not(windows))]
use macrostat_engine::trigger::TriggerEngine;

#[cfg(windows)]
use macrostat_engine::actions::{execute_actions, ActionContext, RandJitter};
#[cfg(windows)]
use std::collections::HashSet;

#[derive(Debug, Default)]
struct Cli {
    config: Option<String>,
    test_macro: Option<String>,
    record_mouse: Option<String>,
    record_input: Option<String>,
    ipc: Option<String>,
    log: Option<String>,
    parent_pid: Option<u32>,
    watch_restart: Option<String>,
    panic_key: Option<String>,
    start_x: Option<i32>,
    start_y: Option<i32>,
    self_test: bool,
    help: bool,
    version: bool,
}

fn parse_args(raw: &[String]) -> Cli {
    let mut cli = Cli::default();
    let mut args = raw.iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--test-macro" => cli.test_macro = args.next().cloned(),
            "--record-mouse" => cli.record_mouse = args.next().cloned(),
            "--record-input" => cli.record_input = args.next().cloned(),
            "--ipc" => cli.ipc = args.next().cloned(),
            "--log" => cli.log = args.next().cloned(),
            "--parent-pid" => cli.parent_pid = args.next().and_then(|value| value.parse().ok()),
            "--watch-restart" => cli.watch_restart = args.next().cloned(),
            "--panic-key" => cli.panic_key = args.next().cloned(),
            "--start-x" => cli.start_x = args.next().and_then(|value| value.parse().ok()),
            "--start-y" => cli.start_y = args.next().and_then(|value| value.parse().ok()),
            "--self-test" => cli.self_test = true,
            "--help" | "-h" => cli.help = true,
            "--version" | "-V" => cli.version = true,
            other if !other.starts_with("--") => cli.config = Some(other.to_string()),
            _ => {}
        }
    }
    cli
}

fn print_help() {
    println!(
        "MacroStat motoru {}\n\nKullanım: macrostat-engine [PROFIL.json] [SEÇENEKLER]\n\nSeçenekler:\n  --test-macro <id|ad>     Bir makroyu bir kez çalıştır (Windows)\n  --record-mouse <dosya>   Fare hareketini kaydet (Windows)\n  --record-input <dosya>   Klavye + fare kaydet (Windows)\n  --ipc <ping|reload|stop|status>  Çalışan motorla kanal üzerinden konuş\n  --watch-restart <KEY_*>  Tuşa basılınca 42 koduyla çık (GUI yeniden başlatır)\n  --parent-pid <pid>       Ebeveyn çıkınca temiz kapan\n  --log <dosya>            Günlük yolunu geçersiz kıl\n  --panic-key <KEY_*>      Kayıt/durdurma tuşunu geçersiz kıl\n  --start-x <px>           Kaydın başlangıç X'i (--start-y ile)\n  --start-y <px>           Kaydın başlangıç Y'si\n  --self-test              Kanca/enjeksiyon tanısı (Windows, GUI'siz)\n  -h, --help               Bu yardımı yaz\n  -V, --version            Sürümü yaz\n\nProfil verilmezse gömülü varsayılan profil kullanılır.",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().collect();
    let cli = parse_args(&raw);

    if cli.help {
        print_help();
        return ExitCode::SUCCESS;
    }
    if cli.version {
        println!("macrostat-engine {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    #[cfg(windows)]
    {
        windows_main(&cli)
    }
    #[cfg(not(windows))]
    {
        linux_main(&cli)
    }
}

#[cfg(not(windows))]
fn linux_main(cli: &Cli) -> ExitCode {
    if cli.test_macro.is_some()
        || cli.record_mouse.is_some()
        || cli.record_input.is_some()
        || cli.watch_restart.is_some()
        || cli.ipc.is_some()
        || cli.parent_pid.is_some()
        || cli.self_test
    {
        eprintln!(
            "[HATA] Girdi backend'i (kanca/SendInput) yalnızca Windows'ta çalışır; \
             bu ikili Linux'ta yalnızca çekirdeği doğrular."
        );
        return ExitCode::from(5);
    }
    match load_profile(cli.config.as_deref()) {
        Ok(profile) => print_core_status(&profile),
        Err(error) => {
            eprintln!("[HATA] {error}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(not(windows))]
fn print_core_status(profile: &ProfileConfig) {
    let (engine, compiled) = TriggerEngine::new(profile);
    println!(
        "macrostat-engine {}: '{}' profilinde {} makro, {} etkin tetikleyici \
         (girdi backend'i {}).",
        env!("CARGO_PKG_VERSION"),
        profile.name,
        profile.macros.len(),
        engine.macros().len(),
        if cfg!(windows) {
            "etkin"
        } else {
            "bu platformda yok"
        },
    );
    for warning in compiled.warnings {
        println!(
            "[UYARI] Geçersiz tetikleyici '{}' (makro: '{}', token: '{}')",
            warning.trigger_key, warning.macro_name, warning.token
        );
    }
}

fn load_profile(path: Option<&str>) -> Result<ProfileConfig, String> {
    let Some(path) = path else {
        return Ok(profile::default_profile());
    };
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("Profil okunamadı ({path}): {error}"))?;
    if metadata.len() > MAX_PROFILE_BYTES {
        return Err(format!(
            "Profil çok büyük ({path}): {} bayt, sınır {} bayt",
            metadata.len(),
            MAX_PROFILE_BYTES
        ));
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("Profil okunamadı ({path}): {error}"))?;
    profile::parse_profile(&contents)
        .ok_or_else(|| format!("Profil geçersiz JSON ya da şema dışı ({path})"))
}

#[cfg(windows)]
fn resolve_panic_key(profile: &ProfileConfig, override_token: Option<&str>) -> Key {
    match override_token {
        Some(token) => keymap::parse(token).unwrap_or_else(|| {
            eprintln!(
                "[UYARI] Geçersiz --panic-key '{}'; profil değeri kullanılıyor.",
                token
            );
            profile.resolved_panic_key()
        }),
        None => profile.resolved_panic_key(),
    }
}

#[cfg(windows)]
fn windows_main(cli: &Cli) -> ExitCode {
    use macrostat_engine::winutil::{harden_dll_search, set_per_monitor_v2, TimerResolution};

    // DLL arama sırasını daralt: geçerli dizin ve PATH elenir. Herhangi bir
    // dinamik yüklemeden önce çalışmalıdır.
    if !harden_dll_search() {
        eprintln!("[UYARI] DLL arama yolu daraltılamadı (SetDefaultDllDirectories).");
    }

    let _dpi = set_per_monitor_v2();

    // IPC client mode needs neither a profile nor a runtime lock.
    if let Some(command) = cli.ipc.as_deref() {
        return run_ipc_command(command);
    }
    if let Some(token) = cli.watch_restart.as_deref() {
        return run_watch_restart(token);
    }
    if cli.self_test {
        return ExitCode::from(macrostat_engine::selftest::run());
    }

    let mut profile = match load_profile(cli.config.as_deref()) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("[HATA] {error}");
            return ExitCode::from(2);
        }
    };
    let panic_key = resolve_panic_key(&profile, cli.panic_key.as_deref());
    if let Some(token) = cli.panic_key.as_deref() {
        if keymap::parse(token).is_some() {
            profile.panic_key = token.to_string();
        }
    }

    if let Some(output) = cli.record_mouse.as_deref() {
        let _timer = TimerResolution::new(1);
        return run_record(output, panic_key, cli, false);
    }
    if let Some(output) = cli.record_input.as_deref() {
        let _timer = TimerResolution::new(1);
        return run_record(output, panic_key, cli, true);
    }
    if let Some(target) = cli.test_macro.as_deref() {
        let _timer = TimerResolution::new(1);
        return run_test_macro(&profile, target);
    }

    run_engine(profile, cli)
}

#[cfg(windows)]
fn run_record(output: &str, panic_key: Key, cli: &Cli, record_keys: bool) -> ExitCode {
    use macrostat_engine::record::{record_to_file, RECORD_MAX_SECS, RECORD_SAMPLE_MS};

    let start = match (cli.start_x, cli.start_y) {
        (Some(x), Some(y)) => Some([x, y]),
        (None, None) => None,
        _ => {
            eprintln!("[UYARI] --start-x ve --start-y birlikte verilmeli; yok sayıldı.");
            None
        }
    };
    let path = std::path::Path::new(output);
    println!(
        "[KAYIT] {} kaydediliyor ({} ms örnekleme, en çok {} sn); durdurmak için {}.",
        if record_keys {
            "Klavye + fare"
        } else {
            "Fare hareketi"
        },
        RECORD_SAMPLE_MS,
        RECORD_MAX_SECS,
        panic_key
    );
    println!("[KAYIT] Çıktı: {}", path.display());
    if let Some([x, y]) = start {
        println!("[KAYIT] Başlangıç imleç konumu: ({x}, {y})");
    }

    match record_to_file(path, panic_key, start, record_keys) {
        Ok(summary) => {
            println!(
                "[KAYIT] {} nokta, {} düğme olayı, {} tuş olayı, {} ms kaydedildi → {}",
                summary.points,
                summary.buttons,
                summary.keys,
                summary.duration_ms,
                path.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[HATA] Kayıt başarısız: {error}");
            ExitCode::from(4)
        }
    }
}

/// `--ipc <cmd>`: read `runtime.json`, talk to the running engine, print the
/// reply. Exit codes: 0 success, 6 connection/protocol failure.
#[cfg(windows)]
fn run_ipc_command(command: &str) -> ExitCode {
    use macrostat_engine::ipc::{send_command, Command};
    use macrostat_engine::runtime::RuntimeInfo;
    use macrostat_engine::winutil;

    if Command::parse(command).is_none() {
        eprintln!("[HATA] Bilinmeyen kanal komutu: {command}");
        return ExitCode::from(2);
    }
    let path = winutil::data_dir().join("runtime.json");
    let info: RuntimeInfo = match macrostat_engine::runtime::read_runtime_file(&path)
        .map_err(|error| error.to_string())
    {
        Ok(info) => info,
        Err(error) => {
            eprintln!("[HATA] runtime.json okunamadı: {error}");
            return ExitCode::from(6);
        }
    };
    // Dosya kurcalanmış olabilir: UNC/başka kullanıcı kanalına bağlanma.
    if let Err(error) = macrostat_engine::runtime::validate_runtime_info(&info) {
        eprintln!("[HATA] runtime.json doğrulanamadı: {error}");
        return ExitCode::from(6);
    }
    match send_command(&info.pipe_name, command) {
        Ok(reply) => {
            print!("{reply}");
            if !reply.ends_with('\n') {
                println!();
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[HATA] Kanal hatası: {error}");
            ExitCode::from(6)
        }
    }
}

/// `--watch-restart <KEY_*>`: install the hook, wait for the key, exit 42.
#[cfg(windows)]
fn run_watch_restart(token: &str) -> ExitCode {
    use macrostat_engine::hook::{self, InputEvent};

    let Some(watch) = keymap::parse(token) else {
        eprintln!("[HATA] Geçersiz --watch-restart tuşu: {token}");
        return ExitCode::from(2);
    };
    let (handle, receiver) = match hook::start(256) {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("[HATA] Kanca kurulamadı: {error}");
            return ExitCode::from(5);
        }
    };
    println!("[İZLE] {watch} tuşu bekleniyor; basılınca 42 koduyla çıkılacak.");
    loop {
        match receiver.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(InputEvent::Key {
                key, down: true, ..
            })
            | Ok(InputEvent::MouseButton {
                key, down: true, ..
            }) if key == watch => {
                drop(handle);
                println!("[İZLE] {watch} basıldı; yeniden başlatma isteği.");
                return ExitCode::from(42);
            }
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                drop(handle);
                eprintln!("[HATA] Kanca olay kanalı kapandı.");
                return ExitCode::from(5);
            }
        }
    }
}

/// Long-running mode: single instance, named pipe, runtime.json, trigger loop.
#[cfg(windows)]
fn run_engine(profile: ProfileConfig, cli: &Cli) -> ExitCode {
    use std::sync::Arc;

    use macrostat_engine::ipc::{self, ControlState};
    use macrostat_engine::log::{Level, Logger, LOG_FILE_NAME};
    use macrostat_engine::runtime::{self, RuntimeInfo, RuntimeOptions};
    use macrostat_engine::winutil::{self, SingleInstance, SingleInstanceError, TimerResolution};

    let Some(sid) = winutil::current_user_sid_string() else {
        eprintln!("[HATA] Kullanıcı SID'i okunamadı; IPC güvenli biçimde açılamaz.");
        return ExitCode::from(6);
    };
    let mutex_name = format!("Local\\MacroStat.Engine.{sid}");
    let _instance = match SingleInstance::acquire(&mutex_name) {
        Ok(instance) => instance,
        Err(SingleInstanceError::AlreadyRunning) => {
            eprintln!("[BILGI] Motor zaten çalışıyor ({mutex_name}).");
            return ExitCode::from(7);
        }
        Err(error) => {
            eprintln!("[HATA] Tek örnek kilidi alınamadı: {error}");
            return ExitCode::from(6);
        }
    };

    let data_dir = winutil::data_dir();
    let log_path = cli
        .log
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| data_dir.join(LOG_FILE_NAME));
    let logger = Arc::new(Logger::new(&log_path));
    let pipe_name = ipc::pipe_name(&sid, &ipc::random_token());
    let elevated = winutil::is_elevated();
    let log_display = log_path.display().to_string();
    let state = Arc::new(ControlState::new(
        std::process::id(),
        pipe_name.clone(),
        log_display.clone(),
        elevated,
        sid.clone(),
    ));

    let server = match ipc::start(Arc::clone(&state)) {
        Ok(server) => server,
        Err(error) => {
            logger.log(Level::Error, &format!("IPC sunucusu açılamadı: {error}"));
            eprintln!("[HATA] IPC sunucusu açılamadı: {error}");
            return ExitCode::from(6);
        }
    };

    let runtime_path = data_dir.join("runtime.json");
    let info = RuntimeInfo {
        pid: std::process::id(),
        creation_time: winutil::process_creation_time(std::process::id()).unwrap_or(0),
        exe_path: std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        sid: sid.clone(),
        pipe_name: pipe_name.clone(),
        log_path: log_display.clone(),
    };
    if let Err(error) = runtime::write_runtime_file(&runtime_path, &info) {
        logger.log(
            Level::Error,
            &format!(
                "runtime.json yazılamadı ({}): {error}",
                runtime_path.display()
            ),
        );
        eprintln!("[HATA] runtime.json yazılamadı: {error}");
        return ExitCode::from(6);
    }

    let _timer = TimerResolution::new(1);
    logger.log(
        Level::Info,
        &format!(
            "IPC hazır: {} (yükseltilmiş: {elevated}), günlük: {log_display}",
            pipe_name
        ),
    );
    println!(
        "[BILGI] Motor çalışıyor (PID {}, kanal {}). Durdurmak için panik tuşu ya da `--ipc stop`.",
        std::process::id(),
        pipe_name
    );

    let result = runtime::run(RuntimeOptions {
        profile,
        profile_path: cli.config.as_deref().map(std::path::PathBuf::from),
        state: Arc::clone(&state),
        logger: Arc::clone(&logger),
        parent_pid: cli.parent_pid,
    });

    remove_runtime_file(&runtime_path);
    drop(server);

    match result {
        Ok(summary) => {
            println!(
                "[BILGI] Motor kapandı: {} olay, {} makro, {} deneme/{} eklendi, {} ms.",
                summary.events,
                summary.triggers,
                summary.injected.attempts,
                summary.injected.sent,
                summary.duration_ms
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            logger.log(Level::Error, &format!("Motor hatası: {error}"));
            eprintln!("[HATA] Motor hatası: {error}");
            ExitCode::from(5)
        }
    }
}

/// Remove `runtime.json` only when it still belongs to this process.
#[cfg(windows)]
fn remove_runtime_file(path: &std::path::Path) {
    // Sınırlı okuma: dosya büyütülmüş olsa da belleğe sınırsız alınmaz.
    let Ok(info) = macrostat_engine::runtime::read_runtime_file(path) else {
        return;
    };
    if info.pid == std::process::id() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(windows)]
fn run_test_macro(profile: &ProfileConfig, target: &str) -> ExitCode {
    use macrostat_engine::inject::{NeverCancel, SleepWaiter, StderrDiagnostics, WindowsInjector};

    let Some(macro_config) = profile
        .macros
        .iter()
        .find(|candidate| candidate.id == target || candidate.name == target)
    else {
        eprintln!("[HATA] Test edilecek makro bulunamadı: {target}");
        return ExitCode::from(3);
    };

    let mut actions = macro_config.actions.clone();
    actions.extend(macro_config.release_actions.iter().cloned());
    if actions.is_empty() {
        eprintln!("[HATA] '{}' makrosunda eylem yok.", macro_config.name);
        return ExitCode::from(4);
    }

    println!(
        "[TEST] '{}' makrosu çalıştırılıyor ({} eylem, {} bırakma eylemi).",
        macro_config.name,
        macro_config.actions.len(),
        macro_config.release_actions.len()
    );

    let mut injector = WindowsInjector::new();
    let mut pressed = HashSet::new();
    let mut waiter = SleepWaiter;
    let mut jitter = RandJitter;
    let mut diagnostics = StderrDiagnostics;
    let cancel = NeverCancel;
    let completed = {
        let mut context = ActionContext {
            injector: &mut injector,
            pressed: &mut pressed,
            waiter: &mut waiter,
            cancel: &cancel,
            jitter: &mut jitter,
            diagnostics: &mut diagnostics,
        };
        execute_actions(&actions, &mut context)
    };

    let stats = injector.stats();
    if stats.shortfalls > 0 {
        eprintln!(
            "[UYARI] SendInput {} çağrıda eksik olay ekledi ({} deneme, {} eklendi). \
             Yükseltilmiş bir pencere ön plandaysa UIPI engellemiş olabilir.",
            stats.shortfalls, stats.attempts, stats.sent
        );
    }
    if !completed {
        eprintln!("[TEST] Eylemler iptal edildi; basılı tuşlar bırakıldı.");
        return ExitCode::from(5);
    }
    println!(
        "[TEST] Tamamlandı: {} olay denendi, {} eklendi, {} eksik.",
        stats.attempts, stats.sent, stats.shortfalls
    );
    ExitCode::SUCCESS
}
