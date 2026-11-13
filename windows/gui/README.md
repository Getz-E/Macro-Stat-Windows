# GUI (PyQt6) — Faz 4 (tamamlandı)

Bu dizin, `../linux/macrostat/` paketinin Windows taşımasıdır. Giriş noktası `main.py`; paket `macrostat/`.

```
pip install -r requirements-win.txt
python main.py            # depo kökünden: python gui/main.py
```

Tek örnek kilidi `%LOCALAPPDATA%\MacroStat\macrostat-gui.lock` (`QLockFile`); ayarlar
`%APPDATA%\MacroStat\settings.json`; profil, `runtime.json` ve günlük
`%LOCALAPPDATA%\MacroStat` altında.

## Taşıma ilkesi

- Platformdan bağımsız modüller **olduğu gibi** kullanılır (kopya `windows/gui/macrostat/` altında; `linux/` değişmez).
- Linux'a özgü yarılar Windows katmanıyla değiştirilir.

| Modül | İşlem |
|---|---|
| `sanitize.py`, `transfer.py`, `theme.py`, `widgets.py`, `nav.py`, `macros.py`, `settings.py` | Değişmeden kullanılır |
| `i18n.py` | Windows'a özgü metinlerin EN karşılıkları eklendi |
| `logpanel.py` | Döndürme algısı `st_ino` yoksa `st_mtime_ns` gerilemesine düşer |
| `core.py` | Yeniden başlatma `app.restart_arguments()` (frozen EXE / `main.py`) |
| `editor.py` | Makro testinde motor konsolu gizlenir |
| `common.py` | Linux yarısı (`/proc`, `fcntl`, `getuid`, XDG, `O_NOFOLLOW`) yerine `%LOCALAPPDATA%`/`%APPDATA%`, reparse-point reddi, `runtime.json`, `--ipc` istemcisi; atomik yazma korunur |
| `winplatform.py` | ctypes Win32: süreç kimliği, Job Object, `SendInput`, `GetCursorPos`, DWM, DPI |
| `system.py` | udev yerine motor başlatma + kanal + Job Object + PID kimliği; yedekleme |
| `recorder.py` | Sistem geneli kayıt motor `--record-input` ile; durdurma sentinel Pause |
| `kwin.py` | Dahil edilmez; imleç `GetCursorPos` |
| `app.py` | Tek örnek (`QLockFile`), DPI, koyu başlık çubuğu |
| `onboarding.py` | udev adımı yerine ortam kontrolleri, Windows notları, motor onayı; yönetici izni istemez |

## Kabul

- Tüm sayfalar çalışır; TR/EN; profil içe/dışa; onboarding yönetici izni istemez. ✅ (kod düzeyi)
- Profil biçimi `../docs/PROFILE-CONTRACT.md` ile aynıdır. ✅
- Testler: `QT_QPA_PLATFORM=offscreen python -m unittest discover -s ../tests -v` (45 test)
- Rapor: `../reports/04-gui.md`

## Paketleme (Faz 5)

`MacroStat.spec` PyInstaller **onedir** paketi üretir: `MacroStat.exe` + `_internal/`
(motor, Qt6, Python, `assets/`). Motor ikilisi `../engine/target/release/macrostat-engine.exe`
altında yoksa derleme bilinçli olarak durur. Simge `assets/macrostat.ico`, sürüm kaynağı
`version_info.txt`.

```
powershell -ExecutionPolicy Bypass -File ..\scripts\build.ps1
```

Ayrıntı: `../reports/05-packaging.md`.
