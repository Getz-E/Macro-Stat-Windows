# Kurulum — Faz 5

Inno Setup 6 ile **kullanıcı bazlı** kurulum (`MacroStat.iss`, UTF-8 BOM'lu).

## Sözleşme

- `PrivilegesRequired=lowest`; varsayılan dizin `%LOCALAPPDATA%\Programs\MacroStat`. Yönetici izni istenmez.
- Kurulanlar: `MacroStat.exe`, `_internal/macrostat-engine.exe`, `_internal/` (Qt, Python, `assets/`), `LICENSE`, `THIRD_PARTY.md`, `VERSION`, kısayollar.
- Lisans sayfası GPL-3.0-or-later (`LicenseFile=..\LICENSE`).
- Diller: English + Türkçe; masaüstü simgesi görevi **varsayılan kapalı**.
- `ArchitecturesAllowed=x64compatible`, `MinVersion=10.0.19045` (Win10 22H2+).
- Kaldırma yalnızca kendi kurduğu dosyaları siler; `%LOCALAPPDATA%\MacroStat` ve `%APPDATA%\MacroStat` için **varsayılan "Hayır"** olan bir onay sorar (`MB_DEFBUTTON2`).
- İmza opsiyonel (`../scripts/sign.ps1`, `../docs/CODE-SIGNING.md`); imzasız sürümde SmartScreen notu.

## Derleme

```powershell
iscc "/DMyAppVersion=1.0.0" windows\installer\MacroStat.iss
```

Normalde doğrudan çağrılmaz: `scripts/build.ps1` önce `dist/MacroStat/` paketini üretir, sonra ISCC'yi çalıştırır. Kaynak `..\dist\MacroStat\*` olduğu için paket yoksa derleme başarısız olur (istenen davranış).

## Çıktı

`windows/dist/MacroStat-Setup-1.0.0.exe` + `SHA256SUMS.txt` + `MacroStat-1.0.0-portable.zip`.
