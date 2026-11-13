# MacroStat — Linux İncelemesi, Güvenlik Değerlendirmesi ve Windows EXE Taşıma Raporu

| | |
|---|---|
| **Rapor tarihi** | 9 Kasım 2026 |
| **Sürüm** | 1.0 (inceleme + fizibilite + plan) |
| **Durum** | İnceleme ve planlama tamamlandı; **uygulama başlamadı** |
| **Kapsam** | `linux/` yalnızca okundu; **hiçbir dosya değiştirilmedi**. Windows'a ilişkin tüm çıktılar `windows/` altında üretilir. |
| **Kaynaklar** | Yerel `linux/` ağacı, https://github.com/Getz-E/Macro-Stat.git, Microsoft Learn (Windows yaşam döngüsü, Artifact Signing), PyPI/PyQt6 sürüm bilgisi |

---

## 1. Yönetici Özeti

MacroStat, Linux için yazılmış **tamamen çevrimdışı, düşük gecikmeli bir masaüstü makro motorudur**. İki bağımsız parçadan oluşur:

1. **Rust motor** (`linux/src/main.rs`, 2.700 satır; `linux/src/kwin_input.rs`, 89 satır): Linux `evdev` ile fiziksel klavye/fare olaylarını okur, `uinput` ile sistem geneline sanal girdi enjekte eder. Tetikleyici derleme, 7 tetikleme kipi, 11 eylem türü, acil STOP, profil sıcak yenileme, tek örnek kilidi, günlük döndürme ve KDE Wayland imleç hizalaması içerir.
2. **PyQt6 arayüzü** (`linux/macrostat/`, 18 modül, ~6.600 satır): Makrolar, Makro Düzenleyici, Canlı Tuş Kaydedici ve Sistem & Kılavuz sayfaları; profil yönetimi, içe/dışa aktarma, yedekleme, tema, Türkçe/İngilizce dil, ilk kurulum sihirbazı, canlı günlük paneli ve motor yaşam döngüsü yönetimi.

Sürüm **1.0.0**, lisans **GPL-3.0-or-later** (PyQt6'nın GPLv3 lisansı nedeniyle). Desteklenen Linux dağıtımları: Ubuntu 24.04+, Debian 13+, Fedora 41+, Arch/CachyOS. Projede **ağ bağlantısı, telemetri, sunucu, veritabanı, API, kimlik doğrulama veya parola saklama yoktur.**

### 1.1 Sonuç

**Proje Windows'a taşınabilir.** PyQt6 Windows'ta birinci sınıf desteklidir ve arayüzün büyük bölümü platformdan bağımsızdır. Rust motorun iş mantığı (profil, tetikleyici, eylemler, kiplar, jitter, mandal) korunur; yalnızca Linux G/Ç katmanı (`evdev`, `uinput`, KWin/X11, sinyaller, `/proc`, udev) Windows API'leriyle değiştirilir.

Hedef çıktılar:

1. **`MacroStat.exe`** — PyQt6 arayüzü (PyInstaller, `--windowed`, onedir).
2. **`macrostat-engine.exe`** — Rust motor (MSVC hedefi, statik CRT, DPI-aware manifest).
3. **`MacroStat-Setup-1.0.0.exe`** — Inno Setup kurulum dosyası (kullanıcı bazlı, yönetici gerektirmez).
4. **`MacroStat-1.0.0-win-x64-portable.zip`** — taşınabilir sürüm.

### 1.2 Kesin sınırlar

| Sınır | Durum | Neden |
|---|---|---|
| Yönetici (elevated) pencerelere enjeksiyon | **Normal yetkiyle imkânsız** | UIPI; `SendInput` engellenir, yükseltilmiş ön planda kanca da olay almaz. Çözüm: isteğe bağlı "yönetici olarak çalıştır" |
| Güvenli masaüstü (UAC istemi, kilit ekranı, Ctrl+Alt+Del) | **İmkânsız** | Ayrı güvenli masaüstü; kanca ve enjeksiyon erişemez |
| Gerçek sanal HID aygıtı | **Kapsam dışı** | İmzalı çekirdek sürücüsü gerektirir |
| Kernel anti-cheat oyunlarında çalışma | **Belirsiz** | Kanca/enjeksiyon tespit edilebilir; tespit atlatma geliştirilmeyecek |
| Ağ/otomatik güncelleme | **Kapsam dışı** | Proje çevrimdışı kalacak |
| `uiAccess` ayrıcalığı | **Bu sürümde yok** | İmzalı sertifika + Program Files koşulu gerekir |

**Sonuç cümlesi:** Windows sürümü, normal masaüstü uygulamalarında Linux sürümüyle işlevsel parite sağlar; yönetici pencereler ve güvenli masaüstü bilinçli olarak kapsam dışıdır ve kullanıcıya açıkça belgelenir.

### 1.3 Efor

Tek deneyimli geliştirici ve Windows CI/VM erişimi varsayımıyla **~270–430 saat** (yaklaşık **7–11 hafta**). En riskli bölüm, girdi backend'i (kancalar + `SendInput` + kenar durumları) ve E2E doğrulamadır.

---

## 2. Yöntem ve İnceleme Sınırları

### 2.1 Yapılanlar

- `linux/` ağacındaki kaynak dosyalar, testler, dokümantasyon, profil şeması, paketleme ve kurulum betikleri **birinci elden okundu**.
- Rust motorun CLI davranışı, kilit yönetimi, tetikleyici/eylem mantığı ve sınırları incelendi.
- PyQt6 arayüzünün modül yapısı, profil yaşam döngüsü, güvenlik yardımcıları ve test kapsamı incelendi.
- GitHub deposu (varsayılan dal `main`, 10 commit, yayın kaydı yok, Windows desteği yok) kontrol edildi.
- Windows fizibilitesi için Microsoft Learn (Windows 10/11 yaşam döngüsü, Azure Artifact Signing), PyQt6 tekerlek durumu ve PyInstaller/Inno Setup derleme modeli araştırıldı.

### 2.2 Sınırlar ve uyarılar

- Yerel `linux/` kopyası bir **git deposu değildir** (`.git` yok); bu nedenle commit geçmişi yerel olarak doğrulanamadı.
- Yerel kopyada `.github/workflows/` görünmüyor. README'de CI/sürüm bütünlüğü anlatılsa da iş akışı dosyaları bu kopyada yok; Windows CI planı **yeni workflow** varsayar.
- Depoda önceden derlenmiş Linux ikilileri bulunuyor (`macrostat-daemon`, `target/release/`). Windows sürümünde kaynak + CI derlemesi + SHA-256 esas alınacak; depoya önceden derlenmiş EXE konmayacak.
- İşbu rapor bir **planlama** raporudur; hiçbir Windows kodu veya kurulum dosyası henüz üretilmedi.

---

## 3. Envanter

### 3.1 Dosya envanteri

| Alan | Dosyalar | Satır/Not |
|---|---|---|
| Rust motor | `src/main.rs` (2.700), `src/kwin_input.rs` (89) | `evdev 0.12`, `uinput`, `libc`, `rand`, `wayland-client/scanner` |
| PyQt6 GUI | `macrostat/*.py` (18 modül) | `common.py` 887, `core.py` 808, `editor.py` 694, `i18n.py` 591, `recorder.py` 536, `system.py` 481, `transfer.py` 404, `macros.py` 381, `theme.py` 338, `onboarding.py` 257, `widgets.py` 267, `logpanel.py` 207, `kwin.py` 149, `sanitize.py` 110, `settings.py` 75, `app.py` 74 |
| Giriş noktaları | `gui.py`, `start-macrostat.sh`, `packaging/macrostat` | Python, bash başlatıcı, paket başlatıcı |
| Profil | `default_profile.json`, `assets/default_profile.json`, `assets/macrostat.svg` | Şema sürümü 1; demo makrolar kapalı |
| Protokol | `protocols/fake-input.xml` | KDE `org_kde_kwin_fake_input` v6 (yalnızca Wayland imleç hizalama) |
| Paketleme | `packaging/99-macrostat.rules`, `install-common.sh` (617), `macrostat.desktop`; `PKGBUILD`; `install_all.sh`, `install_{ubuntu,debian,fedora,arch}.sh`; `uninstall.sh` (147) | udev `uaccess`, pkexec/sudo, dağıtım paketleri, kısayollar |
| Testler | 7 dosya: `test_core.py` (1.160), `test_installer.py` (569), `test_exec_security.py` (472), `test_widgets.py` (348), `test_runtime_files.py` (336), `test_uninstall.py` (177), `test_assets.py` (43) | Python `unittest`; Rust birim testleri `main.rs` içinde |
| Dokümantasyon | `README.md` (26 KB, TR+EN), `SECURITY.md` (12,6 KB, TR+EN), `THIRD_PARTY.md`, GUI tasarım belgesi | Güvenlik modeli ve GUI tasarımı |
| Derleme çıktıları | `macrostat-daemon` (762 KB ELF), `target/release/macrostat-daemon` | Yerel derlenmiş motor |
| Bağımlılıklar | `requirements.txt` → `PyQt6>=6.5,<7`; `Cargo.lock` | Rust 1.79 MSRV; Python 3.10+ |

### 3.2 Depo durumu

| Öğe | Gözlem |
|---|---|
| Varsayılan dal | `main` |
| Commit sayısı | 10 |
| Yayınlanmış sürüm | Görünmüyor |
| Lisans | GPL-3.0-or-later |
| Windows desteği | Yok |
| Yerel git | Yok (`.git` bulunmuyor) |
| Yerel CI dosyaları | `.github/workflows/` yok |

---

## 4. Ürün Amacı ve Yetenekler

### 4.1 Motor davranışı

- Tetikleyiciler açılışta derlenir; her tuş olayında yeniden ayrıştırılmaz.
- Sol/sağ Ctrl, Shift, Alt ve Meta eşdeğer kabul edilir; bir kombinasyon, tuşlardan biri bırakılıp yeniden basılmadıkça tekrar ateşlenmez (mandal).
- Aynı anda çalışan makrolar tek yürütme kilidiyle (`ACTION_LOCK`) serileştirilir; tuş durumu karışmaz.
- Tetikleme kipleri: `ONCE`, `WHILE_HELD`, `TOGGLE_LOOP`, `REPEAT_COUNT`, `DOUBLE_TAP`, `HOLD_LONG_PRESS`, `PRESS_AND_RELEASE`.
- Eylem türleri: `KEY_DOWN`, `KEY_UP`, `KEY_TAP`, `MOUSE_DOWN`, `MOUSE_UP`, `MOUSE_CLICK`, `MOUSE_MOVE`, `MOUSE_SCROLL`, `MOUSE_PATH`, `TEXT`, `DELAY` (jitter destekli).
- `TEXT` Linux'ta yalnızca US-ASCII yazar; ASCII dışı karakterler atlanır ve bir kez uyarı loglanır.
- `MOUSE_PATH`: `[geçen_ms, dx, dy]` üçlüleri; en fazla 200.000 nokta, adım başına en fazla 600.000 ms; kayıt 4 ms'de bir birleştirilir; üst sınır 10 dakika. Başlangıç konumu X11/KDE Wayland'da saklanır ve oynatmada imleç oraya taşınır.
- Acil STOP: varsayılan `KEY_PAUSE`; STOP sonrası motor, arayüzden Yeniden Başlat veya atanmış tuş (varsayılan `KEY_F12`) ile açılana kadar kapalı kalır. STOP'ta hafif `--watch-restart` izleyicisi çalışır ve `42` çıkış koduyla GUI'yi tetikler.
- `SIGTERM`/`SIGINT` ve pencere kapanışında basılı sanal tuşlar bırakılır; `PR_SET_PDEATHSIG` ile GUI ölürse motor kapanır.
- Aynı kullanıcı için ikinci motor `flock` ile engellenir; aygıt ekleme/çıkarma `inotify` ile, yedek olarak 4 saniyelik yoklamayla izlenir.
- Günlük: `$XDG_RUNTIME_DIR/macrostat.log` (yoksa `~/.cache/macrostat`), `0600`, 1 MB'ta döndürme.
- Motor kendi sanal aygıtını (`MacroStat`, `CachyMacro`, `Virtual Engine` adlarıyla) filtreleyerek geri besleme döngüsünü önler.

### 4.2 Arayüz özellikleri

- **Gezinme:** sol kenar çubuğu — Makrolar, Makro Düzenleyici, Canlı Tuş Kaydedici, Sistem & Kılavuz.
- **Üst çubuk:** profil seçici + profil işlemleri (`⋯`), motor durum rozeti, **STOP**, STOP sonrası **Yeniden Başlat**, taşma menüsü (günlük, motoru yeniden başlat, dil, tema, STOP tuşu, yeniden başlatma tuşu).
- **İlk kurulum sihirbazı:** 3 adım — Sistem Durumu (✓/✗), İzinleri Düzelt (yalnızca `uaccess` udev kuralı), Hazır (motoru başlatma seçeneği); sonuç `settings.json` → `onboarding_completed`.
- **Canlı günlük paneli:** metin filtresi, otomatik kaydırma, duraklat/devam, temizle, kapat; en fazla 5.000 satır.
- **Bas-bırak çift eylemi:** `releaseActions` (yalnızca `PRESS_AND_RELEASE` modunda).
- **Tema:** Sistem/Koyu/Açık (varsayılan koyu), anında uygulanır.
- **Dil:** Türkçe/İngilizce; seçim `settings.json`'a yazılır, arayüz yeniden başlar.
- **Profil yönetimi:** yeni, yeniden adlandır, sil (son profil silinemez), içe/dışa aktarma (aktif profil veya tüm profiller tek JSON).
- **Yedekleme:** her kayıtta `profile.json.bak`; sistem sekmesinden tek tıkla geri yükleme; ayrıca tüm profilleri dışa aktarma.
- **Makroyu test et:** kaydedilmemiş eylemlerle `--test-macro` modu; ayrı sanal aygıt adı.
- **Canlı kaydedici:** Linux'ta pencere içi Qt olaylarıyla klavye/fare düğmeleri/tekerlek; sistem geneli fare yolu ayrı `--record-mouse` alt süreciyle `evdev` üzerinden.
- **Wayland uyarı bandı:** kaydın yalnızca pencere odaktayken çalıştığı uyarısı (Linux'a özgü).

### 4.3 Profil şeması (Windows'ta korunacak sözleşme)

```json
{
  "schemaVersion": 1,
  "activeProfileId": "...",
  "profiles": [{
    "id": "profile-...",
    "name": "...",
    "description": "...",
    "panicKey": "KEY_PAUSE",
    "restartKey": "KEY_F12",
    "virtualDeviceName": "MacroStat Virtual Engine",
    "defaultJitterMs": 2,
    "macros": [{
      "id": "macro-...",
      "name": "...",
      "description": "...",
      "enabled": false,
      "triggerKey": "KEY_LEFTSHIFT+KEY_LEFTCTRL",
      "triggerMode": "ONCE",
      "repeatCount": 1,
      "doubleTapTimeoutMs": 300,
      "holdDurationMs": 300,
      "actions": [],
      "releaseActions": [],
      "createdAt": 0,
      "updatedAt": 0
    }]
  }]
}
```

Kimlik kuralı: `[A-Za-z0-9_-]{1,64}`. Tuş adları `KEY_*` / `BTN_*` (motor ayrıca `F9`, `PAUSE` gibi düz adları da çözer). Profil üst sınırı 32 MiB; içe aktarma 5 MiB, 500 makro, 100 profil, 64 KiB metin.

---

## 5. Güvenlik İncelemesi

### 5.1 Belgelenen tehdit modeli (`SECURITY.md`)

| Varlık | Korunma |
|---|---|
| Profil + `.bak` | `0600`; atomik yazma (`mkstemp` + `fsync` + `os.replace`); motor `O_NOFOLLOW` |
| Günlük/PID/kilit | Özel çalışma dizini (`XDG_RUNTIME_DIR` veya `~/.cache/macrostat` 0700); `O_NOFOLLOW`, `0600` |
| Kaydedici geçici dosyaları | `mkstemp`, tahmin edilemez adlar |
| Dışa aktarılan yedekler | `mkstemp` + `0600` |
| Motor ikilisi | Sabit aday listesi, `PATH` yok, güvenilir kullanıcı/kök sahipliği ve güvenli üst dizinler |
| Ayrıcalıklı yardımcılar (`pkexec`/`sudo`, `sh`, `modprobe`, `cat`, `udevadm`) | Sabit kök dizinler, `PATH` yok, yol/symlink/ACL denetimi |
| udev | Yalnızca `TAG+="uaccess"` + `MODE="0660"`; `0666` veya `input` grubu yok |
| Profil/içe aktarma metni | Kontrol, bidi/zero-width ve `<`/`>` temizliği; uzunluk/sayı sınırları |
| Kimlikler | `[A-Za-z0-9_-]{1,64}`; geçersizler yeniden üretilir |
| Sanal aygıt adı | Kontrol karakterleri temizlenir, 80 karakter |
| Kurulum betikleri | Yalnızca dağıtım paket yöneticisi; kullanıcı girdisi kabuğa gömülmez |

Ek olarak: PID kimliği (`pid`, başlangıç zamanı, uid, çalıştırılabilir yol) doğrulanmadan sinyal gönderilmez; `pkill -f` kullanılmaz; günlük `0600`; sürüm bütünlüğü için SHA-256 + Sigstore attestation + RustSec `cargo audit`; CI adımları commit SHA'sına sabitlenir.

### 5.2 Kod düzeyinde öne çıkan bulgular

- **`macrostat/sanitize.py`:** `ID_RE`, `KEY_PART_RE`, `BTN_PART_RE`, `PLAIN_KEY_RE`, kontrol/görünmez karakter regexleri, `coerce_int`.
- **`macrostat/common.py`:** `_runtime_dir`, özel dosya açma (`O_NOFOLLOW`), atomik yazma, yedekleme, `PATH`'siz güvenilir çalıştırılabilir çözümleme (`trusted_system_executable`, `_resolve_verified_path`, POSIX ACL xattr okuma), PID kimliği, `_terminate_daemon_pid`.
- **`macrostat/system.py`:** daemon başlatma/durdurma, pidfile, `SIGUSR1` yenileme, restart watcher.
- **`src/main.rs`:** profil ayrıştırma ve sınırlar; sanal aygıt adı temizliği; kendi aygıtını filtreleme; kilit dosyası sahiplik kontrolü (`uid`, `O_NOFOLLOW`, `flock`); eylem parametrelerinde `clamp`; `MAX_MOUSE_PATH_POINTS` / `MAX_MOUSE_PATH_STEP_MS`.
- **`macrostat/transfer.py`:** 5 MiB dosya, 500 makro, 100 profil, 64 KiB metin; atomik dışa aktarma; içe aktarmada kimlik yenileme, mevcut veriyi ezmeme.

### 5.3 Testlerin doğruladığı güvenlik özellikleri

- **`test_exec_security.py`:** güvenilir çalıştırılabilir/ACL/symlink/`PATH` kuralları, ayrıcalıklı komutların gömülü kullanıcı adı içermemesi.
- **`test_core.py`:** profil migrasyonu, symlink takibi yokluğu, sanitizasyon, atomik kayıt/yedek, PID kimliği, içe aktarma sınırları, udev kapsamı, runtime dizini özel olması, günlük symlink reddi, daemon bulmanın `PATH` kullanmaması.
- **`test_installer.py`:** dağıtım algılama, sürüm kapıları, masaüstü girdisi, kısayol güvenliği, root çalıştırma reddi, CI/release sertleştirmesi.
- **`test_runtime_files.py`, `test_uninstall.py`, `test_assets.py`, `test_widgets.py`:** dosya yaşam döngüsü, kaldırma kapsamı, varlıklar, arayüz bileşenleri.

### 5.4 Belgelenen sınırlamalar

- **`udev uaccess` izni kullanıcı genelindedir, uygulama başına değildir.** Oturum açıkken kullanıcı hesabındaki her süreç tüm `input/event*` aygıtlarını okuyabilir ve `/dev/uinput`'a yazabilir. Uygulama başına yalıtım udev ile ifade edilemez; ayrıcalıklı bir aracı veya sistem servisi gerekir.
- Motor, oturum sahibiyle aynı kullanıcıda çalışır; aynı hesaba erişen saldırgan zaten oturumun tüm yetkilerine sahiptir.
- Profil kullanıcı tarafından elle düzenlenebilir; yükleyici bozuk değerleri temizler, ancak makro davranışını kullanıcı belirler.
- `uinput` ile gönderilen tuşlar sistem genelinde gerçek girdi sayılır.

### 5.5 Değerlendirme

Linux sürümünün güvenlik mühendisliği **güçlüdür**: ayrıcalık yükseltmesi yalnızca kullanıcı onaylı udev kurulumunda kullanılır; yardımcı komutlar sabit ve `PATH`'sizdir; dosya yazımları atomik ve özel izinlidir; PID kimliği doğrulanır; içe aktarma sınırları ve kimlik yenileme vardır; testler bu davranışların çoğunu doğrular.

Ancak iki önemli not vardır:

1. **Belgelenmiş sınırlar uygulamanın değil, işletim sisteminin sınırlarıdır.** udev `uaccess` kullanıcı genelindedir; bu, Linux sürümünün tehdit modelinde açıkça kabul edilmiştir.
2. **Yerel kopyada CI iş akışları yoktur.** Sürüm bütünlüğü iddiaları (imzalı/attestation'lı arşiv) upstream iş akışlarına dayanır; bu kopyada doğrulanamamıştır.

Windows sürümünde bu kontrollerin eşdeğerleri (kullanıcıya özel ACL, atomik `MoveFileEx`/`ReplaceFile`, SID kısıtlı adlandırılmış kanal, imza doğrulama, DLL arama sırası sertleştirmesi) uygulanacaktır.

### 5.6 Windows'a özgü yeni saldırı yüzeyi

- Global düşük seviye kancalar **her tuşu görebilir** (parolalar dahil). Kanca yalnızca kullanıcı oturumunda çalışır; hassas tuş içerikleri loglanmayacaktır.
- Adlandırılmış kanal yerel IPC yüzeyidir; SID ACL, `PIPE_REJECT_REMOTE_CLIENTS`, sabit komut kümesi ve uzunluk/zaman aşımı sınırları gerekir.
- DLL arama sırası ve PyInstaller paket bütünlüğü; `SetDefaultDllDirectories` + tam yol `LoadLibraryEx` ile sertleştirilecektir.
- SmartScreen/antivirüs itibarı; imzalama ve temiz paketleme ile azaltılacak, garanti edilmeyecektir.

---

## 6. Windows Fizibilite Değerlendirmesi

### 6.1 Bileşen bazında matris

| Linux bileşeni | Windows karşılığı | Fizibilite | Başarı olasılığı* | Not |
|---|---|---|---|---|
| `evdev` ile tuş/fare okuma | `WH_KEYBOARD_LL` + `WH_MOUSE_LL` | **Mümkün** | ~%95 | Kanca iş parçacığında mesaj döngüsü; callback hızlı olmalı; kendi enjeksiyonunu `dwExtraInfo` imzasıyla yok say |
| `uinput` ile enjeksiyon | `SendInput` (tarama kodu/VK + fare) | **Mümkün** | ~%95 | UIPI: yükseltilmiş hedeflere enjeksiyon engellenir |
| Sanal aygıt kimliği | Yok; `LLKHF_INJECTED`/`LLMHF_INJECTED` | **Kısmi** | — | Gerçek HID için imzalı sürücü gerekir; kapsam dışı |
| `inotify` aygıt izleme | Kanca yaşam döngüsü; gerekirse `WM_INPUT_DEVICE_CHANGE` | **Mümkün** | ~%98 | Aygıt listesi yönetimi yok |
| X11/KWin imleç konumu | `GetCursorPos` / `SetCursorPos` | **Mümkün (daha basit)** | ~%99 | Çoklu monitör + DPI farkındalığı gerekir |
| KWin betiği/D-Bus | Gerek yok | **Mümkün** | ~%100 | `kwin.py` ve `protocols/` Windows'ta kullanılmaz |
| `SIGUSR1` profil yenileme | Kanal komutu `reload` | **Mümkün** | ~%98 | Alternatif: motoru yeniden başlatma |
| `SIGTERM`/`PR_SET_PDEATHSIG` | Kanal `stop` + Job Object `KILL_ON_JOB_CLOSE` | **Mümkün** | ~%95 | Takılı tuşların bırakılması için nazik kapanış |
| `flock` tek örnek | Adlandırılmış mutex (`Local\MacroStat...`) | **Mümkün** | ~%99 | Kullanıcı SID'i ile ad alanı |
| `udev`/`pkexec`/`modprobe` | Gerek yok | **Mümkün (büyük sadeleşme)** | ~%100 | Windows'ta aygıt izni sorunu yok |
| `TEXT` US-ASCII | `SendInput` + `KEYEVENTF_UNICODE` | **İyileşme** | ~%99 | Unicode (Türkçe karakterler, emoji) desteklenir |
| PID kimliği (`/proc`) | `OpenProcess` + `GetProcessTimes` + `QueryFullProcessImageNameW` + token SID | **Mümkün** | ~%95 | ctypes ile; ek bağımlılık yok |
| Günlük/kilit/pid yolları | `%APPDATA%\MacroStat`, `%LOCALAPPDATA%\MacroStat` | **Mümkün** | ~%99 | `QStandardPaths` veya `SHGetKnownFolderPath` |
| POSIX ACL doğrulaması | Windows DACL/SID doğrulaması | **Uyarlama** | ~%90 | Kurulum dizini kullanıcıya ait; SYSTEM/Administrators erişimi normal |
| PyInstaller | Windows'ta PyInstaller | **Mümkün** | ~%99 | Linux'tan Windows EXE **cross-build edilemez** |
| Rust MSVC derlemesi | `cargo build --target x86_64-pc-windows-msvc` | **Mümkün** | ~%98 | Linux'ta `cargo-xwin` ile cross-compile mümkün; asıl doğrulama Windows'ta |
| Kurulum betikleri | Inno Setup `.iss` | **Mümkün** | ~%99 | Kullanıcı bazlı kurulum yönetici istemez |
| Masaüstü/menü kısayolu | Inno Setup `[Icons]` | **Mümkün** | ~%99 | Opsiyonel `HKCU\...\Run` ile otomatik başlatma |
| Otomatik güncelleme | Yok (proje çevrimdışı) | **Kapsam dışı** | — | Ağ bağlantısı eklenmeyecek |
| Yönetici hedef uygulamalar | UIPI aşımı | **İmkânsız (normal yetkiyle)** | %0 | Çözüm: yönetici olarak çalıştırma veya imzalı `uiAccess` |
| UAC/kilit ekranı (güvenli masaüstü) | Kanca/enjeksiyon | **İmkânsız** | %0 | Çekirdek sürücü/servis olmadan erişilemez |
| Kernel anti-cheat oyunları | Kanca/enjeksiyon | **Belirsiz** | ~%30–60 | Tespit/atlatma geliştirilmeyecek; kullanıcı uyarılacak |
| Windows servisi olarak motor | Session 0 yalıtımı | **Önerilmez** | — | Etkileşimli masaüstüne enjeksiyon için uygun değil |

\* Olasılıklar, bu rapordaki mimariyle Windows 10/11 x64 üzerinde işlevsel başarı tahminidir; garanti değildir. En yüksek belirsizlik girdi backend'i ve anti-cheat/AV davranışındadır.

### 6.2 İmkânlar

1. **Tam özellik paritesi (normal uygulamalar):** tüm tetikleme kipleri, eylemler, profiller, içe/dışa aktarma, TR/EN arayüz, STOP/restart, canlı günlük.
2. **Daha basit izin modeli:** Windows'ta udev/pkexec/sudo/modprobe yok; kurulum yönetici izni istemez.
3. **Windows iyileştirmeleri:** sistem geneli klavye kaydı, Unicode `TEXT`, PerMonitorV2 DPI, çoklu monitör desteği, koyu başlık çubuğu.
4. **Daha güvenilir imleç konumu:** `GetCursorPos` her zaman kullanılabilir; KWin/X11 koşulları yok.
5. **Profil taşınabilirliği:** aynı JSON şeması Linux ve Windows arasında çalışır.

### 6.3 İmkânsızlıklar (bu sürümde)

1. **Yönetici pencerelere enjeksiyon ve okuma:** UIPI nedeniyle normal yetkiyle mümkün değil. Sınır yalnızca enjeksiyonu değil **okumayı da** kapsar: yükseltilmiş bir pencere ön plandayken normal yetkili süreç o pencereye giden girdiyi gözetleyemez; dolayısıyla kayıt ve tetikleme de çalışmaz. Kullanıcı uygulamayı yönetici olarak çalıştırırsa kanca/enjeksiyon o oturumda çalışır, ancak bu varsayılan değildir ve güvenlik önerisi değildir. Ayrıntı: `reports/arastirma-01-windows-sinirlari-ve-alternatifler.md`.
2. **Güvenli masaüstü:** UAC istemi, kilit ekranı ve Ctrl+Alt+Del ekranı ayrı masaüstüdür; kullanıcı modu uygulaması erişemez.
3. **Gerçek sanal HID:** imzalı çekirdek sürücüsü gerektirir; proje kapsamında yok.
4. **Anti-cheat tespitini atlatma:** etik ve teknik olarak kapsam dışı; oyunlarda çalışmama olasılığı belgelenir.

### 6.4 Olasılıklar ve belirsizlikler

| Konu | Olasılık | Açıklama |
|---|---|---|
| Notepad/Explorer/Chrome'da temel tetikleme | Çok yüksek | Kanca + `SendInput` standart yollarla çalışır |
| Kendi enjeksiyonunu yok sayma | Yüksek | `dwExtraInfo` imzası + injected bayrağı; test edilecek |
| Uzun makrolarda kararlılık | Orta-yüksek | Kanca zaman aşımı, iş parçacığı ayrımı ve kuyruk tasarımına bağlı |
| TR-Q klavye düzeninde doğru tuşlar | Yüksek | Tarama kodu + VK haritası; E2E test gerekir |
| Çoklu monitör/DPI doğruluğu | Orta-yüksek | PerMonitorV2 + sanal masaüstü normalizasyonu |
| AV/SmartScreen false positive | Orta | İmza, onedir, UPX yok, Defender'a gönderim |
| Kernel anti-cheat oyunlarında çalışma | Düşük-orta | Tespit edilebilir; garanti yok |

### 6.5 Build ortamı gerçeği

- **Rust motor:** Linux'tan `cargo-xwin`/`xwin` ile MSVC hedefli cross-compile edilebilir; asıl doğrulama Windows'ta yapılmalıdır.
- **PyQt6 GUI EXE:** PyInstaller **cross-compilation desteklemez**. Windows EXE yalnızca Windows'ta üretilir. Seçenekler: (a) GitHub Actions `windows-latest` (önerilen), (b) yerel Windows VM/makine, (c) Wine üzerinde PyInstaller (desteklenmez, güvenilmez).
- **Inno Setup:** yalnızca Windows'ta (veya Wine'da kısmi) derlenir; CI'da `windows-latest` kullanılmalıdır.

### 6.6 Dış kaynak araştırması (9 Kasım 2026)

**Windows 10 yaşam döngüsü.** Windows 10 22H2 desteği **14 Ekim 2025**'te sona erdi; Ekim 2025 güvenlik güncellemesi son normal güncellemeydi. Ekim 2026 itibarıyla yalnızca ESU kanalından güncelleme alıyor. Kullanıcı hedefi Windows 10 22H2 + Windows 11 olduğundan:
- Windows 11 hedefi güncel destek penceresindedir.
- Windows 10 testleri ESU'lu/ESU'suz sistemlerde yapılacak; rapor ve README "destek dışı işletim sistemi" uyarısını içerecektir.
- Üretim kullanımı için Windows 11 önerilir; Windows 10 uyumluluğu "çalışır" düzeyinde tutulur.

**PyQt6.** PyQt6 6.12.0, Python ≥3.10 için `win_amd64` ve `win_arm64` tekerlekleri yayınlıyor; Windows birinci sınıf desteklidir. Bu sürüm x64 hedefler; ARM64 kapsam dışıdır.

**Kod imzalama — Azure Artifact Signing (eski adı Trusted Signing).**
- Public Trust sertifikaları yalnızca şu ülkelerdeki **kuruluşlara** verilir: ABD, Kanada, Avrupa Birliği, Birleşik Krallık, Avustralya, Yeni Zelanda, Japonya, Güney Kore, Singapur, İsviçre, Norveç, İsrail. **Bireysel geliştiriciler yalnızca ABD veya Kanada'da** olabilir. **Türkiye bu listede yoktur.**
- Ücretli Azure aboneliği + Microsoft Entra tenant zorunludur; ücretsiz/deneme/sponsor abonelik kabul edilmez.
- Kotalar: **Basic 5.000 imza/ay**, **Premium 100.000 imza/ay**. Taban ve aşım ücretleri Microsoft fiyatlandırma sayfasında ülkeye göre dinamik gösterilir; sabit rakam yayınlanmaz. Faturalama oransal değildir.
- Kimlik doğrulama yalnızca Azure portalinden yapılır; **1–20 iş günü** sürebilir. Bireysel doğrulamada devlet kimliği + Microsoft Authenticator/Verified ID (AU10TIX) gerekir.
- Sertifikalar kısa ömürlüdür ve hizmette saklanır; imzalama `signtool` + dlib/metadata.json ile yapılır; GitHub Actions entegrasyonu vardır; zaman damgası `timestamp.acs.microsoft.com`.
- **EV sertifikası verilmez** ve gelecekte de planlanmamaktadır. SmartScreen itibarı indirme geçmişiyle otomatik oluşur; imza tek başına uyarıyı garanti etmez.
- **Türkiye merkezli geliştirici için Public Trust yolu uygun değildir.**

**Kod imzalama — geleneksel OV/EV Authenticode.**
- CA/Browser Forum Code Signing Baseline Requirements gereği özel anahtar FIPS 140-2 Level 2 veya üstü donanımda (USB belirteç/HSM) tutulmak zorundadır; dosya tabanlı `.pfx` ile imzalama uygun değildir.
- CI'da donanım belirteci sorun olduğundan bulut HSM/uzak imzalama (DigiCert KeyLocker, Sectigo Remote Signing, GlobalSign VSIGN vb.) gerekir.
- OV/EV sertifikaları Türkiye'deki tüzel kişilere/kuruluşlara verilebilir (CA'nın ülke politikasına bağlı). EV tarihsel olarak daha hızlı SmartScreen itibarı sağlasa da Microsoft bunu garanti etmez.
- Piyasa aralığı (araştırma tahmini): OV ~200–500 USD/yıl; EV ~300–700 USD/yıl; bulut HSM/uzak imzalama aboneliği ayrıca. Kesin teklif CA'dan alınmalıdır.

**Öneri.** İlk sürüm kullanıcı kararıyla **imzasız** yayınlanır. Faz 0'da (a) uygun ülkede tüzel kişilik varsa Azure Artifact Signing Basic, (b) aksi halde OV + bulut HSM/uzak imzalama teklifleri karşılaştırılır. İmza altyapısı `signtool` + RFC 3161 zaman damgası + CI secret olacak şekilde sonradan eklenebilir tasarlanır.

---

## 7. Önerilen Windows Mimarisi

### 7.1 Süreç modeli

```
MacroStat.exe (PyQt6 GUI)
   │  başlatır / kontrol eder
   ├── macrostat-engine.exe  (normal mod; profil yolu argümanı)
   ├── macrostat-engine.exe --test-macro <id>
   ├── macrostat-engine.exe --record-input <dosya>      (yeni: sistem geneli klavye+fare)
   └── macrostat-engine.exe --watch-restart <KEY_*>
   │
   └── Adlandırılmış kanal: \\.\pipe\macrostat-<SID>-<token>
         komutlar: ping | reload | stop | status
```

- **GUI:** mevcut PyQt6 kodu; `windows/gui/` altında platform katmanı ayrıştırılmış kopya.
- **Motor:** `windows/engine/` altında Rust; Linux motorunun iş mantığı korunur, Linux G/Ç katmanı Windows'a yazılır.
- **Yaşam döngüsü:** GUI motoru `subprocess`/`CreateProcessW` ile başlatır ve bir **Job Object**'e atar (`KILL_ON_JOB_CLOSE`). Nazik kapanışta önce kanaldan `stop`, sonra gerekirse süreç sonlandırma.
- **Tek örnek:** motor `Local\MacroStat.Engine.<SID>` mutex'i; GUI `QLockFile` veya `Local\MacroStat.Gui.<SID>` mutex'i.

### 7.2 IPC ve kimlik doğrulama

- Kanal adı kullanıcı SID'i + rastgele token içerir; token `%LOCALAPPDATA%\MacroStat\runtime.json` içinde yalnızca kullanıcıya okunur ACL ile saklanır.
- Sunucu `PIPE_REJECT_REMOTE_CLIENTS` kullanır; güvenlik tanımlayıcısı yalnızca geçerli kullanıcı SID'ine erişim verir.
- Mesajlar satır tabanlı JSON (`{"cmd":"reload"}` / `{"ok":true,...}`); maksimum uzunluk ve zaman aşımı sınırlıdır.
- PID dosyası JSON: `{pid, creationTime, exePath, sid, pipeName, logPath}`; GUI her işlemden önce kimliği doğrular.
- Kanal açılamazsa geri dönüş: süreci sonlandır + yeniden başlat (Linux davranışıyla eşdeğer).

### 7.3 Motor (Rust) modülleri

| Modül | Sorumluluk |
|---|---|
| `main.rs` | CLI, mod seçimi, yaşam döngüsü, console handler, tek örnek |
| `profile.rs` | Profil şeması, çoklu profil sarmalayıcı, sınırlar (`serde`) — Linux'tan taşınır |
| `keymap.rs` | `KEY_*`/`BTN_*` → Windows VK/tarama kodu/fare düğmesi; sol/sağ eşdeğerlik; desteklenmeyen tuş uyarısı |
| `trigger.rs` | Tetikleyici derleme, mandal, double-tap, hold, press-release durum makinesi — Linux mantığı |
| `actions.rs` | Eylem yürütme, `ACTION_LOCK`, jitter, iptal bayrağı, takılı tuş bırakma |
| `hook.rs` | `WH_KEYBOARD_LL` + `WH_MOUSE_LL`, mesaj döngüsü, olay kuyruğu, enjekte olay filtresi |
| `inject.rs` | `SendInput` klavye/fare/metin; `dwExtraInfo` imzası; bütünlük düzeyi önsezisi (UIPI engeli `GetLastError` ile algılanamaz) |
| `mouse.rs` | `MOUSE_PATH` kayıt/oynatma; `GetCursorPos`/`SetCursorPos`; sanal masaüstü normalizasyonu |
| `ipc.rs` | Adlandırılmış kanal sunucusu, JSON protokolü, ACL |
| `log.rs` | Dönüşümlü günlük (1 MB), kullanıcıya özel ACL, hata/uyarı biçimi |
| `winutil.rs` | DPI farkındalığı, sürüm bilgisi, SID, Job Object, tek örnek, yol doğrulama |
| `record.rs` | `--record-input`: sistem geneli klavye+fare kaydı (yeni yetenek) |

### 7.4 Kritik teknik kararlar

1. **Enjekte olay geri beslemesi:** tüm `SendInput` çağrılarına sabit `dwExtraInfo` imzası (`0x4D535441` = "MSTA") eklenir; kancalar bu imzayı taşıyan olayları yok sayar. `LLKHF_INJECTED`/`LLMHF_INJECTED` bayrağı loglanır.
2. **Kanca güvenliği:** callback'te yalnızca kuyruğa yazma; tetikleyici eşleme ve eylem yürütme ayrı iş parçacıklarında. `LowLevelHooksTimeout` aşımına yol açacak bloklayıcı iş yapılmaz. Kanca düşerse motor durumu GUI'ye bildirir.
3. **Enjeksiyon yöntemi:** klavye için `KEYEVENTF_SCANCODE` + gerekli `KEYEVENTF_EXTENDEDKEY`; fare için `MOUSEEVENTF_*`; metin için `KEYEVENTF_UNICODE` (UTF-16, vekil çiftler).
4. **Fare yolu:** kayıtta `WH_MOUSE_LL` ekran koordinatlarından delta üretimi (4 ms birleştirme); oynatmada hızlanma etkisini önlemek için sanal masaüstüne normalize edilmiş **mutlak** hareket (`MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | MOUSEEVENTF_MOVE_NOCOALESCE`) veya sıralı göreli hareket. İlk sürümde mutlak + `SetCursorPos` başlangıç hizalaması; saha testine göre göreli mod seçeneği.
5. **DPI:** uygulama ve motor `PER_MONITOR_AWARE_V2`; koordinatlar fiziksel piksel; çoklu monitörde negatif koordinatlar desteklenir.
6. **UIPI farkındalığı:** Microsoft'a göre UIPI engeli ne dönüş değerinden ne de `GetLastError`'dan anlaşılır. Bu nedenle enjeksiyon öncesi ön plandaki pencerenin bütünlük düzeyi sezilir (`GetForegroundWindow` → süreç → `TokenIntegrityLevel`); yükseltilmiş hedefte GUI rozeti ve günlük uyarısı üretilir. `SendInput` dönüş sayısı yalnızca genel "kısmen iletilemedi" sinyali olarak kullanılır; UIPI'ye özel teşhis iddia edilmez.
7. **Takılı tuş güvenliği:** `stop`, kanca kaldırma, GUI kapanışı ve beklenmeyen çıkışta `pressed` kümesindeki tüm tuşlar `SendInput` ile bırakılır; en iyi çaba + günlük.
8. **Tuş tekrarı (auto-repeat):** Windows basılı tutulan tuş için tekrar `WM_KEYDOWN` üretir ve `KBDLLHOOKSTRUCT`'ta Linux'taki `value=2` ayrımı yoktur. Motor kendi "basılı tuşlar" kümesini tutar; kümede zaten olan tuşun yeni basışını tekrar sayar ve tetikleyiciyi yeniden ateşlemez. Kanca sonradan kurulursa başlangıç durumu `GetAsyncKeyState` ile tohumlanır.
9. **Ebeveyn izleme:** Linux'taki `PR_SET_PDEATHSIG`/`getppid()==1` karşılığı olarak motor `--parent-pid <pid>` alır; `OpenProcess(SYNCHRONIZE)` ile ebeveyni izleyen bir iş parçacığı, GUI çökerse basılı tuşları bırakıp temiz çıkar. Job Object yalnızca son çare backstop'tur.
10. **Kayıt modu:** `--record-input` hem klavyeyi hem fareyi kancalarla kaydeder (Linux'taki pencere içi sınırını kaldırır); çıktı JSON, `MOUSE_PATH` + düğme olayları + tuş olayları. Pause/STOP kaydı bitirir; 10 dk/200.000 nokta sınırı korunur. Yükseltilmiş pencere ön plandayken kanca olay almadığından kayıt da durur; bu durum rozetle bildirilir.
11. **Restart watcher:** `--watch-restart <KEY_*>` yalnızca kanca + mesaj döngüsü çalıştırır; tuşa basılınca `42` ile çıkar (Linux ile aynı sözleşme).
12. **Test modu:** `--test-macro <id>` profili okur, tek makroyu çalıştırır, kilit/tek örnek kullanmaz; Linux davranışı korunur.

### 7.5 GUI taşıma stratejisi

**İlke:** platformdan bağımsız modülleri olduğu gibi kullan; Linux'a özgü modülleri Windows karşılıklarıyla değiştir. `windows/gui/macrostat/` altında kopya oluşturulur; `linux/` değişmez.

| Modül | İşlem |
|---|---|
| `sanitize.py`, `transfer.py`, `i18n.py`, `theme.py`, `widgets.py`, `logpanel.py`, `nav.py`, `macros.py`, `editor.py`, `core.py`, `onboarding.py` | Ağırlıklı olarak yeniden kullanılır; yalnızca yol/etiket/OS koşulları uyarlanır |
| `common.py` | Linux'a özgü yarısı (`/proc`, `fcntl`, `os.getuid`, XDG, POSIX ACL, pkexec, udev, `O_NOFOLLOW`) Windows katmanına ayrılır; atomik yazma/sanitizasyon yardımcıları korunur |
| `system.py` | `fix_permissions`/udev kaldırılır; motor kontrolü kanal + Job Object + PID kimliği; sistem durumu: motor, kanal, kanca, günlük, Windows sürümü, UIPI/AV notu |
| `recorder.py` | Sistem geneli kayıt motorun `--record-input` moduna devredilir; Qt içi kayıt yedek olarak kalır; Wayland bandı kaldırılır |
| `kwin.py` | Windows'ta dahil edilmez; imleç konumu `GetCursorPos` |
| `app.py` | Tek örnek: `QLockFile`/mutex; DPI; Qt koyu başlık çubuğu; yüksek DPI |
| `settings.py` | `%APPDATA%\MacroStat\settings.json`; `QStandardPaths` |
| `onboarding.py` | udev adımı yerine "Motoru Başlat + Kanca Testi + SmartScreen/AV bilgisi"; yönetici izni istenmez |
| `theme.py` | Qt standart ikonları + yerleşik SVG; koyu/açık sistem teması |
| `i18n.py` | Windows'a özgü yeni anahtarlar TR+EN; mevcut çeviri bütünlüğü testi korunur |

**Dizinler:** profil `%APPDATA%\MacroStat\profile.json`; ayarlar `%APPDATA%\MacroStat\settings.json`; günlük/PID/kanal token'ı `%LOCALAPPDATA%\MacroStat\`; yedekler aynı dizinde; içe/dışa aktarma kullanıcı seçtiği yol.

**Windows'a özgü iyileştirmeler (parite üstü):**
- Sistem geneli klavye kaydı (Linux'ta yalnızca pencere içi).
- Unicode `TEXT` desteği.
- System Tray simgesi + "Arka planda çalış" (varsayılan kapalı).
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` ile otomatik başlatma (varsayılan kapalı, ayarlardan açılır).
- Yüksek DPI ve çoklu monitör iyileştirmeleri.
- Windows koyu başlık çubuğu (`DwmSetWindowAttribute`).

### 7.6 `windows/` proje yerleşimi

```
windows/
  README.md                      # Windows sürümü hızlı başlangıç + mimari
  REPORT.md                      # Bu rapor
  reports/
    00-baseline.md               # envanter + kararlar
    01-engine-core.md
    02-input-backend.md
    03-ipc-lifecycle.md
    04-gui-port.md
    05-packaging.md
    06-security-review.md
    07-e2e-test.md
    08-final-report.md
  engine/
    Cargo.toml
    build.rs                     # ikon/sürüm kaynağı, manifest
    src/{main,profile,keymap,trigger,actions,hook,inject,mouse,ipc,log,winutil,record}.rs
    tests/
  gui/
    gui.py
    macrostat/
    assets/
    requirements-win.txt
    MacroStat.spec               # PyInstaller
    version_info.txt
  installer/
    MacroStat.iss                # Inno Setup
    LICENSE
    THIRD_PARTY.md
  scripts/
    build.ps1
    sign.ps1
    verify.ps1
  docs/
    WINDOWS-ARCHITECTURE.md
    SECURITY-WINDOWS.md
    COMPATIBILITY.md
    TEST-MATRIX.md
  tests/
```

---

## 8. Paketleme, Dağıtım ve CI

### 8.1 Çıktılar

- **`MacroStat.exe` + `_internal/`** — PyInstaller onedir, `--windowed`, uygulama ikonu, sürüm bilgisi, manifest.
- **`macrostat-engine.exe`** — Rust release; `windows` crate; statik CRT; ikon/sürüm kaynağı `embed-resource`; manifest: `asInvoker`, `PerMonitorV2`, `longPathAware`, `supportedOS` Win10/11.
- **`MacroStat-Setup-1.0.0.exe`** — Inno Setup; kullanıcı bazlı `%LOCALAPPDATA%\Programs\MacroStat`, yönetici gerektirmez; isteğe bağlı "tüm kullanıcılar" modu.
- **`MacroStat-1.0.0-win-x64-portable.zip`** — iki EXE + LICENSE + README.
- Opsiyonel: `MacroStat-1.0.0-win-x64.msix` (imzalı olmadan kurulamaz; `uiAccess` yok).

### 8.2 PyInstaller notları

PyQt6 eklentileri (`platforms/qwindows.dll`, `styles`, `imageformats`, `iconengines`) açıkça toplanır; gereksiz Qt modülleri dışlanır; `--onefile` isteğe bağlıdır (başlangıç gecikmesi ve AV false-positive riski artar; onedir önerilir). Spec dosyası sürümlenir.

### 8.3 Kod imzalama

- İlk sürüm **imzasız** (kullanıcı kararı).
- Öncelik: (1) uygun ülkede tüzel kişilik varsa Azure Artifact Signing Basic (5.000 imza/ay; EV yok; Türkiye Public Trust kapsamı dışında), (2) OV sertifikası + bulut HSM/uzak imzalama (Türkiye'de uygulanabilir yol).
- Sertifika yoksa SmartScreen "Bilinmeyen yayıncı" uyarısı verir; README'de SHA-256 doğrulama ve kaynak bağlantısı sunulur.
- `uiAccess=true` yalnızca imzalı ve Program Files altında çalışan EXE'lerde işler; bu proje varsayılan olarak `uiAccess` kullanmaz.

### 8.4 GPL uyumu

Kaynak kod bağlantısı, `LICENSE` (GPL-3.0-or-later), `THIRD_PARTY.md`, PyQt6 GPL bildirimi kuruluma ve portable ZIP'e eklenir; Inno Setup lisans sayfası gösterilir.

### 8.5 CI

GitHub Actions `windows-2022`/`windows-2025`:

1. `cargo fmt --check`, `cargo clippy`, `cargo test`.
2. Python `unittest` (Qt offscreen).
3. PyInstaller build.
4. Inno Setup derleme.
5. SHA-256 üretimi.
6. Artifact yükleme; etiketli sürümde Sigstore attestation.
7. İmzalama secret'ları yalnızca korumalı ortamda; sertifika yoksa adım atlanır.

Linux CI korunur; Windows iş akışı ayrı dosyadır ve `linux/` ağacına dokunmaz.

---

## 9. Windows Güvenlik Modeli

- **En az yetki:** `asInvoker`; yönetici izni istenmez; sürücü yok; ağ yok.
- **Kod bütünlüğü:** motor EXE'si GUI'nin yanında sabit yolda; `PATH` araması yok; başlatmadan önce yol bileşenleri ve sahiplik/ACL doğrulanır; opsiyonel `WinVerifyTrust` imza kontrolü.
- **DLL arama sırası:** `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR)`; `LoadLibraryEx` ile tam yol; PyInstaller `_internal` dizini.
- **IPC:** yalnızca yerel, SID kısıtlı kanal; `PIPE_REJECT_REMOTE_CLIENTS`; uzunluk/zaman aşımı sınırları; sabit komut kümesi.
- **Dosya:** profil/ayar/günlük kullanıcıya özel ACL; atomik yazma (`MoveFileEx`/`ReplaceFile`); tahmin edilemez geçici adlar; reparse point/symlink takibi yok (varsa reddet).
- **Girdi:** kanca yalnızca kullanıcı oturumunda; enjekte olay imzası; UIPI sınırı açıkça belgelenir; güvenli masaüstünde çalışmaz.
- **Günlük:** kullanıcı tarafından okunabilir, başka kullanıcılar okuyamaz; hassas veri (tuş içerikleri) loglanmaz; yalnızca olay adları/hatalar.
- **Kurulum:** Inno Setup imzalıysa SmartScreen azalır; kurulum yolu kullanıcıya ait; `PrivilegesRequired=lowest`; kaldırma yalnızca kendi dosyalarını siler.
- **Tehdit modeli:** aynı kullanıcı hesabındaki kötü niyetli süreçler kancaları okuyabilir/enjekte edebilir; MacroStat ek bir sınır koymaz. Yükseltilmiş hedefler ve güvenli masaüstü kapsam dışıdır. Anti-cheat tespitini atlatma kesinlikle geliştirilmeyecektir.

---

## 10. Linux ↔ Windows Uyumluluğu

| Konu | Davranış |
|---|---|
| Profil JSON şeması | Aynen korunur; Linux'ta üretilen profiller Windows'ta açılır ve tersi |
| Tuş adları | `KEY_*` / `BTN_*` olarak kalır; motor düz adları da çözer |
| Karşılığı olmayan tuşlar | Yüklenirken uyarı verir, tetikleyici devre dışı kalır; profil bozulmaz |
| `virtualDeviceName` | Windows'ta kozmetiktir (aygıt oluşturulmaz); alan korunur |
| `TEXT` | Windows'ta Unicode'a genişler; Linux'a geri taşınan profilde ASCII dışı metin Linux'ta atlanır |
| `MOUSE_PATH` başlangıç konumu | Windows'ta her zaman okunabilir; Linux'ta X11/KDE Wayland koşullu |
| `restartKey`, `panicKey`, `defaultJitterMs`, `repeatCount`, `doubleTapTimeoutMs`, `holdDurationMs` | Aynı anlamda |
| Ayar dosyaları | İşletim sistemine özgü ayrı dizinler; profil taşınabilir |

---

## 11. Fazlar ve Kabul Ölçütleri

| Faz | Kapsam | Çıktı/Rapor | Kabul ölçütü |
|---|---|---|---|
| 0 | Baseline: Linux sözleşmesinin dondurulması, kararlar, build ortamı, imza araştırması | `reports/00-baseline.md` | Profil şeması/anahtar sözlüğü belgelenmiş; `windows/` iskeleti hazır |
| 1 | Motor çekirdeği: profil, keymap, trigger, actions (platformdan bağımsız) | `reports/01-engine-core.md` | Rust birim testleri yeşil; Linux mantığıyla eşdeğerlik tablosu |
| 2 | Windows girdi backend'i: kancalar, `SendInput`, STOP, restart watcher, takılı tuş temizliği | `reports/02-input-backend.md` | Notepad'de tetikleme/enjeksiyon; kendi enjeksiyonunu yok sayma; STOP çalışıyor |
| 3 | IPC/yaşam döngüsü/günlük/kayıt | `reports/03-ipc-lifecycle.md` | reload/stop/status; Job Object; `--record-input` çıktısı geçerli |
| 4 | GUI platform katmanı ve özellik paritesi | `reports/04-gui-port.md` | Tüm sayfalar çalışıyor; TR/EN; profil içe/dışa; onboarding |
| 5 | Paketleme: PyInstaller + Inno Setup + portable ZIP | `reports/05-packaging.md` | Temiz Windows VM'de kurulum/açılış/kaldırma |
| 6 | Güvenlik incelemesi ve tehdit modeli | `reports/06-security-review.md` | ACL/kanal/DLL/günlük kontrolleri; bağımlılık denetimi |
| 7 | E2E test matrisi (Win10/11, DPI, çoklu monitör, UIPI, AV) | `reports/07-e2e-test.md` | Matris sonuçları; bilinen sınırlar |
| 8 | Nihai rapor ve sürüm kontrol listesi | `reports/08-final-report.md` | Tüm EXE'ler üretilmiş; SHA-256; sürüm notları |

Her faz sonunda rapor **Türkçe** yazılır; komut çıktıları ve kararlar eklenir.

**Durum (11 Kasım 2026):** Faz 0–8 kaynak düzeyinde tamamlandı
(`reports/00-baseline.md` … `reports/08-final-report.md`). Windows üretimi, VM
kabulü ve imza için açık maddeler `reports/08-final-report.md` §4–5'te ve
`docs/CI-RUNBOOK.md`'de listelenmiştir.

---

## 12. Test Planı

- **Rust birim:** keymap (her `KEY_*`/`BTN_*` için VK/tarama eşlemesi, sol/sağ eşdeğerlik, desteklenmeyenler), tetikleyici derleme/mandal/double-tap/hold/press-release, profil ayrıştırma/sınırlar, IPC çerçeveleme, JSON şema.
- **Python birim:** mevcut 7 test dosyasının Windows uyarlaması (yol/ACL/PID/sinyal yerine kanal); `sanitize`, `transfer`, `i18n` bütünlüğü, `settings` atomikliği, tek örnek.
- **Entegrasyon:** `macrostat-engine.exe --self-test` (uygulandı): kanca kurulumu, `SendInput` imzası, kendi olayını yok sayma, dış enjeksiyonu görme, panik mandalı, takılı tuş temizliği; GUI'siz çalışır.
- **E2E matris:** Windows 10 22H2 x64 (ESU'lu/ESU'suz), Windows 11 x64; %100/%150/%200 DPI; tek/çoklu monitör (negatif koordinat); TR-Q ve US klavye düzeni; Notepad, Explorer, Chrome/Edge, yönetici CMD (UIPI negatif testi), UAC istemi (kanca yok testi), kilit ekranı, uyku/uyanma, RDP oturumu, hızlı kullanıcı değiştirme, Windows Defender.
- **CI:** `windows-latest` üzerinde fmt/clippy/test, Python testleri, build, installer, checksum; Linux CI korunur (regresyon yok).
- **Kabul:** profil uyumluluğu, TR/EN arayüz, STOP/restart, Unicode metin, sistem geneli kayıt, kurulum/kaldırma, imzasız SmartScreen davranışının belgelenmesi.

---

## 13. Riskler ve Önlemler

| Risk | Olasılık | Etki | Önlem |
|---|---|---|---|
| UIPI: yönetici pencerelere enjeksiyon yok | Yüksek | Orta | Belgelenir; "yönetici olarak çalıştır" seçeneği; `uiAccess` yol haritası (imza şartı) |
| Güvenli masaüstünde çalışmama (UAC/kilit) | Kesin | Düşük | Belgelenir; sürücü kapsam dışı |
| Antivirüs/SmartScreen false positive | Orta-yüksek | Yüksek | Kod imzalama, onedir, UPX yok, Defender'a gönderim, SHA-256, kaynak |
| Kanca zaman aşımı/otomatik kaldırma | Orta | Yüksek | Hızlı callback, kuyruk, worker thread, kanca durum izleme + yeniden kurma |
| Geri besleme döngüsü | Orta | Yüksek | `dwExtraInfo` imzası + injected bayrağı, test |
| Takılı tuşlar (çökme) | Orta | Yüksek | Job Object, kanal stop, en iyi çaba temizliği, `--self-test` |
| Klavye düzeni farkı | Orta | Orta | Tarama kodu + VK haritası; TR/EN testleri; `KEY_*` sözlüğü |
| DPI/çoklu monitör koordinatları | Orta | Orta | PerMonitorV2, sanal masaüstü normalizasyonu, matris testi |
| PyInstaller EXE boyutu/başlangıç | Orta | Düşük | onedir, gereksiz Qt modülleri dışlama, ölçüm |
| GPL/PyQt6 uyumu | Düşük | Orta | LICENSE/THIRD_PARTY, kaynak bağlantısı, lisans sayfası |
| Build ortamı yokluğu | Orta | Yüksek | GitHub Actions `windows-latest` + yerel VM; Wine kullanılmaz |
| Windows 10 desteğinin sona ermesi | Kesin | Orta | Windows 11 önerilir; Win10 "çalışır" düzeyinde; ESU notu |
| Kod imzalama uygunluğu (Türkiye) | Yüksek | Orta | OV + bulut HSM yolu; imzasız sürümde SHA-256 |
| Sürücü talebi/anti-cheat | Düşük | Yüksek | Kapsam dışı; tespit atlatma yok; belgeleme |

---

## 14. Efor Tahmini

| Faz | Saat |
|---|---|
| Faz 0–1 (sözleşme + çekirdek port + testler) | ~40–60 |
| Faz 2 (kanca/enjeksiyon/kenar durumları) | ~60–100 |
| Faz 3 (IPC/yaşam döngüsü/kayıt) | ~40–60 |
| Faz 4 (GUI platform katmanı + TR/EN + onboarding) | ~60–100 |
| Faz 5 (PyInstaller/Inno/CI) | ~30–50 |
| Faz 6–7 (güvenlik + E2E) | ~40–60 |
| **Toplam** | **~270–430 saat** |

Tek deneyimli geliştirici, Windows CI/VM erişimi ve normal öncelik varsayımıyla takvim **~7–11 hafta**.

---

## 15. Alınan Kararlar (kullanıcı onayı)

1. **Dağıtım biçimi:** Kurulum EXE'si + uygulama EXE'leri. Inno Setup ile `MacroStat-Setup-1.0.0.exe`; içinde `MacroStat.exe` + `macrostat-engine.exe`; ayrıca taşınabilir ZIP. Tek dosya (`--onefile`) varsayılan değil.
2. **Kod imzalama:** Sertifika henüz yok; sürüm imzasız üretilecek. Araştırma tamamlandı: Azure Artifact Signing Public Trust Türkiye'yi kapsamıyor ve EV vermiyor; Türkiye için gerçekçi yol OV + bulut HSM/uzak imzalama. İmza altyapısı sonradan eklenebilir tasarlanacak; Faz 0'da CA teklifleri karşılaştırılacak.
3. **Yönetici hedefler:** Normal uygulamalar yeterli. Varsayılan `asInvoker`; UIPI ve güvenli masaüstü sınırı belgelenecek. Kullanıcı isterse uygulamayı "Yönetici olarak çalıştır" ile açabilir; ayrı `uiAccess` yapılandırması bu sürümde yok.
4. **Derleme/test ortamı:** GitHub Actions `windows-latest` resmi derleme, test, PyInstaller ve Inno Setup için kullanılacak. Yerel Linux'ta Rust `cargo-xwin` cross-build ve statik analiz yapılabilir; PyInstaller/Inno Setup Linux'ta çalıştırılmayacak.
5. **Kapsam:** Parite + Windows iyileştirmeleri. Tüm Linux özellikleri korunur; ek olarak sistem geneli klavye kaydı, Unicode `TEXT`, PerMonitorV2 DPI, çoklu monitör iyileştirmeleri. Tray/otomatik başlatma opsiyonel ve varsayılan kapalı değerlendirilecek.
6. **Hedef:** Windows 10 22H2 (build 19045) + Windows 11, x64. ARM64 bu sürümün kapsamı dışında.

---

## 16. Açık İşler ve Sonraki Adım

### Faz 0'da netleştirilecekler

1. OV/EV Authenticode + bulut HSM/uzak imzalama için en az iki CA'dan teklif ve Türkiye uygunluğu.
2. Windows 10 22H2 test imajının ESU durumu; README uyarı metni.
3. `windows/` iskeletinin oluşturulması (README, reports, engine, gui, installer, scripts, docs, tests).
4. Profil şeması ve `KEY_*`/`BTN_*` sözlüğünün sözleşme dosyası olarak dondurulması.
5. GitHub Actions Windows workflow taslağı (Linux workflow'a dokunmadan).
6. PyInstaller spec ve Inno Setup `.iss` taslakları.
7. `--self-test` kabul senaryolarının yazılması.

### Uygulama sırası

Faz 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8. Her faz sonunda Türkçe rapor: `windows/reports/NN-*.md`.

### Değişmez kural

`linux/` ağacı **salt okunur** kalır. Windows'a ait her dosya, komut çıktısı ve rapor `windows/` altında üretilir.

---

## 17. Kaynak Referansları

- `linux/README.md` — ürün tanımı, motor davranışı, arayüz özellikleri, sürüm doğrulama.
- `linux/SECURITY.md` — tehdit modeli, dosya sistemi/yürütme varsayımları, bilinen sınırlamalar.
- `linux/src/main.rs` — motorun tüm davranışı (CLI, tetikleyici, eylemler, kayıt, kilit).
- `linux/src/kwin_input.rs`, `linux/protocols/fake-input.xml` — KDE Wayland imleç hizalama.
- `linux/macrostat/*.py` — GUI, profil, güvenlik yardımcıları.
- `linux/tests/*.py` — güvenlik ve davranış testleri.
- `linux/packaging/*`, `install_*.sh`, `PKGBUILD`, `uninstall.sh` — Linux kurulum/kaldırma.
- GitHub: https://github.com/Getz-E/Macro-Stat.git (main, 10 commit, GPL-3.0-or-later).
- Microsoft Learn: Windows 10 22H2 yaşam döngüsü; Azure Artifact Signing quickstart/FAQ/fiyatlandırma.
- PyPI/PyQt6 6.12.0 tekerlek bilgisi.
- Sınır araştırması: `SendInput`, `SendSAS`, `RAWINPUTDEVICE`, `timeBeginPeriod`, UAC, `uiAccess`, VHF belgeleri; AutoHotkey SSS; ViGEmBus/Interception depoları — özet ve sonuçlar `reports/arastirma-01-windows-sinirlari-ve-alternatifler.md`.

---

**Rapor sonu.** Bu belge inceleme, fizibilite ve planlama aşamasını kaydeder. Uygulama başladığında her fazın sonucu `windows/reports/` altında ayrıca raporlanacaktır.
