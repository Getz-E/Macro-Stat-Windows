# MacroStat for Windows

[English](#english) | [Türkçe](#türkçe)

**Status: Release Candidate (1.0.0).** MacroStat for Windows is the Windows port
of the MacroStat project: an offline macro engine written in Rust with a PyQt6
desktop interface. It is not a stable release yet; see
[Known limitations](#known-limitations).

<a id="english"></a>

## English

### Project overview

MacroStat runs user-defined keyboard and mouse macros system-wide. A Rust engine
(`macrostat-engine.exe`) reads input through low-level Windows hooks and injects
synthetic input with `SendInput`; a PyQt6 application edits profiles, records
input, and manages the engine process. Everything runs locally in the user
session: there is no network access, telemetry, or automatic update.

The profile format and key names are inherited from the Linux version of
MacroStat, so profiles can be moved between the two projects. The Linux project
is a separate repository and is not included here.

- License: GPL-3.0-or-later
- Version: 1.0.0 (release candidate)

### Features

- **7 trigger modes** — `ONCE`, `WHILE_HELD`, `TOGGLE_LOOP`, `REPEAT_COUNT`,
  `DOUBLE_TAP`, `HOLD_LONG_PRESS`, `PRESS_AND_RELEASE`; combo triggers are
  supported.
- **11 action types** — `KEY_DOWN`, `KEY_UP`, `KEY_TAP`, `MOUSE_DOWN`,
  `MOUSE_UP`, `MOUSE_CLICK`, `MOUSE_MOVE`, `MOUSE_SCROLL`, `MOUSE_PATH`,
  `TEXT`, `DELAY`.
- **System-wide input** — `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks; injected
  events are tagged and ignored by the engine's own hook.
- **Panic key** — stops the engine and releases held keys even when the hook
  queue is busy. The default is `KEY_PAUSE` and it is configurable per profile.
- **Single instance and local IPC** — a named pipe restricted to the current
  user SID, protected by a random 128-bit token and a first-instance flag;
  `ping`, `reload`, `status` and `stop` commands.
- **Profile safety** — bounded profile size, macro count, action count, text
  length and path points; atomic writes; reparse points (symlinks and
  junctions) are rejected.
- **Input recorder** — records keyboard and mouse input system-wide into a
  profile-compatible JSON file (`--record-input`).
- **Self-test** — `macrostat-engine.exe --self-test` checks the hook chain,
  injection signature isolation, external injection visibility, the panic latch
  and stuck-key cleanup without starting the GUI.
- **Turkish and English interface**, macro editor, live log panel, onboarding.
- **Offline by design** — no network, telemetry, updater or cloud service.

### Platform support

- Windows 11, **x64 only**, is the officially supported platform.
- Windows 10 22H2 (build 19045), x64, is for development and testing only; it is
  not an officially supported platform.
- ARM64, 32-bit Windows and Windows Server are out of scope.
- Windows 10 22H2 is past its regular support period; extended security update
  (ESU) availability depends on the installation environment.
- No CPU, RAM or disk requirements are stated: they have not been measured.
  The build-time requirements below are the only verified ones.

### Installation

There are two supported ways to obtain MacroStat. Neither is a stable public
release yet.

#### 1. Prebuilt CI artifacts

Each successful `build` job of the GitHub Actions workflow produces, as a
workflow artifact named `macrostat-win-x64` (30-day retention):

- `MacroStat-Setup-1.0.0.exe` — per-user installer
  (`%LOCALAPPDATA%\Programs\MacroStat`), no administrator rights required;
- `MacroStat-1.0.0-portable.zip` — portable package, no installation;
- `SHA256SUMS.txt` — SHA-256 checksums;
- `MacroStat/` — the raw PyInstaller output.

Artifacts can be downloaded only by users with access to this repository. The
repository is private, so these artifacts are not public download links.

No stable download page or GitHub Release has been published or verified for
this project, so no download link is given here. If you download an artifact,
verify it first:

```powershell
Get-FileHash .\MacroStat-Setup-1.0.0.exe -Algorithm SHA256
```

The build is currently **unsigned**; SmartScreen may warn about an unknown
publisher.

#### 2. From source (Windows)

Requirements:

- Windows 10 22H2 / Windows 11 x64;
- Rust 1.79 or newer with the MSVC toolchain
  (`stable-x86_64-pc-windows-msvc`);
- Python 3.10 or newer (CI uses 3.12);
- for installers only: Inno Setup 6.3 or newer, and the build requirements in
  `windows/gui/requirements-build.txt`.

From the repository root:

```powershell
cargo build --manifest-path windows/engine/Cargo.toml --release --locked
python -m pip install -r windows/gui/requirements-win.txt
python windows/gui/main.py
```

The engine binary must exist at
`windows/engine/target/release/macrostat-engine.exe`; the GUI locates it there.
The engine is a Windows binary; running the product on Linux or under Wine is
neither supported nor verified. On non-Windows systems only the
platform-independent Rust core can be built and tested with `cargo test`.

To build the installer and portable ZIP, see
[`windows/scripts/README.md`](windows/scripts/README.md) and
[`windows/README.md`](windows/README.md).

### Quick start

1. Start MacroStat (Start menu shortcut after installation, `MacroStat.exe` from
   the portable package, or `python windows/gui/main.py` from source).
2. The first-run onboarding checks the environment and confirms that the engine
   can be started. No administrator permission is requested.
3. Open the macro editor, create a macro, choose a trigger key and mode, and add
   actions.
4. Save the profile; the engine reloads it. The status bar shows whether the
   engine and hook are running.
5. Press the panic key (default `KEY_PAUSE`) to stop the engine and release all
   held keys.

### Usage

- **Macros page** — list, enable/disable, and test macros. A test run executes a
  single macro without taking the single-instance lock
  (`macrostat-engine.exe --test-macro <id>`).
- **Macro editor** — trigger key and mode, action list, delays, mouse paths and
  text actions. Profiles are JSON files validated and sanitized on load.
- **Recorder** — records system-wide input through the engine and converts it
  into a profile-compatible recording.
- **System & guide page** — engine status, diagnostics, log panel, backup and
  onboarding.
- **Diagnostics** — run
  `windows\dist\MacroStat\_internal\macrostat-engine.exe --self-test`
  to test the hook and injection chain without the GUI. Exit code `0` means all
  checks passed, `4` at least one check failed, `5` not running on Windows.

Detailed Windows engineering notes are in [`windows/README.md`](windows/README.md).

### Uninstallation

- **Installer build** — use *Apps & features* or the Start menu uninstaller.
  It removes only the files it installed. It then asks whether to delete
  `%LOCALAPPDATA%\MacroStat` (profiles, logs, `runtime.json`) and
  `%APPDATA%\MacroStat` (settings); the default answer is **No**. Note that this
  prompt is currently shown in English even in the Turkish installer.
- **Portable build** — stop MacroStat and delete the extracted folder. Delete
  `%LOCALAPPDATA%\MacroStat` and `%APPDATA%\MacroStat` manually if you also want
  to remove your data.
- **Run from source** — stop MacroStat and delete the cloned repository.

### Security

- Both processes run as the invoking user (`asInvoker`); no administrator
  elevation, service, kernel driver, `uiAccess` or anti-cheat bypass is used.
- The IPC named pipe is local-only, restricted to the current user SID, rejects
  remote clients, uses a random 128-bit token, and is created with a
  first-instance flag. `runtime.json` is written atomically.
- Injected input carries an engine signature; the engine ignores its own events.
- Profiles are untrusted input and are bounded and sanitized before use.
- **Documented limits:** injection into administrator windows is blocked by UIPI,
  and the hook does not receive events on the secure desktop (UAC prompt, lock
  screen). In both cases macros do not run; the interface reports this instead of
  failing silently.
- Logs do not contain typed key content.
- Reporting policy: [`SECURITY.md`](SECURITY.md) (English) and
  [`SECURITY.tr.md`](SECURITY.tr.md) (Turkish). GitHub Private Vulnerability
  Reporting / Security Advisories is **not verified to be enabled** for this
  repository, and the **Report a vulnerability** option can be used only if a
  maintainer has enabled it. If it is not available, open a normal issue that
  contains no vulnerability details and ask the maintainers for a private
  channel; do not share technical details, proof-of-concept code, or other
  sensitive information until a private channel is provided.
- The Windows threat model is in
  [`windows/docs/SECURITY-WINDOWS.md`](windows/docs/SECURITY-WINDOWS.md).

### Testing

Verified locally (2026-11-14) on Linux unless noted:

| Suite | Command | Result |
|---|---|---|
| Rust engine | `cargo test --all` (from `windows/engine`) | 99 passed |
| GUI | `QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -p "test_gui*.py" -v` | 55 passed |
| Windows E2E | `python -m unittest discover -s windows/tests -p "test_e2e*.py" -v` | Windows only; 7 test methods cover the 9 acceptance scenarios U-01…U-09 |

The E2E file also contains 5 platform-independent regression tests for UTF-8
output decoding and subprocess pipe handling. On non-Windows systems the 7 E2E
methods are skipped; a skip is not a pass.

Latest verified GitHub Actions run (2026-11-14, run 43638328015): `Dependency
audit`, `Engine core (fmt / clippy / test)`, `GUI unit tests`, `Build installer +
portable ZIP` and `E2E acceptance (real engine)` succeeded; `Sign artifacts
(optional)` was skipped because no signing certificate is configured. VM
acceptance (installation and interaction on a clean Windows VM) has not been
performed.

### Known limitations

- **Release candidate, not a stable release.**
- **Unsigned binaries.** SmartScreen may warn; signing infrastructure exists but
  no certificate is configured.
- **UIPI.** Macros do not reach windows running elevated.
- **Secure desktop.** No macros on UAC prompts or the lock screen.
- **No VM acceptance test yet.** Behavior was verified by unit, GUI and CI E2E
  runs, not on a clean interactive Windows VM.
- **Not implemented:** automatic start with Windows, system tray integration and
  automatic profile-file watching. These appear in older planning documents but
  are not present in the code.
- **Installer localization gap.** The uninstall data-deletion prompt is English
  in both language variants.
- **ARM64 is out of scope.**
- **Windows 10 22H2** is past regular support; ESU status depends on the system.

### Contributing

Development setup, formatting, test commands, CI expectations and repository rules
are described in [`CONTRIBUTING.md`](CONTRIBUTING.md) (English) and
[`CONTRIBUTING.tr.md`](CONTRIBUTING.tr.md) (Turkish).

### License

MacroStat is licensed under **GPL-3.0-or-later**. The full license text is in
[`LICENSE`](LICENSE); the copy used by the installer is
[`windows/LICENSE`](windows/LICENSE), which is byte-identical. Copyright (C)
2026 MacroStat contributors.

Third-party components and their licenses are listed in
[`windows/THIRD_PARTY.md`](windows/THIRD_PARTY.md).

### Bug reports and support

- **General bugs and feature requests:** [GitHub Issues](../../issues). Search
  existing issues first and include the MacroStat version, Windows version,
  architecture, and reproduction steps.
- **Security vulnerabilities:** do **not** open a public issue or post any
  details. Follow [`SECURITY.md`](SECURITY.md): if the private reporting option
  is not available, ask the maintainers for a private channel without disclosing
  technical information.
- No response-time or support guarantee is provided.

[Back to top](#macrostat-for-windows)

---

<a id="türkçe"></a>

## Türkçe

### Proje hakkında

MacroStat, kullanıcı tanımlı klavye ve fare makrolarını sistem genelinde
çalıştırır. Rust ile yazılmış motor (`macrostat-engine.exe`) girdiyi Windows'un
düşük seviyeli kancalarıyla okur ve `SendInput` ile sentetik girdi enjekte eder;
PyQt6 arayüzü profilleri düzenler, girdi kaydeder ve motor sürecini yönetir. Her
şey kullanıcı oturumunda ve yerel olarak çalışır: ağ erişimi, telemetri veya
otomatik güncelleme yoktur.

Profil biçimi ve anahtar adları MacroStat'ın Linux sürümünden devralınmıştır; bu
sayede profiller iki proje arasında taşınabilir. Linux projesi ayrı bir depodur ve
burada yer almaz.

- Lisans: GPL-3.0-or-later
- Sürüm: 1.0.0 (sürüm adayı)

### Özellikler

- **7 tetikleme kipi** — `ONCE`, `WHILE_HELD`, `TOGGLE_LOOP`, `REPEAT_COUNT`,
  `DOUBLE_TAP`, `HOLD_LONG_PRESS`, `PRESS_AND_RELEASE`; kombo tetikleyiciler
  desteklenir.
- **11 eylem türü** — `KEY_DOWN`, `KEY_UP`, `KEY_TAP`, `MOUSE_DOWN`,
  `MOUSE_UP`, `MOUSE_CLICK`, `MOUSE_MOVE`, `MOUSE_SCROLL`, `MOUSE_PATH`,
  `TEXT`, `DELAY`.
- **Sistem geneli girdi** — `WH_KEYBOARD_LL` ve `WH_MOUSE_LL` kancaları; motorun
  enjekte ettiği olaylar imzalanır ve kendi kancası tarafından yok sayılır.
- **Panik tuşu** — kanca kuyruğu dolu olsa bile motoru durdurur ve basılı tuşları
  bırakır. Varsayılan `KEY_PAUSE` olup profil başına değiştirilebilir.
- **Tek örnek ve yerel IPC** — kullanıcı SID'iyle sınırlı adlandırılmış kanal,
  rastgele 128 bit token ve ilk örnek bayrağı; `ping`, `reload`, `status`,
  `stop` komutları.
- **Profil güvenliği** — profil boyutu, makro ve eylem sayısı, metin uzunluğu ve
  yol noktası sayısı sınırlandırılır; atomik yazma; sembolik bağ ve junction
  (reparse point) reddedilir.
- **Girdi kaydedici** — sistem geneli klavye/fare girdisini profile uyumlu bir
  JSON dosyasına kaydeder (`--record-input`).
- **Kendi kendini sınama** — `macrostat-engine.exe --self-test`, GUI açmadan
  kanca zincirini, enjeksiyon imzası yalıtımını, dış enjeksiyon görünürlüğünü,
  panik mandalını ve takılı tuş temizliğini denetler.
- **Türkçe ve İngilizce arayüz**, makro düzenleyici, canlı günlük paneli, ilk
  kurulum sihirbazı.
- **Tasarım gereği çevrimdışı** — ağ, telemetri, güncelleyici veya bulut yok.

### Platform desteği

- Windows 11, **yalnızca x64**, resmî olarak desteklenen platformdur.
- Windows 10 22H2 (derleme 19045), x64, yalnızca geliştirme ve test içindir;
  resmî olarak desteklenen bir platform değildir.
- ARM64, 32 bit Windows ve Windows Server kapsam dışıdır.
- Windows 10 22H2 normal destek süresini doldurmuştur; genişletilmiş güvenlik
  güncellemesi (ESU) durumu kurulum ortamına bağlıdır.
- CPU, RAM veya disk gereksinimi belirtilmez: ölçülmemiştir. Yalnızca aşağıdaki
  derleme gereksinimleri doğrulanmıştır.

### Kurulum

MacroStat'ı edinmenin iki desteklenen yolu vardır. Henüz ikisi de kararlı ve
herkese açık bir sürüm değildir.

#### 1. CI tarafından üretilen hazır artefaktlar

GitHub Actions iş akışının başarılı her `build` işi, `macrostat-win-x64` adlı iş
artefaktı olarak (30 gün saklanır) şunları üretir:

- `MacroStat-Setup-1.0.0.exe` — kullanıcı bazlı kurulum
  (`%LOCALAPPDATA%\Programs\MacroStat`), yönetici izni gerekmez;
- `MacroStat-1.0.0-portable.zip` — kurulum gerektirmeyen taşınabilir paket;
- `SHA256SUMS.txt` — SHA-256 toplamları;
- `MacroStat/` — ham PyInstaller çıktısı.

Artefaktlar yalnızca bu depoya erişim yetkisi olan kullanıcılar tarafından
indirilebilir. Depo özel (private) olduğundan bu artefaktlar herkese açık
indirme bağlantıları değildir.

Bu proje için kararlı bir indirme sayfası veya GitHub Release yayımlanmamış ve
doğrulanmamıştır; bu nedenle burada indirme bağlantısı verilmez. Bir artefakt
indirirseniz önce doğrulayın:

```powershell
Get-FileHash .\MacroStat-Setup-1.0.0.exe -Algorithm SHA256
```

Derleme şu anda **imzasızdır**; SmartScreen bilinmeyen yayımcı uyarısı
gösterebilir.

#### 2. Kaynaktan (Windows)

Gereksinimler:

- Windows 10 22H2 / Windows 11 x64;
- Rust 1.79 veya üzeri, MSVC toolchain
  (`stable-x86_64-pc-windows-msvc`);
- Python 3.10 veya üzeri (CI 3.12 kullanır);
- yalnızca kurulum paketi için: Inno Setup 6.3 veya üzeri ve
  `windows/gui/requirements-build.txt` içindeki derleme bağımlılıkları.

Depo kökünden:

```powershell
cargo build --manifest-path windows/engine/Cargo.toml --release --locked
python -m pip install -r windows/gui/requirements-win.txt
python windows/gui/main.py
```

Motor ikilisi `windows/engine/target/release/macrostat-engine.exe` altında
bulunmalıdır; GUI onu orada arar. Motor bir Windows ikilisidir; ürünün Linux'ta
veya Wine altında çalıştırılması desteklenmez ve doğrulanmamıştır. Windows dışı
sistemlerde yalnızca platformdan bağımsız Rust çekirdeği `cargo test` ile
derlenip test edilebilir.

Kurulum paketini ve taşınabilir ZIP'i üretmek için
[`windows/scripts/README.md`](windows/scripts/README.md) ve
[`windows/README.md`](windows/README.md) belgelerine bakın.

### Hızlı başlangıç

1. MacroStat'ı başlatın (kurulumdan sonra Başlat menüsü kısayolu, taşınabilir
   pakette `MacroStat.exe` veya kaynaktan `python windows/gui/main.py`).
2. İlk kurulum sihirbazı ortamı denetler ve motorun başlatılabildiğini onaylar.
   Yönetici izni istenmez.
3. Makro düzenleyiciyi açın, bir makro oluşturun, tetikleme tuşunu ve kipini
   seçin, eylemleri ekleyin.
4. Profili kaydedin; motor profili yeniden yükler. Durum çubuğu motorun ve
   kancanın çalışıp çalışmadığını gösterir.
5. Motoru durdurmak ve basılı tuşları bırakmak için panik tuşuna (varsayılan
   `KEY_PAUSE`) basın.

### Kullanım

- **Makrolar sayfası** — makroları listeler, etkinleştirir/devre dışı bırakır ve
  test eder. Test çalıştırması tek örnek kilidini almadan tek bir makroyu
  yürütür (`macrostat-engine.exe --test-macro <id>`).
- **Makro düzenleyici** — tetikleme tuşu ve kipi, eylem listesi, gecikmeler, fare
  yolları ve metin eylemleri. Profiller JSON dosyalarıdır; yüklenirken doğrulanır
  ve sınırlandırılır.
- **Kaydedici** — girdiyi motor üzerinden sistem genelinde kaydeder ve profile
  uyumlu bir kayda dönüştürür.
- **Sistem ve kılavuz sayfası** — motor durumu, tanılama, günlük paneli, yedekleme
  ve ilk kurulum.
- **Tanılama** — GUI olmadan kanca/enjeksiyon zincirini sınamak için
  `windows\dist\MacroStat\_internal\macrostat-engine.exe --self-test` çalıştırın.
  Çıkış kodu `0` tüm denetimlerin geçtiğini, `4` en az bir denetimin başarısız
  olduğunu, `5` Windows dışı ortamı gösterir.

Ayrıntılı Windows mühendislik notları:
[`windows/README.md`](windows/README.md).

### Kaldırma

- **Kurulum paketi** — *Uygulamalar ve özellikler* veya Başlat menüsündeki
  kaldırıcıyı kullanın. Yalnızca kendi kurduğu dosyaları siler. Ardından
  `%LOCALAPPDATA%\MacroStat` (profiller, günlükler, `runtime.json`) ve
  `%APPDATA%\MacroStat` (ayarlar) dizinlerinin silinip silinmeyeceğini sorar;
  varsayılan yanıt **Hayır**'dır. Bu istem şu anda Türkçe kurulumda da İngilizce
  görünür.
- **Taşınabilir paket** — MacroStat'ı durdurun ve açtığınız klasörü silin.
  Verilerinizi de kaldırmak istiyorsanız `%LOCALAPPDATA%\MacroStat` ve
  `%APPDATA%\MacroStat` dizinlerini elle silin.
- **Kaynaktan çalıştırma** — MacroStat'ı durdurun ve kopyaladığınız depoyu silin.

### Güvenlik

- İki süreç de kullanıcı olarak (`asInvoker`) çalışır; yönetici yükseltme,
  hizmet, çekirdek sürücüsü, `uiAccess` veya anti-cheat atlatma kullanılmaz.
- IPC kanalı yalnızca yereldir, geçerli kullanıcı SID'iyle sınırlıdır, uzak
  istemcileri reddeder, rastgele 128 bit token kullanır ve ilk örnek bayrağıyla
  oluşturulur. `runtime.json` atomik yazılır.
- Enjekte edilen girdi motor imzası taşır; motor kendi olaylarını yok sayar.
- Profiller güvenilmeyen girdi kabul edilir; kullanılmadan önce sınırlandırılır ve
  temizlenir.
- **Belgelenmiş sınırlar:** yönetici pencerelerine enjeksiyon UIPI tarafından
  engellenir ve kanca güvenli masaüstünde (UAC istemi, kilit ekranı) olay almaz.
  Her iki durumda da makrolar çalışmaz; arayüz sessiz başarısızlık yerine bunu
  bildirir.
- Günlükler yazılan tuş içeriğini barındırmaz.
- Bildirim politikası: [`SECURITY.md`](SECURITY.md) (İngilizce) ve
  [`SECURITY.tr.md`](SECURITY.tr.md) (Türkçe). GitHub Private Vulnerability
  Reporting / Security Advisories özelliğinin bu depo için **etkin olduğu
  doğrulanmamıştır**; **Report a vulnerability** seçeneği yalnızca bir bakımcı
  etkinleştirdiyse kullanılabilir. Seçenek yoksa güvenlik ayrıntısı içermeyen
  normal bir issue açıp bakımcılardan özel bir kanal isteyin; özel kanal
  sağlanana kadar teknik ayrıntı, kanıt kodu veya başka hassas bilgi paylaşmayın.
- Windows tehdit modeli:
  [`windows/docs/SECURITY-WINDOWS.md`](windows/docs/SECURITY-WINDOWS.md).

### Testler

Aksi belirtilmedikçe 14 Kasım 2026'da Linux üzerinde doğrulanmıştır:

| Takım | Komut | Sonuç |
|---|---|---|
| Rust motoru | `cargo test --all` (`windows/engine` içinde) | 99 geçti |
| GUI | `QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -p "test_gui*.py" -v` | 55 geçti |
| Windows E2E | `python -m unittest discover -s windows/tests -p "test_e2e*.py" -v` | Yalnızca Windows; 7 test metodu 9 kabul senaryosunu (U-01…U-09) kapsar |

E2E dosyasında ayrıca UTF-8 çıktı çözümlemesi ve alt süreç borusu yönetimi için
5 platformdan bağımsız regresyon testi bulunur. Windows dışı sistemlerde 7 E2E
metodu atlanır; atlama "geçti" anlamına gelmez.

Doğrulanmış son GitHub Actions koşusu (14 Kasım 2026, koşu 43638328015):
`Dependency audit`, `Engine core (fmt / clippy / test)`, `GUI unit tests`,
`Build installer + portable ZIP` ve `E2E acceptance (real engine)` başarılı;
`Sign artifacts (optional)` imza sertifikası tanımlı olmadığı için atlandı. VM
kabulü (temiz bir Windows VM'de kurulum ve etkileşim) yapılmamıştır.

### Bilinen kısıtlamalar

- **Sürüm adayı; kararlı sürüm değil.**
- **İmzasız ikililer.** SmartScreen uyarı gösterebilir; imza altyapısı hazır ama
  sertifika tanımlı değil.
- **UIPI.** Makrolar yönetici olarak çalışan pencerelere ulaşmaz.
- **Güvenli masaüstü.** UAC istemlerinde ve kilit ekranında makro çalışmaz.
- **VM kabul testi henüz yok.** Davranış birim, GUI ve CI E2E koşularıyla
  doğrulandı; temiz ve etkileşimli bir Windows VM'de doğrulanmadı.
- **Uygulanmamış:** Windows ile otomatik başlatma, sistem tepsisi ve profil
  dosyasını otomatik izleme. Bunlar eski planlama belgelerinde geçer, kodda
  bulunmaz.
- **Kurulum yerelleştirme eksiği.** Kaldırmada veri silme istemi her iki dilde de
  İngilizce görünür.
- **ARM64 kapsam dışıdır.**
- **Windows 10 22H2** normal destek dışındadır; ESU durumu sisteme bağlıdır.

### Katkıda bulunma

Geliştirme ortamı, biçimlendirme, test komutları, CI beklentileri ve depo kuralları
[`CONTRIBUTING.md`](CONTRIBUTING.md) (İngilizce) ve
[`CONTRIBUTING.tr.md`](CONTRIBUTING.tr.md) (Türkçe) belgelerinde açıklanmıştır.

### Lisans

MacroStat **GPL-3.0-or-later** ile lisanslanmıştır. Tam lisans metni
[`LICENSE`](LICENSE) dosyasındadır; kurulumun kullandığı kopya
[`windows/LICENSE`](windows/LICENSE) olup bayt düzeyinde aynıdır. Telif hakkı
(C) 2026 MacroStat contributors.

Üçüncü taraf bileşenler ve lisansları
[`windows/THIRD_PARTY.md`](windows/THIRD_PARTY.md) içinde listelenmiştir.

### Hata bildirimi ve destek

- **Genel hatalar ve öneriler:** [GitHub Issues](../../issues). Önce mevcut
  kayıtları arayın; MacroStat sürümünü, Windows sürümünü, mimariyi ve yeniden
  üretme adımlarını ekleyin.
- **Güvenlik açıkları:** herkese açık issue **açmayın** ve ayrıntı paylaşmayın.
  [`SECURITY.tr.md`](SECURITY.tr.md) sürecini izleyin: özel bildirim seçeneği
  yoksa teknik bilgi paylaşmadan bakımcılardan özel bir kanal isteyin.
- Yanıt süresi veya destek garantisi verilmez.

[Başa dön](#macrostat-for-windows)
