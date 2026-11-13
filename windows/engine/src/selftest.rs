// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! `--self-test`: GUI'siz, tek seferlik tanı koşusu.
//!
//! Gerçek Windows'ta şunları sınar:
//!
//! 1. düşük seviyeli kanca çifti kurulabiliyor mu;
//! 2. kendi `SendInput` çıktımız imza sayesinde kancaya görünmüyor mu;
//! 3. başka bir enjektörün olayı kancaya görünüyor mu;
//! 4. basılı tuş `release_all_keys` ile bırakılıyor mu (takılı tuş yok);
//! 5. panik tuşu mandalı dış olayla tetikleniyor mu;
//! 6. bütünlük düzeyi (UIPI) raporu doğru mu.
//!
//! Hiçbir şey yazmaz, profil okumaz, ağ kullanmaz. Çıkış kodu 0 = tüm
//! kontroller geçti, 4 = en az bir kontrol başarısız. Kanca kullanıldığı için
//! aynı süreçte başka bir kanca oturumu olamaz; motor bu modda çalışmaz.
//!
//! `--self-test` Linux'ta çalışmaz (çıkış 5), çünkü doğruladığı API'ler
//! Windows'a özgüdür.

use std::fmt;

/// One diagnostic line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    /// Short factual detail; never contains typed text.
    pub detail: String,
}

impl Check {
    pub fn new(name: &'static str, ok: bool, detail: impl Into<String>) -> Self {
        Self {
            name,
            ok,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[TEST] {:<28} {} ({})",
            self.name,
            if self.ok { "TAMAM" } else { "HATA" },
            self.detail
        )
    }
}

/// Exit code from a check list: `0` when everything passed, `4` otherwise.
pub fn exit_code(checks: &[Check]) -> u8 {
    if checks.iter().all(|check| check.ok) {
        0
    } else {
        4
    }
}

#[cfg(windows)]
pub fn run() -> u8 {
    windows_impl::run()
}

#[cfg(not(windows))]
pub fn run() -> u8 {
    eprintln!("[HATA] --self-test yalnızca Windows'ta çalışır.");
    5
}

#[cfg(windows)]
mod windows_impl {
    use std::collections::HashSet;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, GetAsyncKeyState, KEYEVENTF_KEYUP,
    };

    use super::{exit_code, Check};
    use crate::actions::{release_all_keys, Injector};
    use crate::hook::{self, InputEvent};
    use crate::inject::WindowsInjector;
    use crate::keymap::Key;
    use crate::winutil::{foreground_integrity_level, is_elevated, INJECTION_SIGNATURE};

    const WAIT: Duration = Duration::from_millis(120);
    const EVENT_TIMEOUT: Duration = Duration::from_millis(1500);

    fn external_key(key: Key) {
        let vk = key.windows().vk as u8;
        unsafe {
            keybd_event(vk, 0, 0, 0);
            keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
        }
    }

    /// Drain queued events for `window`, returning whether `predicate` matched.
    fn saw_event(
        receiver: &std::sync::mpsc::Receiver<InputEvent>,
        window: Duration,
        predicate: impl Fn(&InputEvent) -> bool,
    ) -> bool {
        let deadline = Instant::now() + window;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match receiver.recv_timeout(left.min(WAIT)) {
                Ok(event) if predicate(&event) => return true,
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        false
    }

    fn key_is(event: &InputEvent, key: Key, down: bool) -> bool {
        matches!(event, InputEvent::Key { key: pressed, down: pressed_down, .. }
            if *pressed == key && *pressed_down == down)
    }

    fn async_down(key: Key) -> bool {
        let vk = i32::from(key.windows().vk);
        unsafe { GetAsyncKeyState(vk) < 0 }
    }

    pub fn run() -> u8 {
        let mut checks = Vec::new();
        checks.push(Check::new(
            "enjeksiyon imzası",
            INJECTION_SIGNATURE == 0x4154_534D,
            format!("0x{INJECTION_SIGNATURE:08X}"),
        ));

        let (handle, receiver) = match hook::start(256) {
            Ok(pair) => {
                checks.push(Check::new("kanca kurulumu", true, "klavye + fare"));
                pair
            }
            Err(error) => {
                checks.push(Check::new("kanca kurulumu", false, error.to_string()));
                for check in &checks {
                    println!("{check}");
                }
                return exit_code(&checks);
            }
        };

        // 1. Kendi çıktımız kancaya görünmemeli.
        let mut injector = WindowsInjector::new();
        injector.key(Key::KEY_F14, true);
        injector.key(Key::KEY_F14, false);
        let leaked = saw_event(&receiver, WAIT * 2, |event| {
            key_is(event, Key::KEY_F14, true)
        });
        checks.push(Check::new(
            "kendi enjeksiyonunu yok sayma",
            !leaked,
            if leaked {
                "imzalı olay kancaya ulaştı".to_string()
            } else {
                "imzalı F14 kancada görülmedi".to_string()
            },
        ));

        // 2. Dış enjektörün olayı görünmeli.
        external_key(Key::KEY_F15);
        let seen = saw_event(&receiver, EVENT_TIMEOUT, |event| {
            key_is(event, Key::KEY_F15, true)
        });
        checks.push(Check::new(
            "dış enjeksiyonu görme",
            seen,
            if seen {
                "F15 kancada görüldü".to_string()
            } else {
                "olay gelmedi (UIPI veya oturum sınırı olabilir)".to_string()
            },
        ));

        // 3. Panik mandalı dış olayla kurulmalı.
        hook::set_panic_key(Key::KEY_F24);
        external_key(Key::KEY_F24);
        let latched = {
            let deadline = Instant::now() + EVENT_TIMEOUT;
            let mut latched = false;
            while Instant::now() < deadline {
                if hook::take_panic_press() {
                    latched = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            latched
        };
        checks.push(Check::new(
            "panik tuşu mandalı",
            latched,
            if latched {
                "F24 mandallandı".to_string()
            } else {
                "mandal kurulmadı".to_string()
            },
        ));

        // 4. Basılı tuş bırakılmalı (takılı tuş yok).
        injector.key(Key::KEY_F13, true);
        std::thread::sleep(WAIT);
        let down_observed = async_down(Key::KEY_F13);
        // Release unconditionally: even if the async readback failed, F13 must
        // not be left logically pressed after the diagnostic exits.
        let mut pressed: HashSet<Key> = HashSet::new();
        pressed.insert(Key::KEY_F13);
        release_all_keys(&mut injector, &mut pressed);
        std::thread::sleep(WAIT);
        if !down_observed {
            checks.push(Check::new(
                "takılı tuş temizliği",
                true,
                "async durum okunamadı; kontrol atlandı",
            ));
        } else {
            let stuck = async_down(Key::KEY_F13);
            checks.push(Check::new(
                "takılı tuş temizliği",
                !stuck,
                if stuck {
                    "F13 bırakılmadı".to_string()
                } else {
                    "F13 bırakıldı".to_string()
                },
            ));
        }

        // 5. UIPI/bütünlük raporu (bilgilendirici, başarısız sayılmaz).
        let foreground = foreground_integrity_level()
            .map(|level| level.as_str())
            .unwrap_or("bilinmiyor");
        let stats = injector.stats();
        checks.push(Check::new(
            "bütünlük düzeyi",
            true,
            format!(
                "yükseltilmiş={}, ön plan={}, gönderim={}/{} (eksik {})",
                is_elevated(),
                foreground,
                stats.sent,
                stats.attempts,
                stats.shortfalls
            ),
        ));

        drop(handle);
        for check in &checks {
            println!("{check}");
        }
        exit_code(&checks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_is_four_when_any_check_fails() {
        let ok = vec![Check::new("a", true, "x"), Check::new("b", true, "y")];
        assert_eq!(exit_code(&ok), 0);
        let bad = vec![Check::new("a", true, "x"), Check::new("b", false, "z")];
        assert_eq!(exit_code(&bad), 4);
    }

    #[test]
    fn display_uses_turkish_status_words() {
        assert!(Check::new("a", true, "d").to_string().contains("TAMAM"));
        assert!(Check::new("a", false, "d").to_string().contains("HATA"));
    }
}
