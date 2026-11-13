# MacroStat — Windows

MacroStat'ın Windows 10 22H2 / Windows 11 (x64) sürümü. Arayüz PyQt6,
girdi motoru Rust (`SendInput` + düşük seviye kancalar), paketleme
PyInstaller + Inno Setup. Ürün **tamamen çevrimdışıdır**: telemetri,
otomatik güncelleme, bulut veya ağ erişimi yoktur.

| | |
|---|---|
| **Durum** | Faz 6–8 kaynak düzeyinde tamamlandı — motor **95** birim testi, GUI **47** test, gerçek ikili üzerinde **9** E2E senaryosu. Sürüm **aday**. İlk Windows CI koşusu, VM kabulü ve kod imzası bekliyor. |
| **Lisans** | GPL-3.0-or-later — bkz. [`LICENSE`](LICENSE) ve [`windows/LICENSE`](windows/LICENSE) |
| **Hedef** | Windows 10 22H2 + Windows 11, x64 |
| **Ayrıcalık** | `asInvoker` (yönetici istemez); yönetici pencereler ve güvenli masaüstü belgelenmiş sınırlardır |

## Depo yerleşimi

```
windows/     Windows sürümünün tamamı (motor, GUI, paketleme, testler, belgeler)
  README.md  Ayrıntılı Windows belgesi — buradan başlayın
  engine/    Rust motoru (MSVC hedefi)
  gui/       PyQt6 arayüzü ve PyInstaller spec
  installer/ Inno Setup tanımı
  scripts/   build.ps1 / sign.ps1 / verify.ps1
  tests/     Windows'a özgü Python testleri
  reports/   Faz raporları ve inceleme belgeleri
ci/          GitHub Actions kaynağı ve çevrimdışı ön kontrol
  windows.yml          iş akışının kaynağı (SHA'ya sabitli)
  preflight.py         çevrimdışı sözleşme denetimi
  requirements.txt     preflight için PyYAML
.github/workflows/windows.yml   etkin GitHub Actions iş akışı (ci/windows.yml kopyası)
```

`linux/` ağacı bu depoya **dahil değildir** ve `.gitignore` ile dışlanmıştır;
Windows derlemesi Linux ağacına bağımlı değildir.

## Hızlı başlangıç (Windows 10/11 x64)

```powershell
pip install -r windows\gui\requirements-win.txt
python windows\gui\main.py
```

Paketleme (tek komut):

```powershell
powershell -ExecutionPolicy Bypass -File windows\scripts\build.ps1
```

Çıktılar `windows\dist\` altında: `MacroStat-Setup-<sürüm>.exe`,
`MacroStat-<sürüm>-portable.zip`, `SHA256SUMS.txt`.

Tanılama:

```powershell
windows\dist\MacroStat\_internal\macrostat-engine.exe --self-test
```

## Testler ve denetimler

```bash
# Motor
cd windows/engine && cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test

# GUI + E2E (Windows'ta E2E, Linux'ta atlanır)
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests

# Çevrimdışı CI ön kontrolü
python -m pip install -r ci/requirements.txt
python ci/preflight.py
```

## CI

`ci/windows.yml` ile `.github/workflows/windows.yml` **birebir aynıdır**;
`preflight.py` eşitliği ve iş grafiğini denetler. İşler: `core` (fmt/clippy/
test + anahtar tablosu), `gui-tests`, `build`, `e2e`, `audit` (PR'da
bilgilendirici, push/etiket/haftalık taramada engelleyici), `sign`
(yalnızca `push`/`workflow_dispatch`, `SIGNING_ENABLED=true` ve korumalı
`signing` ortamı). Bütün `uses:` satırları 40 karakterli commit SHA'larına
sabitlidir.

İlk gerçek koşu için adım adım kılavuz:
[`windows/docs/CI-RUNBOOK.md`](windows/docs/CI-RUNBOOK.md).

## Belgeler

- [`windows/README.md`](windows/README.md) — Windows sürümünün ana belgesi
- [`windows/REPORT.md`](windows/REPORT.md) — inceleme, fizibilite, mimari, riskler
- [`windows/reports/`](windows/reports/) — faz raporları (kaynak, güvenlik, E2E, boşluk incelemesi)
- [`windows/docs/SECURITY-WINDOWS.md`](windows/docs/SECURITY-WINDOWS.md) — tehdit modeli
- [`windows/docs/PROFILE-CONTRACT.md`](windows/docs/PROFILE-CONTRACT.md) — donmuş profil sözleşmesi

## Bilinen sınırlar

- Kod imzası **yoktur**; SmartScreen uyarısı beklenir (altyapı hazır, sertifika dış bağımlılık).
- Yönetici olarak çalışan pencerelere enjeksiyon UIPI nedeniyle yapılamaz; arayüz bunu bildirir.
- UAC istemi ve güvenli masaüstünde kanca çalışmaz.
- VM kabul matrisi (DPI, çoklu monitör, RDP, uyku/uyanma) henüz yürütülmemiştir.

## Lisans

GPL-3.0-or-later. Ayrıntı ve üçüncü taraf bileşenler için
[`windows/THIRD_PARTY.md`](windows/THIRD_PARTY.md).
