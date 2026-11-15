# Faz 9 — Sorun ve Eksik İncelemesi (12 Kasım 2026)

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


Bu rapor, 1.0.0 kaynak ağacının **geriye kalan sorunlarını ve eksiklerini**
denetler. İnceleme üç eksende yapıldı: (1) çalışma zamanı hatası olabilecek
kod yolları, (2) paketleme/CI tedarik zinciri, (3) belge–kod tutarlılığı.
`linux/` ağacına dokunulmadı.

## 1. Özet

| # | Bulgu | Önem | Durum |
|---|---|---|---|
| G-01 | `--self-test`: async okuma başarısızsa F13 mantıksal basılı kalıyordu | Orta | **Düzeltildi** |
| G-02 | README'de tanılama kipi (`--self-test`) belgelenmemişti | Düşük | **Düzeltildi** |
| G-03 | `REPORT.md` §12 hâlâ "`--self-test` (planlanacak)" diyordu | Düşük | **Düzeltildi** |
| G-04 | Windows CI hiç koşmadı; `cfg(windows)` kodu hiç derlenmedi | **Yüksek** | Açık (ortam) |
| G-05 | VM kabul matrisi (kurulum/DPI/UIPI/SmartScreen) yapılmadı | **Yüksek** | Açık (ortam) |
| G-06 | `cargo audit` / `pip-audit` çalıştırılmadı | Orta | CI `audit` işi eklendi (bkz. 10) |
| G-07 | GitHub Actions etiketleri SHA'ya sabitli değil | Orta | **Düzeltildi** (bkz. 10) |
| G-08 | Bağımlılık sürümleri aralıkla serbest (`PyQt6>=6.5,<7`, `pyinstaller>=6.0,<7`) | Orta | Kısmen: `requirements-build.txt`; tam kilitleme ilk yeşil koşudan sonra |
| G-09 | E2E'nin girdi senaryoları hosted koşucuda atlanabilir | Orta | Kabul edilen sınır |
| G-10 | Kod imzası yok | Orta | Açık (sertifika) |
| G-11 | Uzun süreli (10 dk) dayanıklılık testi yok | Düşük | Açık (VM) |
| G-12 | `--self-test` etkileşimli masaüstüne F13–F15/F24 enjekte eder | Düşük | Belgelenmiş sınır |

## 2. Düzeltilenler

### G-01 — Takılı tuş temizliğinde erken çıkış (gerçek hata)

`windows/engine/src/selftest.rs` içinde "takılı tuş temizliği" kontrolü
`async_down(KEY_F13)` **false** döndüğünde `release_all_keys` çağırmadan
`TAMAM` yazıyordu. `GetAsyncKeyState` okunamazsa (oturum kısıtı, UIPI, farklı
masaüstü) F13 mantıksal olarak basılı kalıyor ve tanı sonrası takılı tuş
bırakıyordu.

Düzeltme: bırakma artık **koşulsuz** yapılıyor; okuma başarısızsa kontrol yine
"async durum okunamadı; kontrol atlandı" olarak raporlanıyor.

### G-02 / G-03 — Belge tutarlılığı

- `windows/README.md`'ye "Tanılama" bölümü eklendi: komut, kontrol listesi,
  çıkış kodları (`0`/`4`/`5`) ve işlev tuşu enjeksiyonu uyarısı.
- `windows/REPORT.md` §12'deki `(planlanacak)` ibaresi `(uygulandı)` oldu.

## 3. Açık işler ve gerekçeleri

### G-04 — Windows derlemesi/CI (yüksek)

Bu ortam çevrimdışı Linux'tur; `cfg(windows)` blokları (kanca, `SendInput`,
kanal DACL'i, Job Object, `windows-sys` FFI) **hiç derlenmedi**. Linux'ta
`cargo clippy -D warnings` temiz olsa da Windows'ta ilk derlemede tür/uyarı
hatası çıkma olasılığı gerçektir. Kapanış: `core`, `gui-tests`, `build`,
`e2e` işlerinin `windows-latest` üzerinde yeşil olması
(`docs/CI-RUNBOOK.md`).

### G-05 — VM kabulü (yüksek)

Kaynak düzeyinde doğrulanamayan tek sınıf gerçek işletim sistemi davranışıdır:
kurulum/kaldırma kalıntısı, SmartScreen görsel kanıtı, %150/%200 DPI, çoklu
monitör negatif koordinatları, yönetici hedef (UIPI) negatifi, UAC/kilit
ekranında kanca yokluğu, RDP ve uyku/uyanma. Bunlar `08-final-report.md` §4
listesindedir.

### G-06 / G-07 — Tedarik zinciri (orta)

- `cargo audit`, `pip-audit` çevrimdışı ortamda çalıştırılamadı; CI'a
  `audit` işi olarak eklendi (bilgilendirici, `continue-on-error`). İlk
  yeşil koşuda sonuç rapora işlenecek.
- `uses:` satırları **doğrulanmış 40 karakterli commit SHA'larına**
  sabitlendi (bkz. `reports/10-ci-hardening.md`). `ci/preflight.py` artık
  etiket/dal sabitlemesini **hata** olarak raporluyor.

### G-08 — Sürüm aralıkları (orta)

`windows/gui/requirements-win.txt` `PyQt6>=6.5,<7`, CI ise
`pyinstaller>=6.0,<7` kuruyor. Aynı kaynak iki farklı tarihte farklı
tekerleklerle paketlenebilir; ayrıca PyInstaller sürümü `build.ps1`'de yalnızca
"kurulu mu" diye kontrol ediliyor. Öneri: ilk yeşil CI koşusundan sonra
`python -m pip freeze` çıktısından `requirements-build.txt` üretip CI ve
`build.ps1`'i bu dosyaya bağlamak, PyQt6 sürümünü de tam sabitlemek. Bu adım
gerçek Windows ortamı olmadan yapılmamalı (yanlış sürümü kilitleme riski).

### G-09 — E2E girdi senaryoları (orta)

Hosted koşucu etkileşimli oturum sağlamazsa U-05, U-08, U-09 atlanır ve bu
"geçti" sayılmaz (test çıktısında `skipped`). Kalıcı çözüm etkileşimli VM'dir;
kod tarafında ek iş yok.

### G-10 / G-11 — İmza ve dayanıklılık (orta/düşük)

Sertifika dış bağımlılıktır; altyapı (`sign.ps1`, `verify.ps1`,
`SIGNING_ENABLED` koşulu) hazırdır. 10 dakikalık döngü/sel testi VM işidir ve
CI varsayılanına süre nedeniyle eklenmedi.

## 4. İncelenip sorun bulunmayan alanlar

| Alan | Sonuç |
|---|---|
| `recorder` durdurma yolu | Sentinel Pause **imzasız** enjekte ediliyor; kayıtçı kancası onu dış olay olarak görüyor — doğru. İmzalı olsaydı durdurma çalışmazdı. |
| `logpanel.poll()` | Döndürülmüş dosya ve kısalma (`st_size < offset`) doğru ele alınıyor; okuma sınırlı. |
| `common.stop_engine` | `runtime.json` yoksa `True` (zaten durmuş) — beklenen semantik. |
| `--watch-restart` tuş karşılaştırması | Tam eşitlik bilinçli: yeniden başlatma tuşu düğme kabul etmez, sol/sağ değiştirici ayrımı korunur. |
| Kayıt durdurma tuşu | `keys_are_equivalent` ile fare düğmeleri de kapsanıyor (Faz 6/M3). |
| GUI tek örnek / motor Job Object | Kapanış sırası ve çift `release_all` yolu tutarlı. |

## 5. Doğrulama (bu inceleme sonrası)

| Komut | Sonuç |
|---|---|
| `cargo fmt --all -- --check` | Temiz |
| `cargo clippy --all-targets --offline -- -D warnings` | Temiz |
| `cargo test --locked --offline` | **95/95** |
| `python -m unittest discover -s windows/tests` (`QT_QPA_PLATFORM=offscreen`) | **54 test, 7 atlandı** (Windows E2E) |
| `python ci/preflight.py` | Geçti (yalnızca SHA sabitleme uyarıları) |

## 6. Sonuç

Kaynak düzeyinde **kapatılması gereken yeni bir hata kalmadı**. Kalan maddeler
ya ortam sınırıdır (Windows derlemesi/VM, ağ gerektiren denetimler ve SHA
sabitleme, sertifika) ya da ilk yeşil koşudan sonra yapılması doğru olan
sürüm kilitlemesidir. Sürüm kararı için tek zorunlu kapı G-04 ve G-05'tir.
