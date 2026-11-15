# Adım Raporu — Faz 4 PyQt6 Arayüzü (Windows)

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


| | |
|---|---|
| **Tarih** | 10 Kasım 2026 |
| **Faz** | 4 — PyQt6 arayüzünün Windows'a taşınması |
| **Durum** | Kod tamamlandı; 39 GUI birim testi geçti; gerçek Windows'ta uçtan uca doğrulama Faz 6'da |
| **Kapsam** | `windows/gui/` — `app`, `common`, `system`, `recorder`, `winplatform`, `onboarding` + Linux'tan uyarlanan modüller |
| **Kabul ölçütü** | Tüm sayfalar çalışır; TR/EN; profil içe/dışa; onboarding yönetici izni istemez; profil biçimi sözleşmeyle aynı |

## 1. Özet

Linux arayüzü (`linux/macrostat/`) davranış paritesi korunarak Windows'a taşındı. Platformdan bağımsız modüller (profil yönetimi, düzenleyici, transfer, tema, i18n, sanitize) kopyalanıp yalnızca yol/OS varsayımları uyarlandı; Linux'a özgü yarılar (`udev`, `/proc`, `fcntl`, `/dev/uinput`, `SIGUSR1`, X11/KWin) tamamen kaldırıldı ve yerine Win32/kanal/Job Object katmanı geldi.

**Arayüzün motorla ilişkisi:**

- Motor, arayüzün alt süreci olarak `[profile, "--parent-pid", <gui_pid>]` ile başlatılır; `CREATE_NO_WINDOW` ve `DEVNULL` stdio kullanılır (motor kendi günlüğünü yazar).
- Süreç bir Job Object'e (`KILL_ON_JOB_CLOSE | BREAKAWAY_OK`) atanır; arayüz çökerse çekirdek motoru sonlandırır. `--parent-pid` zarif yoldur.
- Komutlar (`ping`/`reload`/`stop`) Rust istemcisiyle `macrostat-engine.exe --ipc <cmd>` alt süreci üzerinden gönderilir; kanalın SID DACL'i ve `runtime.json` belirteci doğrulaması Rust tarafında kalır.
- Canlılık, `runtime.json` + PID oluşturulma zamanı + görüntü adı üçlüsüyle doğrulanır (PID yeniden kullanımı ayıklanır).

**Sonuçlar:** `py_compile` ✅ · başsız (offscreen) 4 sayfa kurulumu ✅ · editör/profil/sistem yolları ✅ · `unittest` **39/39** ✅ · sabit `T()` metinlerinin **%100'ü** EN sözlüğünde ✅ · `linux/` dokunulmadı ✅.

> Bu ortam Linux'tur. Gerçek kanca, `SendInput`, `QProcess` ile EXE çalıştırma ve Job Object davranışı burada doğrulanamaz; ifadeler derleme, içe aktarma ve sahte (mock) katman testlerini kasteder. Gerçek doğrulama CI `windows-latest` + TEST-MATRIX §6'dadır.

## 2. Üretilen/değişen dosyalar

| Dosya | İçerik |
|---|---|
| `gui/main.py` | **Yeni** giriş noktası (geliştirme ve PyInstaller) |
| `gui/macrostat/app.py` | **Yeni**: `MacroStatApp`, `QLockFile` tek örnek, DPI, koyu başlık çubuğu, `restart_arguments()` |
| `gui/macrostat/winplatform.py` | **Yeni** katman: `OpenProcess`/`GetProcessTimes`/`QueryFullProcessImageNameW`/`GetExitCodeProcess`/`TerminateProcess`, `OpenProcessToken`+`TokenElevation`, `CreateJobObjectW`, `GetCursorPos`, `SendInput` (Pause), `DwmSetWindowAttribute`, DPI, reparse-point |
| `gui/macrostat/common.py` | **Yeni**: `%APPDATA%`/`%LOCALAPPDATA%` yolları, PATH'siz motor arama, atomik/özel dosya I/O, `runtime.json`, `ipc_command`, `stop_engine`, profil tohumlama |
| `gui/macrostat/system.py` | **Yeni**: motor başlatma/durdurma, Job Object, tanılama, yedekleme/geri yükleme, STOP + `--watch-restart` (çıkış 42), `closeEvent` temizliği |
| `gui/macrostat/recorder.py` | **Yeni**: motor `--record-input` tabanlı sistem geneli kayıt, `merge_input_events`, sanitize, `SendInput` Pause ile güvenli durdurma |
| `gui/macrostat/onboarding.py` | **Yeni**: yönetici izni istemeyen 3 adım (kontroller, Windows notları, motor onayı) |
| `gui/macrostat/i18n.py` | Windows'a özgü 55 metnin EN karşılığı (`_EN`) |
| `gui/macrostat/core.py` | `restart_application` → `restart_arguments()` (frozen EXE/geliştirme ayrımı) |
| `gui/macrostat/editor.py` | Makro testinde motor konsolu gizlenir (`hide_console`) |
| `gui/macrostat/logpanel.py` | Döndürme algısı `st_ino` yoksa `st_mtime_ns` gerilemesine düşer |
| `gui/assets/default_profile.json` | Motor şablonunun bayt kopyası (tohumlama/paketleme) |
| `gui/assets/macrostat.svg` | Pencere simgesi |
| `tests/test_gui_windows.py` | **Yeni**: 39 birim testi |
| `docs/TEST-MATRIX.md` | §6.1 Faz 4 sonuçları |
| `README.md` | Durum, yerleşim, arayüz bölümü |

Değişmeyenler: `sanitize.py`, `transfer.py`, `widgets.py`, `nav.py`, `macros.py`, `theme.py`, `settings.py` (Linux kopyası; ayar yolları zaten Windows yerleşimine uygun).

## 3. Süreç ve yaşam döngüsü

```
MacroStat.exe / python gui/main.py
   │  QLockFile (tek örnek) + DPI farkındalığı
   ├─ PyQt6 arayüzü (SystemMixin motoru yönetir)
   │     ├─ Popen macrostat-engine.exe [profile, --parent-pid, GUI_PID]
   │     └─ AssignProcessToJobObject (KILL_ON_JOB_CLOSE)
   ├─ --ipc ping/reload/stop  (kısa ömürlü istemci süreçleri)
   └─ --watch-restart KEY_*   (yalnızca STOP sonrası; QProcess, çıkış 42)
```

| Durum | Arayüz davranışı |
|---|---|
| Kaydedilmiş profil + motor | Otomatik başlatma (onboarding bitince) |
| Kullanıcı STOP | Motor durur, `is_manually_stopped`; yeniden başlatma tuşu dinleyicisi açılır |
| Tuş dinleyicisi çıkış 42 | Üst banner + motor `force=True` ile yeniden başlar |
| Profil kaydı | Motor çalışıyorsa `reload` (yeniden başlatma yok); değilse yeniden başlat |
| Editör açık | Motor durdurulur; çıkışta profil yeniden uygulanır |
| Panik tuşu | Günlükte `Acil durdurma tetiklendi` imzası; arayüz "DURDURULDU" rozetini gösterir |
| Arayüz kapanışı | Dinleyici durur, tuşlar bırakılır, kayıt süreci 1,5 sn içinde sonlanır, motor durur, Job Object kapanır |

## 4. Kayıt akışı (Windows'a özgü)

Linux'taki Qt içi kayıt yerine kayıt **motorda** yapılır:

1. "Kaydı Başlat" → çalışan motor durdurulur, `--record-input <tmp> --panic-key KEY_PAUSE [--start-x/--start-y]` başlar.
2. Kanca sistem genelinde klavye + fare düğmeleri + topçuk + hareket noktalarını zaman damgalarıyla toplar.
3. "Durdur" → `SendInput` ile **sentinel Pause** (VK 0x13, tarama 0x45, enjeksiyon imzası yok) gönderilir; kayıtçı bunu gerçek panik tuşu görüp dosyayı atomik yazar.
4. Enjeksiyon başarısızsa/zaman aşımında süreç sonlandırılır ve kullanıcıya kayıp riski bildirilir.
5. Arayüz JSON'u okur, sınırları uygular (`sanitize_mouse_buttons`, `sanitize_key_events`, nokta/süre tavanları), `merge_input_events` ile `MOUSE_PATH` / `DELAY` / `KEY_*` / `MOUSE_*` eylemlerine çevirir; `start` ilk `MOUSE_PATH`'e iliştirilir.
6. Motor kayıttan önce çalışıyorduysa yeniden başlatılır.

Fare hareketi istenmezse yalnızca noktalar atılır; tuş/düğme olayları korunur.

## 5. Güvenlik kararları

| Konu | Uygulama |
|---|---|
| Motor arama | `shutil.which` **yok**; sabit aday listesi; reparse-point (symlink/junction) reddi |
| `runtime.json` | Normal dosya, 64 KiB sınırı, katı tip denetimi (`pid>0`, `creationTime>=0`, `pipeName` boş değil) |
| PID yeniden kullanımı | `process_creation_time` + görüntü adı eşleşmesi |
| Profil/günlük | Atomik yazma (mkstemp + `fsync` + `os.replace`), 32 MiB sınırı, sembolik bağlantı reddi |
| Yetki | `asInvoker`; onboarding hiçbir adımda yükseltme istemez; UIPI/güvenli masaüstü sınırları arayüzde açıklanır |
| Ağ | Yok: telemetri, güncelleme, uzak kanal yok (`PIPE_REJECT_REMOTE_CLIENTS` motor tarafında) |
| Tek örnek | Kullanıcıya özel `%LOCALAPPDATA%\MacroStat\macrostat-gui.lock` (`QLockFile`) |
| Çıkış temizliği | `aboutToQuit` + `closeEvent` çift yol; Job Object çekirdek yedeği |

## 6. Testler

```
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v
# Ran 39 tests ... OK
```

| Grup | Kapsam |
|---|---|
| `RuntimeInfoTests` | Geçerli/bozuk `runtime.json`, bool PID, eksik alan |
| `IpcTests` | Son satır JSON ayrıştırma, `ok:false`, sıfır olmayan çıkış kodu, motor yokluğu |
| `EngineRunningTests` | Ölü süreç, PID yeniden kullanımı, yabancı görüntü, kimlik eşleşmesi |
| `PrivateFileTests` | Atomik tur, boyut sınırı, bozuk UTF-8, symlink reddi, yedek, günlük döndürme |
| `SanitizerTests` / `MergeInputTests` | Düğme/tuş doğrulama, zaman kırpma, nokta–olay birleştirme ve `DELAY` hesabı |
| `PathAndKeyTests` | Yeniden başlatma tuşu normalizasyonu, `--watch-restart` komutu, şablon varlığı |
| `TranslationCoverageTests` | Her sabit `T("...")` metni `_EN`'de; `{}` sayıları eşleşir |
| `SettingsTests` | Bozuk dil/tema → güvenli varsayılan |
| `OnboardingCheckTests` | Beklenen 5 kontrol; yönetici gerektiren kontrol yok |

Ek olarak başsız duman testleri: uygulama kurulumu, 4 sayfa, sistem tanılaması, kayıt tablosu, düzenleyici yükleme, sihirbaz kurulumu — hata yok.

## 7. Bilinen farklar ve sınırlar

| Konu | Durum |
|---|---|
| Gerçek Windows E2E | Faz 6'da (CI `windows-latest`); bu raporda yalnızca kod/statik doğrulama var |
| `QIcon.fromTheme` | Windows'ta sistem tema ikonu yok; düğmeler metinle çalışır (işlevsel kayıp yok) |
| Motor ikilisi yoksa | Kayıt ve makro testi anlaşılır uyarı verir; sistem sekmesi derleme komutunu gösterir |
| Yönetici hedefler | UIPI belgelenmiş sınırdır; arayüz yönetici çalıştırmayı önermez, isteğe bağlı bırakır |
| Kayıt süresi/sayısı | Motorla aynı: 600 s, 200 000 nokta; sınırda dosya yine yazılır |
| İmzalama | Kapsam dışı (Faz 7); SmartScreen notu sihirbazda açıklanır |

## 8. Sonraki adım

Faz 5 — paketleme ve CI: PyInstaller (onedir), Inno Setup kurulumu, `windows/scripts/` derleme/imza/doğrulama, GitHub Actions `windows-latest` iş akışı, motor + GUI + şablonun tek kurulumda birleştirilmesi.
