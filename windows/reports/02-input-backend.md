# Adım Raporu — Faz 2 Windows Girdi Backend'i

| | |
|---|---|
| **Tarih** | 10 Kasım 2026 |
| **Faz** | 2 — Windows girdi backend'i |
| **Durum** | Kod tamamlandı; gerçek Windows E2E doğrulaması bekliyor |
| **Kapsam** | `hook`, `inject`, `mouse`, `winutil`, `record`, `main` (kayıt + tek makro testi) |
| **Kod durumu** | Linux'ta çekirdek + saf yardımcılar derleniyor/test ediliyor; Windows hedefi `cargo check/clippy` ile doğrulandı |

## 1. Özet

Faz 2'nin kabul ölçütü olan Windows girdi katmanı yazıldı: küresel `WH_KEYBOARD_LL`/`WH_MOUSE_LL` kancaları, `SendInput` tabanlı klavye/fare/metin enjeksiyonu, sanal masaüstü koordinat normalizasyonu, bütünlük düzeyi (UIPI) önsezisi ve Linux ile aynı şemada `--record-mouse` kaydı. Kendi enjeksiyonumuz `dwExtraInfo = 0x4154534D` ("MSTA") imzasıyla kancada ayıklanır; `LLKHF_INJECTED` yalnızca tanılama için taşınır.

`windows-sys 0.61` (ham FFI) kullanıldı; üst düzey `windows` sarmalayıcısı gerekmedi. Windows API çağrıları `#[cfg(windows)]` arkasında; saf mantık (VK→tuş ters eşlemesi, koordinat normalizasyonu, teker birimi, UTF-16, kayıt birleştirme, bütünlük RID sınıflandırması) Linux'ta test edilebilir durumda.

**Sonuçlar:** Linux `fmt` ✅ · Linux `clippy -D warnings` ✅ · `cargo test --all` ✅ **78/78** · üretici `--check` ✅ · Windows hedefi `cargo check --all-targets` ✅ · Windows hedefi `clippy -D warnings` ✅.

> Gerçek Windows'ta kanca/enjeksiyon davranışı bu ortamda çalıştırılamadı (Linux'ta MSVC bağlayıcısı yok). CI `windows-latest` bu testleri ve E2E matrisini çalıştıracak; aşağıdaki "doğrulandı" ifadeleri aksi belirtilmedikçe derleme/statik doğrulamayı kasteder.

## 2. Üretilen/değişen dosyalar

| Dosya | İçerik |
|---|---|
| `engine/Cargo.toml` | `[target.'cfg(windows)'.dependencies] windows-sys 0.61` + 8 feature |
| `engine/Cargo.lock` | `windows-sys`, `windows-link` kilitlendi |
| `engine/tools/gen_keymap.py` | `from_windows(vk, extended, scan)` + `MouseButton::key()` üretimi; çakışma denetimleri |
| `engine/src/keymap.rs` | **Üretilmiş**: ters eşleme, fare düğmesi→kanonik tuş, 3 yeni test |
| `engine/src/winutil.rs` | Geniş string, enjeksiyon imzası, DPI, bütünlük düzeyi, yükseltme, timer çözünürlüğü |
| `engine/src/mouse.rs` | `VirtualScreen`, 0–65535 normalizasyonu, teker birimi, imleç konumu |
| `engine/src/inject.rs` | `SendInput` enjektörü, Unicode/denetim karakterleri, `SleepWaiter`, `FlagCancel` |
| `engine/src/hook.rs` | Kanca iş parçacığı, olay kuyruğu, imza filtresi, fare/klavye çözümleme |
| `engine/src/record.rs` | Linux şemalı kayıt, 4 ms birleştirme, 200k/600 sn sınırları, atomik yazım |
| `engine/src/main.rs` | CLI: `--test-macro`, `--record-mouse`, `--panic-key`, `--start-x/y`, `--help`, `--version` |
| `reports/02-input-backend.md` | Bu rapor |
| `docs/TEST-MATRIX.md` | Faz 2 sonuçları ve E2E bekleme durumu |
| `README.md` | Durum Faz 2, rapor bağlantısı |

## 3. API sözleşmesi (Faz 3 için)

### 3.1 Kanca (`hook`)

```rust
let (handle, rx) = hook::start(4096)?;
while let Ok(event) = rx.recv() {
    match event {
        InputEvent::Key { key, down, time_ms, injected } => { /* TriggerEngine */ }
        InputEvent::MouseMove { x, y, .. } => { /* kayıt */ }
        InputEvent::MouseButton { key, down, .. } => { /* BTN_* tetikleyici */ }
        InputEvent::Wheel { delta_units, .. } => { /* 120 birim/diş */ }
    }
}
drop(handle); // WM_QUIT + join
```

| Öğe | Davranış |
|---|---|
| Kurulum | Ayrı `macrostat-hook` iş parçacığı; `SetWindowsHookExW(WH_KEYBOARD_LL/WH_MOUSE_LL, modül, 0)` |
| Hazır sinyali | İş parçacığı kimliği ana iş parçacığına döner; `PeekMessageW` ile kuyruk önceden kurulur |
| Kuyruk | Sınırlı `sync_channel`; `try_send`; dolduğunda olay düşer ve sayaç artar |
| Kendi olaylarımız | `dwExtraInfo == INJECTION_SIGNATURE` → kancada yok sayılır |
| Dış enjeksiyonlar | `injected: true` ile teslim edilir (Linux'taki diğer `uinput` aygıtları gibi) |
| Bastırma | **Yok** — Linux paritesi; panik tuşunu da engellerdi (bkz. §8) |
| Ömür | Süreç başına tek kanca seti (`OnceLock`); Faz 3'te yeniden başlatma ayrı ele alınacak |

### 3.2 Enjeksiyon (`inject`)

`WindowsInjector`, Faz 1'deki `actions::Injector` trait'ini uygular; böylece 11 eylem ve iptal mantığı değişmeden çalışır.

| Trait metodu | Windows karşılığı |
|---|---|
| `key(key, down)` | Klavye: tarama kodu varsa `KEYEVENTF_SCANCODE`, yoksa VK; `KEYEVENTF_EXTENDEDKEY`; `KEYUP` |
| `key(BTN_*, down)` | `MOUSEEVENTF_LEFT/RIGHT/MIDDLE/X DOWN/UP`; X için `XBUTTON1/2` |
| `mouse_move(dx,dy)` | `MOUSEEVENTF_MOVE` (göreli) |
| `mouse_wheel(delta)` | `MOUSEEVENTF_WHEEL`, `delta × 120` |
| `mouse_absolute(x,y)` | `MOVE|ABSOLUTE|VIRTUALDESK`, 0–65535 normalizasyonu |
| `text_char(ch)` | `KEYEVENTF_UNICODE` ile UTF-16 kod birimi; `\n`/`\r` → Enter, `\t` → Tab; NUL/diğer denetim karakterleri reddedilir |

Her `INPUT` `dwExtraInfo = INJECTION_SIGNATURE` taşır. `SendInput` dönüşü 1 değilse `shortfalls` sayacı artar; bu **kesin UIPI kanıtı değildir** ve yalnızca uyarı olarak gösterilir.

### 3.3 Kayıt (`record`)

- Şema Linux ile birebir: `version`, `sample_ms = 4`, `duration_ms`, `points [[dt,dx,dy]]`, `buttons [[t,"BTN_*",1|0]]`, `start`.
- Kanca mutlak imleç konumu verir; kayıtçı bunu delta'ya çevirip 4 ms dilimlerinde birleştirir.
- Sınırlar: profil `MAX_MOUSE_PATH_POINTS` (200 000) ve 600 s.
- Yazım: `dosya.tmp` + `sync_all` + `rename` (yarım dosya kalmaz). Windows ACL sıkılaştırması Faz 3.
- Başlangıç konumu `GetCursorPos` ile okunur; `--start-x/--start-y` ile geçersiz kılınabilir.

### 3.4 `winutil`

| Fonksiyon | Kullanım |
|---|---|
| `INJECTION_SIGNATURE` | "MSTA" imzası; kanca filtresi + enjeksiyon |
| `IntegrityLevel`, `from_rid`, `can_drive` | UIPI önsezisi; saf sınıflandırma |
| `process_integrity_level(pid)` | `OpenProcess` → token → `TokenIntegrityLevel` → son SID alt-otoritesi |
| `foreground_integrity_level()` | Ön plan penceresinin bütünlük düzeyi |
| `is_elevated()` | `TokenElevation` |
| `set_per_monitor_v2()` | `SetProcessDpiAwarenessContext` |
| `TimerResolution` | `timeBeginPeriod(1)` … `timeEndPeriod` (RAII) |
| `wide()` | NUL sonlandırmalı UTF-16 |

## 4. Windows API seçimleri

| Karar | Gerekçe |
|---|---|
| `windows-sys 0.61` | Yalnız kullanılan imzaları bağlar; `windows` crate'inin COM/üst düzey yükü yok; MSRV 1.71, proje 1.79 |
| Ham FFI + ince sarmalayıcı | Kanca callback'i sıcak yol; ara katman allocation/kilit eklemesin |
| `WH_KEYBOARD_LL` / `WH_MOUSE_LL` | Linux `evdev` okumasının karşılığı; sürücü gerekmez |
| `SendInput` + tarama kodu | `uinput` enjeksiyonunun karşılığı; TR-Q/US düzeninde aynı fiziksel tuş |
| `dwExtraInfo` imzası | Linux'taki ayrı sanal aygıt ayrımının Windows karşılığı |
| `MOUSEEVENTF_VIRTUALDESK` | Çoklu monitör/negatif koordinat desteği |
| `KEYEVENTF_UNICODE` | Linux'un ASCII sınırını aşan Windows iyileştirmesi |
| `timeBeginPeriod(1)` + `Sleep` | Faz 2 tek seferlik çalıştırma için yeterli; QPC/waitable timer işçi katmanıyla Faz 3 |

## 5. Linux ↔ Windows backend eşdeğerliği

| Davranış | Linux | Faz 2 Windows | Durum |
|---|---|---|---|
| Klavye okuma | `evdev` | `WH_KEYBOARD_LL` | ✅ |
| Fare okuma | `evdev` | `WH_MOUSE_LL` | ✅ |
| Kendi olayını ayıklama | Ayrı sanal aygıt | `dwExtraInfo` imzası | ✅ |
| Dış enjeksiyon | Ayırt edilemez | `LLKHF_INJECTED` yalnız bilgi | ✅ Parite |
| Tuş enjeksiyonu | `uinput` | `SendInput` tarama/VK | ✅ |
| Sol/sağ modifier | Ayrı kodlar | Ayrı VK'lar | ✅ |
| Enter / KP Enter | Ayrı kodlar | VK 0x0D + extended ayrımı | ✅ |
| Pause / NumLock | Ayrı kodlar | VK öncelikli (tarama 0x45 çakışması) | ✅ |
| F13–F24 | Ayrı kodlar | Yalnız VK yolu | ✅ |
| Fare düğmeleri | `BTN_*` | `MOUSEEVENTF_*` / XBUTTON | ✅ |
| Tekerlek | `REL_WHEEL` diş | 120 birim/diş | ✅ |
| Göreli hareket | `REL_X/Y` | `MOUSEEVENTF_MOVE` | ✅ |
| Mutlak konum | X11/Wayland | Sanal masaüstü + 0–65535 | ✅ |
| Metin | ASCII | Unicode + satır sekmesi | ➕ İyileştirme |
| Olay bastırma | Yok | Yok | ✅ Parite |
| Kayıt şeması | 4 ms / 200k / 600 s | Aynı | ✅ |
| Otomatik tekrar filtresi | evdev `value == 2` | Faz 1 motorundaki basılı küme | ✅ |
| Bütünlük düzeyi | Kavram yok | Önsezi + günlük | ➕ Windows |

## 6. Testler

| Modül | Adet | Yeni kapsam |
|---|---|---|
| `actions` | 15 | (Faz 1) |
| `profile` | 10 | (Faz 1) |
| `trigger` | 21 | (Faz 1) |
| `keymap` | 14 | +3: tüm tuşlarda Windows→kanonik gidiş-dönüş, Enter/KP Enter & Pause/NumLock ayrımı, tarama yedeği, fare düğmesi eşlemesi |
| `winutil` | 4 | RID→bütünlük düzeyi, UIPI yönü, UTF-16/geniş string, imza baytları |
| `mouse` | 5 | Tek/çoklu monitör kenarları, negatif origin, gidiş-dönüş toleransı, dejenere ekran, teker birimi |
| `inject` | 3 | BMP/surrogate UTF-16, denetim karakteri reddi, paylaşılan iptal bayrağı |
| `record` | 6 | 4 ms birleştirme, düğme adları, ilk hareketin `start` olması, sıfır delta, sınırlar, Linux şema anahtarları |
| **Toplam** | **78** | |

## 7. Doğrulama komutları ve sonuçları

```bash
cd windows/engine
/usr/bin/cargo-fmt --all -- --check
/usr/bin/cargo-clippy --all-targets -- -D warnings
cargo test --all
python3 tools/gen_keymap.py --check

# Windows hedefi (Linux üzerinde çapraz derleme; bağlama yok)
~/.cargo/bin/cargo check  --target x86_64-pc-windows-msvc --all-targets
~/.cargo/bin/cargo clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
```

| Kontrol | Sonuç |
|---|---|
| `cargo fmt --check` | ✅ |
| Linux `clippy -D warnings` | ✅ 0 uyarı |
| `cargo test --all` | ✅ 78/78 |
| `gen_keymap.py --check` | ✅ güncel |
| Windows hedefi `cargo check --all-targets` | ✅ |
| Windows hedefi `cargo clippy -D warnings` | ✅ |
| Windows testlerini bağlama/çalıştırma | ⚠️ Linux'ta `link.exe` yok; `windows-latest` CI'da çalışacak |
| `cargo run` (Linux) | ✅ varsayılan profil bilgisi |
| `cargo run -- --test-macro x` (Linux) | ✅ exit 5 + açıklayıcı hata |
| `linux/` dokunulmadı | ✅ mtime denetimi |

## 8. Kararlar ve notlar

1. **Bastırma (suppression) yok.** Linux da girdiyi yutamıyor; ayrıca bütün klavyeyi yutmak panik tuşunu da engellerdi. Tetikleyiciye özgü bastırma ileride ayrı bir yetenek olarak, panik tuşu istisnasıyla değerlendirilecek (D-13).
2. **Kendi imzamız filtrelenir, dış enjeksiyonlar işlenir.** Böylece makro kendini yeniden tetiklemez; başka araçların enjeksiyonları Linux paritesi gereği gerçek girdi sayılır ve `injected` alanıyla işaretlenir.
3. **VK + extended öncelikli, tarama kodu yedek.** `0x0D` (Enter/KP Enter), `0x45` (Pause/NumLock) ve benzeri çakışmalar VK ile çözülür; sürücü beklenmedik VK bildirirse tarama kodu denenir. Üretici aynı `(vk, extended)` veya `(scan, extended)` çiftine iki tuş düşerse **hata verir**.
4. **Mutlak koordinat formülü** `(x - left) × 65535 / (width - 1)`, yuvarlamalı ve `0..=65535` sınırlı. Son piksel tam 65535'e, negatif origin tam 0'a düşer; dejenere genişlikte bölme yok.
5. **`SendInput` eksikliği kesin UIPI değildir.** Sayaç yalnız uyarı üretir; Windows engellenen pencereyi bildirmez. Kullanıcıya "yönetici hedef engellendi" denmeden önce bütünlük düzeyi önsezisiyle birlikte değerlendirilir.
6. **Metin denetim karakterleri.** `\n`/`\r` Enter, `\t` Tab olarak gerçek tuşla gönderilir; NUL ve diğer C0 karakterleri reddedilir (Linux da ASCII dışını atlıyordu). Unicode surrogate çiftleri ayrı ayrı `KEYEVENTF_UNICODE` olayıdır.
7. **Zamanlayıcı.** Faz 2 tek makro testi `SleepWaiter` + `timeBeginPeriod(1)` kullanır. İptal edilebilir, QPC tabanlı waitable-timer işçisi Faz 3'te; `Waiter` trait'i bunu bekliyor.
8. **Kanca ömrü.** `OnceLock` nedeniyle süreçte tek kanca seti kurulabilir; kayıt ve test ayrı süreçlerde çalışır. Faz 3 yeniden başlatma/yeniden yükleme için kanca sahipliği genişletilecek.
9. **Kayıt dosyası ACL'i.** Atomik yazım var; kullanıcıya özel ACL sertleştirmesi Faz 3 güvenlik adımında.
10. **`--start-x/y` yalnız birlikte.** Tek koordinat verilirse uyarıyla yok sayılır.

## 9. Gerçek Windows'ta doğrulanacaklar

| ID | Senaryo | Durum |
|---|---|---|
| I-01 | Notepad'de tetikleme + enjeksiyon | ⏳ |
| I-02 | Kendi enjeksiyonunu yok sayma | ⏳ (imza kodu hazır) |
| I-03 | STOP tuşu | ⏳ (kayıtta hazır; sürekli döngü Faz 3) |
| I-04 | Basılı tuşta çıkış | ⏳ (Faz 1 iptal mantığı + Faz 3 süreç çıkışı) |
| I-05 | Auto-repeat | ⏳ (Faz 1 motoru) |
| I-06 | Yükseltilmiş ön plan | ⏳ + UIPI rozeti Faz 4 |
| I-07 | MacroStat yönetici olarak | ⏳ |
| I-08 | UAC/kilit ekranı | ⏳ belgelenmiş sınır |
| I-09 | Unicode metin | ⏳ |
| I-10 | `MOUSE_PATH` çoklu monitör | ⏳ (normalizasyon testli) |
| I-11 | DPI %200 | ⏳ |
| I-12 | TR-Q / US düzeni | ⏳ (tarama kodu yolu) |
| I-13 | Metin reddeden uygulama | ⏳ |

Ek olarak `MapVirtualKeyW(vk, MAPVK_VK_TO_VSC)` çapraz kontrolü (Faz 1'den devir) hâlâ gerçek Windows gerektirir; bu fazda tablo ters eşlemeyle tutarlı hale getirildi, ancak işletim sistemi doğrulaması yapılmadı.

## 10. Faz 3'e taşınanlar

1. IPC (`ping/reload/stop/status`), tek örnek mutex'i ve yaşam döngüsü.
2. Sürekli tetikleyici döngüsü + işçi iş parçacıkları (loop, WHILE_HELD, HOLD, panik).
3. QPC + waitable timer ile iptal edilebilir hassas bekleme; `timeBeginPeriod` ömrü.
4. Günlük dosyası, rotasyon, kullanıcı ACL'i; kayıt dosyası ACL'i.
5. Kanca sahipliğinin yeniden başlatma/yeniden yükleme için genişletilmesi.
6. UIPI rozeti ve günlük entegrasyonu; `SendInput` eksikliğinin bütünlük önsezisiyle birleştirilmesi.
7. Gerçek Windows E2E matrisi (I-01…I-13) ve `MapVirtualKeyW` raporu.

## 11. Değişmez kural kontrolü

- `linux/` altında hiçbir dosya yazılmadı/değiştirilmedi (mtime denetimi).
- Tüm yeni/değişen dosyalar `windows/` altında.
- Ağ bağlantısı, telemetri veya otomatik güncelleme eklenmedi.
- Çekirdek sürücüsü, anti-cheat atlatma, `uiAccess` yok.
- Yeni bağımlılık yalnız `windows-sys` (Windows hedefi; ham Win32 bağları).
