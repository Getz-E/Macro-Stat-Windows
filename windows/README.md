# MacroStat — Windows sürümü

Bu dizin, MacroStat'ın Windows'a taşınmasına ait **tüm** çalışmayı barındırır. Linux ağacı (`../linux/`) salt okunur referanstır; burada üretilen hiçbir dosya Linux tarafını değiştirmez.

| | |
|---|---|
| **Durum** | Faz 6–8 kaynak düzeyinde tamamlandı — motor **95** birim testi, GUI **47** test, gerçek ikili üzerinde **9** E2E senaryosu; sürüm **aday**. Windows CI ilk koşusu, VM kabulü ve imza `docs/CI-RUNBOOK.md` + `reports/08-final-report.md` kontrol listesinde bekliyor |
| **Hedef** | Windows 10 22H2 + Windows 11, x64 |
| **Motor** | Rust (MSVC hedefi) — `engine/` |
| **Arayüz** | PyQt6, Python 3.10+ — `gui/` |
| **Paketleme** | PyInstaller (onedir) + Inno Setup — `installer/` |
| **Ağ** | Yok. Çevrimdışı; telemetri, otomatik güncelleme, bulut yok. |
| **Lisans** | GPL-3.0-or-later |

## Nereden başlanır

| Belge | İçerik |
|---|---|
| [`REPORT.md`](REPORT.md) | İnceleme, fizibilite, mimari, riskler, kararlar (ana rapor) |
| [`reports/00-inceleme-ve-planlama.md`](reports/00-inceleme-ve-planlama.md) | İnceleme adımı raporu |
| [`reports/arastirma-01-windows-sinirlari-ve-alternatifler.md`](reports/arastirma-01-windows-sinirlari-ve-alternatifler.md) | UIPI / güvenli masaüstü / sanal HID araştırması |
| [`reports/00-baseline.md`](reports/00-baseline.md) | Faz 0 raporu ve kabul kanıtları |
| [`reports/01-engine-core.md`](reports/01-engine-core.md) | Faz 1 raporu: çekirdek, Windows anahtar tablosu, eşdeğerlik |
| [`reports/02-input-backend.md`](reports/02-input-backend.md) | Faz 2 raporu: kancalar, `SendInput`, kayıt, UIPI önsezisi |
| [`reports/03-ipc-lifecycle.md`](reports/03-ipc-lifecycle.md) | Faz 3 raporu: IPC, yaşam döngüsü, günlük, `--record-input` |
| [`reports/04-gui.md`](reports/04-gui.md) | Faz 4 raporu: PyQt6 taşıması, modül farkları, testler |
| [`reports/05-packaging.md`](reports/05-packaging.md) | Faz 5 raporu: derleme zinciri, kurulum, imza, CI, doğrulama |
| [`reports/06-security-review.md`](reports/06-security-review.md) | Faz 6 raporu: güvenlik incelemesi, bulgular, düzeltmeler, artık riskler |
| [`reports/07-e2e.md`](reports/07-e2e.md) | Faz 7 raporu: uçtan uca motor kabul koşucusu, senaryolar, CI işi |
| [`reports/08-final-report.md`](reports/08-final-report.md) | Faz 8 raporu: nihai durum, sürüm kontrol listesi, açık işler |
| [`reports/09-gap-analysis.md`](reports/09-gap-analysis.md) | Sorun/eksik incelemesi: düzeltmeler ve kalan maddeler |
| [`reports/10-ci-hardening.md`](reports/10-ci-hardening.md) | CI/tedarik zinciri: SHA sabitleme, `audit` işi, paketleme bağımlılıkları |
| [`reports/11-repo-prep.md`](reports/11-repo-prep.md) | Depo hazırlığı: kök dosyalar, etkin workflow, tetikleyiciler, audit politikası, preflight sertleştirmesi |
| [`reports/12-security-audit.md`](reports/12-security-audit.md) | Kapsamlı güvenlik denetimi: bulgular, düzeltmeler, testler, kalan riskler |
| [`RELEASE-NOTES-1.0.0.md`](RELEASE-NOTES-1.0.0.md) | 1.0.0 sürüm notları ve bilinen sınırlar |
| [`docs/CI-RUNBOOK.md`](docs/CI-RUNBOOK.md) | İlk gerçek Windows CI koşusu için adım adım kılavuz |
| [`docs/WINDOWS-ARCHITECTURE.md`](docs/WINDOWS-ARCHITECTURE.md) | Süreç modeli, modüller, iş parçacıkları, katman sınırları |
| [`docs/PROFILE-CONTRACT.md`](docs/PROFILE-CONTRACT.md) | **Donmuş** profil ve anahtar sözleşmesi |
| [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) | Linux ↔ Windows yetenek/anahtar eşlemesi |
| [`docs/SECURITY-WINDOWS.md`](docs/SECURITY-WINDOWS.md) | Windows tehdit modeli ve önlemler |
| [`docs/TEST-MATRIX.md`](docs/TEST-MATRIX.md) | Kabul test matrisi (sonuçlar fazlarda doldurulur) |
| [`docs/CODE-SIGNING.md`](docs/CODE-SIGNING.md) | İmza seçenekleri ve öneri |

## Yerleşim

```
windows/
  README.md  REPORT.md  reports/  docs/
  VERSION    LICENSE  THIRD_PARTY.md
  engine/    Cargo.toml + src/{lib,main,profile,keymap,trigger,actions,hook,inject,mouse,ipc,log,winutil,record,runtime}.rs
             assets/default_profile.json   tools/gen_keymap.py
  gui/       main.py + macrostat/{app,common,system,recorder,winplatform,...}.py
             MacroStat.spec  version_info.txt  requirements-win.txt
             assets/{default_profile.json,macrostat.svg,macrostat.ico}
  installer/ MacroStat.iss (Inno Setup 6, kullanıcı kapsamlı)
  scripts/   build.ps1  sign.ps1  verify.ps1
  tests/     Windows'a özgü Python testleri (Faz 3+)

../ci/windows.yml   GitHub Actions kaynağı; .github/workflows/windows.yml birebir kopyasıdır
```

## Geliştirme ortamı

- **Derleme/çalıştırma:** Windows 10/11 x64 veya GitHub Actions `windows-latest`. Wine ile EXE üretimi **desteklenmez**; motor Linux'ta yalnızca platformdan bağımsız çekirdek için `cargo check/test` edilebilir.
- **Rust:** 1.79+ (MSVC toolchain, `stable-x86_64-pc-windows-msvc`).
- **Python:** 3.10+; `pip install -r gui/requirements-win.txt`.
- **Inno Setup:** 6.3+ (Faz 5).

## Arayüz (Faz 4)

Giriş noktası `gui/main.py`; paket `gui/macrostat/`. Çalıştırma:

```
pip install -r gui/requirements-win.txt
python gui/main.py
```

| Modül | Windows karşılığı |
|---|---|
| `app.py` | Tek örnek (`QLockFile`), `QApplication` öncesi DPI, koyu başlık çubuğu, yeniden başlatmada EXE/`main.py` ayrımı |
| `winplatform.py` | ctypes: süreç kimliği, Job Object, `SendInput`, `GetCursorPos`, DWM, reparse-point |
| `common.py` | `%LOCALAPPDATA%`/`%APPDATA%` yolları, atomik yazma, `runtime.json`, `--ipc` istemcisi, güvenli motor arama |
| `system.py` | Motoru `--parent-pid` + Job Object ile başlatma; STOP/`--watch-restart`; tanılama; yedekleme |
| `recorder.py` | Kayıt motorun `--record-input` kipiyle; durdurmak için `SendInput` ile sentinel Pause |
| `onboarding.py` | Yönetici izni istemeyen 3 adım: kontroller, Windows notları, motor onayı |
| Diğer | `core/macros/editor/nav/widgets/theme/i18n/transfer/sanitize/settings/logpanel` Linux ile aynı davranış, yol/OS uyarlamalarıyla |

Testler: `QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v`

## Paketleme (Faz 5)

Tek komut, Windows 10/11 x64 üzerinde:

```powershell
powershell -ExecutionPolicy Bypass -File windows\scripts\build.ps1
```

Zincir: `cargo build --release --locked` → PyInstaller onedir → `dist\MacroStat\` → portable ZIP → Inno Setup kurulumu → `SHA256SUMS.txt`. Sürüm `windows/VERSION`'dan gelir ve `engine/Cargo.toml` ile eşleşmek zorundadır.

Üretilenler (`windows/dist/`):

| Dosya | İçerik |
|---|---|
| `MacroStat-Setup-1.0.0.exe` | Kullanıcı kapsamlı kurulum (yönetici istemez) |
| `MacroStat-1.0.0-portable.zip` | Kurulumsuz taşınabilir paket |
| `MacroStat/` | Ham PyInstaller çıktısı (imzalama girdisi) |
| `SHA256SUMS.txt` | `sha256  ad` biçiminde bütünlük toplamları |

İmza ve doğrulama:

```powershell
powershell ... -File windows\scripts\sign.ps1 -PfxPath <pfx> -PfxPassword <sır>
powershell ... -File windows\scripts\verify.ps1 [-RequireSignature]
```

CI: `ci/windows.yml` → `core` (fmt/clippy/test), `gui-tests`, `build`, `e2e` (gerçek ikili üzerinde uçtan uca kabul), `audit` (`cargo audit --deny warnings`/`pip-audit --strict`; PR'da bilgilendirici, push/etiket/haftalık taramada engelleyici), `sign` (yalnızca `push`/`workflow_dispatch`, `SIGNING_ENABLED=true` ve korumalı `signing` ortamı). Bütün `uses:` satırları doğrulanmış commit SHA'larına sabittir. Ayrıntı: [`reports/05-packaging.md`](reports/05-packaging.md), [`reports/07-e2e.md`](reports/07-e2e.md).

## Tanılama

GUI açılmadan kanca/enjeksiyon zincirini sınamak için:

```powershell
windows\dist\MacroStat\_internal\macrostat-engine.exe --self-test
```

Kontroller: imza sabiti, kanca kurulumu, kendi enjeksiyonunu yok sayma, dış
enjeksiyonu görme, panik mandalı, takılı tuş temizliği ve bütünlük (UIPI)
özeti. Çıkış kodu `0` = tümü geçti, `4` = en az bir kontrol başarısız,
`5` = Windows dışı ortam. Tanı, F13/F14/F15/F24 gibi işlev tuşlarını kısa süre
enjekte eder; etkileşimli bir masaüstünde çalıştırırken bunu göz önünde tutun.

## Değişmez kurallar

1. `../linux/` **salt okunur**; oradaki hiçbir dosya değiştirilmez.
2. Windows'a ait her dosya, komut çıktısı ve rapor `windows/` altında üretilir.
3. Her faz sonunda Türkçe rapor: `reports/NN-*.md`.
4. Çekirdek sürücüsü yok, anti-cheat atlatma yok, ağ yok, `uiAccess` yok (v1).
5. Yönetici pencereler ve güvenli masaüstü **belgelenmiş sınırlardır**; sessiz başarısızlık yerine arayüzde farkındalık gösterilir.
