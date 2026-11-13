# Build / imza / doğrulama betikleri — Faz 5

| Betik | Görev |
|---|---|
| `build.ps1` | Sürüm çapraz kontrolü → `cargo build --release --locked` (MSVC) → GUI testleri (opsiyonel) → PyInstaller onedir → portable ZIP → Inno Setup → `SHA256SUMS.txt` |
| `sign.ps1` | `signtool` ile Authenticode + RFC 3161 zaman damgası (PFX veya mağaza parmak izi; opsiyonel) |
| `verify.ps1` | Zorunlu dosyalar, SHA-256, `macrostat-engine.exe --version`, Authenticode durumu; hiçbir şeyi değiştirmez |

## Kullanım (depo kökünden)

```powershell
powershell -ExecutionPolicy Bypass -File windows\scripts\build.ps1
powershell ... -File windows\scripts\build.ps1 -SkipInstaller -SkipTests -Version 1.0.0
powershell ... -File windows\scripts\sign.ps1 -PfxPath C:\secure\cert.pfx -PfxPassword (Get-Content C:\secure\pw.txt)
powershell ... -File windows\scripts\sign.ps1 -Thumbprint <40-hex>
powershell ... -File windows\scripts\verify.ps1 [-RequireSignature] [-Dist windows\dist]
```

## Kurallar

- PATH'e sessizce güvenilmez: her araç açık yolla, bilinen adayla veya `Get-Command` + anlaşılır hata ile çözülür (`-Cargo`, `-Python`, `-Iscc`, `-Signtool`).
- `-Version` verilmezse `windows/VERSION` kullanılır; `engine/Cargo.toml` ile eşleşmezse derleme **durur**.
- Çıktılar yalnızca `windows/dist/` altında; `dist/` her derlemede temizlenir.
- Sırlar hiçbir zaman yazdırılmaz, diske kalıcı yazılmaz (CI'da geçici PFX `finally` ile silinir).
- `verify.ps1` salt okunurdur ve yayın kapısıdır; sert hata çıkış kodunu 1 yapar.

Ayrıntılar: [`../reports/05-packaging.md`](../reports/05-packaging.md), [`../docs/CODE-SIGNING.md`](../docs/CODE-SIGNING.md).
