# MacroStat 1.0.0 — Windows Sürüm Notları

**Durum:** sürüm adayı. İkili dosyalar `windows-latest` CI koşusunda üretilir;
bu belge kaynak ağacındaki sürümü tarif eder.

## Öne çıkanlar

- **Windows'a tam taşıma.** Linux motorunun davranışı korunarak Rust ile
  yeniden yazıldı: 7 tetikleme kipi, 11 eylem türü, kombo tetikleyiciler,
  `TOGGLE_LOOP`/`WHILE_HELD`/`PRESS_RELEASE` semantiği.
- **Sistem geneli girdi.** `WH_KEYBOARD_LL` + `WH_MOUSE_LL` düşük seviyeli
  kancalar; `SendInput` ile enjeksiyon; kendi enjeksiyonunu imza ile yok sayma.
- **Panik tuşu her koşulda çalışır.** Dolu kanca kuyruğunda bile tuş kancada
  mandallanır; tuşlar bırakılarak motor kapanır.
- **Tek örnek + güvenli IPC.** Kullanıcı SID'i ve rastgele 128 bit token'lı
  adlandırılmış kanal, korumalı DACL, ilk örnek bayrağı, PID/oluşturma zamanı
  doğrulaması.
- **PyQt6 arayüz.** Türkçe/İngilizce, profil düzenleyici, kayıt sihirbazı,
  günlük paneli, onboarding; yönetici izni istemez (`asInvoker`).
- **Kendi kendini sınar.** `macrostat-engine --self-test` kanca, imza yalıtımı,
  dış enjeksiyon, panik mandalı ve takılı tuş temizliğini GUI'siz doğrular.
- **Çevrimdışı.** Ağ, telemetri, otomatik güncelleme yok.

## Güvenlik

- `SetDefaultDllDirectories` ile DLL arama sırası daraltıldı.
- Profil sınırları zorunlu: 500 makro, makro başına 2000 eylem, 64 KiB `TEXT`,
  200 000 `MOUSE_PATH` noktası, 600 s öteleme sınırları.
- `runtime.json`, ayarlar ve kayıtlar atomik yazılır; sembolik bağ reddedilir.
- Günlükte tuş içeriği yok; denetim karakterleri temizlenir.
- Ayrıntı: `windows/docs/SECURITY-WINDOWS.md`,
  `windows/reports/06-security-review.md`.

## Kurulum

1. `MacroStat-Setup-1.0.0.exe` — kullanıcı bazlı kurulum
   (`%LOCALAPPDATA%\Programs\MacroStat`), yönetici gerekmez.
2. `MacroStat-1.0.0-portable.zip` — kurulum istemeyen taşınabilir sürüm.
3. `SHA256SUMS.txt` — indirdikten sonra doğrulayın:

```powershell
Get-FileHash .\MacroStat-Setup-1.0.0.exe -Algorithm SHA256
```

## Bilinen sınırlar

- **Kod imzası yok.** SmartScreen "bilinmeyen yayımcı" uyarısı gösterebilir;
  "Daha fazla bilgi → Yine de çalıştır" ile devam edilir. İmza altyapısı hazır,
  sertifika sağlanınca aynı sürüm imzalanabilir.
- **UIPI.** Yönetici olarak çalışan pencerelere enjeksiyon Windows tarafından
  engellenir; uygulamayı yönetici olarak başlatmak gerekir.
- **Güvenli masaüstü.** UAC istemi ve kilit ekranında kanca olay almaz;
  bu sırada makro çalışmaz, tuşlar bırakılır.
- **Windows 10 22H2 + Windows 11 x64** hedeflenir; ARM64 kapsam dışıdır.
- **Windows 10 22H2** Ekim 2025 sonrası normal destek dışındadır; ESU durumu
  kurulum ortamına göre değerlendirilmelidir.

## Doğrulama

- Rust: `cargo fmt --check`, `cargo clippy -D warnings`, **95** test.
- Python: **47** birim testi + **9** Windows E2E senaryosu.
- Ayrıntı: `windows/reports/07-e2e.md`, `windows/reports/08-final-report.md`.

## Lisans

GPL-3.0-or-later. Üçüncü taraf bileşenler: `THIRD_PARTY.md`.
