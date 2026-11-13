# Üçüncü Taraf Bileşenler ve Lisansları (Windows)

MacroStat'ın kendi kaynak kodu **GPL-3.0-or-later** ile lisanslanmıştır
(bkz. `LICENSE`). Aşağıdaki bileşenler bu lisansla uyumludur.

## Rust bağımlılıkları

| Paket | Sürüm | Lisans |
|---|---|---|
| `serde` | 1.0.x | MIT OR Apache-2.0 |
| `serde_json` | 1.0.x | MIT OR Apache-2.0 |
| `rand` | 0.8.x | MIT OR Apache-2.0 |
| `windows-sys` | 0.61.x | MIT OR Apache-2.0 |

`Cargo.lock` içindeki geçişli paketler **MIT**, **Apache-2.0**,
**BSD-2-Clause**, **Unlicense**, **Unicode-3.0** veya
**Apache-2.0 WITH LLVM-exception** lisans ailelerinden birini kullanır.
Tam liste `windows/engine/Cargo.lock` içinde sabitlenmiştir.

## Arayüz ve paketleme

| Bileşen | Lisans | Not |
|---|---|---|
| PyQt6 / PyQt6-Qt6 | GPL-3.0 | PyQt6 GPLv3 altında dağıtılır; bu nedenle MacroStat GPL-3.0-or-later'dır. |
| Qt 6 | LGPL-3.0 / GPL-3.0 | PyInstaller paketinde dinamik kütüphaneler olarak birlikte gelir. |
| Python | PSF-2.0 | PyInstaller'ın paketlediği yorumlayıcı. |
| PyInstaller | GPL-2.0-or-later WITH bootloader-exception | Yalnızca derleme aracı; çalışma zamanı bileşeni değildir. |
| Inno Setup | Inno Setup License | Yalnızca kurulum üretim aracı; kurulum paketinde çalışma zamanı bileşeni yoktur. |

## Windows sistem bileşenleri

`user32`, `kernel32`, `advapi32`, `dwmapi` — yalnızca belgelenmiş Win32 API'leri
çağrılır; sistem DLL'i yeniden dağıtılmaz. Hiçbir çekirdek sürücüsü, sanal HID
sürücüsü veya üçüncü taraf kanca kütüphanesi kullanılmaz.
