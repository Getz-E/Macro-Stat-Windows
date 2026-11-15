# Kabul Test Matrisi

Sonuçlar fazlar ilerledikçe doldurulur. `Durum`: ☐ bekliyor, ✅ geçti, ❌ kaldı, ⚠️ belgelenmiş sınır.

> **Güncel test toplamı (14 Kasım 2026):** Rust **99**, GUI **55**; E2E dosyasında
> **12** test metodu (7'si yalnızca Windows'ta koşar ve 9 kabul senaryosunu
> kapsar, 5'i platformdan bağımsız regresyon testidir). Aşağıdaki faz
> bölümlerindeki sayılar **o tarihteki** sonuçlardır ve geriye dönük
> değiştirilmemiştir.

## 1. Ortamlar

| Kod | Ortam |
|---|---|
| W10 | Windows 10 22H2 x64 (build 19045), ESU'lu ve ESU'suz |
| W11 | Windows 11 x64 (güncel) |
| DPI100 / DPI150 / DPI200 | %100 / %150 / %200 ölçek |
| MONO / MULTI | Tek monitör / çoklu monitör (negatif koordinat) |
| KB-TRQ / KB-US | Türkçe-Q / ABD klavye düzeni |
| VM-CLEAN | Temiz Windows VM (kurulum/kaldırma) |

## 2. Faz 0 — Baseline doğrulamaları

| ID | Kontrol | Yöntem | Beklenen | Durum |
|---|---|---|---|---|
| B-01 | `windows/` iskeleti | Dizin ağacı | Belgelenen yerleşim mevcut | ✅ |
| B-02 | Motor iskeleti derlenir | `cargo check` (Linux, saf çekirdek) | Hatasız | ✅ |
| B-03 | Profil şeması geçerli JSON | `python -m json.tool` | Hatasız | ✅ |
| B-04 | Varsayılan profil sözleşmeye uyar | Yapısal kontrol betiği | Zorunlu alanlar ve tipler uyumlu | ✅ |
| B-05 | Anahtar sözlüğü üretildi | `KEY-DICTIONARY.tsv` | 119 kanonik tuş; alias'lar dolu | ✅ |
| B-06 | Linux dokunulmadı | `find linux -newermt` | Çıktı yok | ✅ |
| B-07 | Belgeler tutarlı | Çapraz bağlantı kontrolü | Kırık bağlantı yok | ✅ |

## 3. Motor çekirdeği (Faz 1)

| ID | Senaryo | Beklenen |
|---|---|---|
| E-01 | Her `KEY_*`/`BTN_*` için eşleme | Faz 1'de tablo tam; eksik yok |
| E-02 | Tetikleyici derleme: tek tuş, kombo, alias | Aynı iç `Key` kümesi |
| E-03 | Geçersiz tetikleyici | Makro atlanır + uyarı; çökme yok |
| E-04 | 7 tetikleme kipi | Linux semantiğiyle aynı |
| E-05 | `ACTION_LOCK`, jitter, iptal | Aynı davranış |
| E-06 | Profil sınırları | 32 MiB/500 makro/200k nokta |

### Faz 1 doğrulama sonuçları (10 Kasım 2026)

| ID | Kontrol | Yöntem | Sonuç |
|---|---|---|---|
| B-08 | Biçim | `cargo fmt --all -- --check` | ✅ |
| B-09 | Lint | `cargo clippy --all-targets -- -D warnings` | ✅ 0 uyarı |
| B-10 | Birim testleri | `cargo test --all` | ✅ 57/57 |
| B-11 | Anahtar tablosu üretimi | `python tools/gen_keymap.py --check` | ✅ güncel |
| B-12 | Windows anahtar tablosu kapsamı | 119/119 tuşta VK; farelerde VK yok | ✅ |
| E-01 | Her `KEY_*`/`BTN_*` eşlemesi | `keymap::tests` | ✅ 119 kanonik + tüm alias |
| E-02 | Tetikleyici derleme | `trigger::tests` | ✅ tek/kombo/alias/ön ek |
| E-03 | Geçersiz tetikleyici | `trigger::tests` | ✅ atlanır + yapılı uyarı |
| E-04 | 7 tetikleme kipi | `trigger::tests` | ✅ Linux semantiği |
| E-05 | `ActionGate`, jitter, iptal | `actions::tests` | ✅ |
| E-06 | Sınırlar (200k nokta, dt clamp) | `actions::tests` | ✅ |

> Windows VK/tarama değerleri Faz 1'de **bildirimsel** veridir; Faz 2'de gerçek Windows'ta `MapVirtualKeyW(vk, MAPVK_VK_TO_VSC)` ile karşılaştırılacak ve fark varsa günlüğe yazılacaktır (açık bilinmeyen Q-02 ile birlikte).

### Faz 2 doğrulama sonuçları (10 Kasım 2026)

| ID | Kontrol | Yöntem | Sonuç |
|---|---|---|---|
| B-13 | Windows API katmanı tip denetimi | `cargo check --target x86_64-pc-windows-msvc --all-targets` | ✅ |
| B-14 | Windows hedefi lint | `cargo clippy --target … -- -D warnings` | ✅ 0 uyarı |
| B-15 | Linux biçim + lint | `cargo fmt --check`, `clippy -D warnings` | ✅ |
| B-16 | Birim testleri | `cargo test --all` | ✅ 78/78 |
| B-17 | Ters anahtar eşlemesi üretimi | `python3 tools/gen_keymap.py --check` | ✅ güncel |
| B-18 | Windows testlerini bağlama/çalıştırma | Linux'ta `link.exe` yok | ⚠️ CI `windows-latest` |
| E-07 | Windows VK/tarama → kanonik tuş | `keymap::tests` | ✅ tüm tuşlar + çakışma ayrımı |
| E-08 | Kendi enjeksiyon imzası | Kod: `dwExtraInfo == 0x4154534D` | ☐ gerçek Windows (I-02) |
| I-01…I-13 | Girdi backend'i E2E | TEST-MATRIX §4 | ☐ gerçek Windows/CI |

> Faz 2'de VK tablosunun ters eşlemesi (`from_windows`) ve fare normalizasyonu birim testleriyle doğrulandı; `MapVirtualKeyW` ile işletim sistemi çapraz kontrolü hâlâ gerçek Windows gerektirir.

### Faz 3 doğrulama sonuçları (10 Kasım 2026)

| ID | Kontrol | Yöntem | Sonuç |
|---|---|---|---|
| B-19 | IPC protokolü + durum | `ipc::tests` | ✅ komut, satır birleştirici, bayraklar, JSON |
| B-20 | Günlük dönmesi | `log::tests` | ✅ 1 MB eşiği, UTC damga, satır düzleştirme |
| B-21 | Runtime saf parçalar | `runtime` (Windows'a özgü) | ✅ `load_profile`, `write_runtime_file` derlenir |
| B-22 | `--record-input` şeması | `record::tests` | ✅ `keys`, boş alan atlama |
| B-23 | Linux biçim + lint | `cargo fmt --check`, `clippy -D warnings` | ✅ |
| B-24 | Birim testleri | `cargo test --all` | ✅ 91/91 |
| B-25 | Windows hedefi check + lint | `cargo check/clippy --target …` | ✅ 0 uyarı |
| L-01 | `ping/reload/stop/status` | `ipc::tests` + CI E2E | ✅ protokol; ☐ gerçek Windows |
| L-02 | GUI çökmesi (`--parent-pid`) | CI E2E | ☐ gerçek Windows |
| L-03 | İkinci motor örneği | Mutex → çıkış 7 | ☐ gerçek Windows |
| L-04 | Yanlış kanal token'ı | `CreateFileW` hatası | ☐ gerçek Windows |
| L-05 | Profil değişikliği | `reload` testi | ✅ birim; ☐ E2E |
| L-06 | `--record-input` geçerli JSON | `record::tests` | ✅ şema; ☐ E2E |

## 4. Girdi backend'i (Faz 2)

| ID | Senaryo | Beklenen |
|---|---|---|
| I-01 | Notepad'de tetikleme + enjeksiyon | Karakterler doğru |
| I-02 | Kendi enjeksiyonunu yok sayma | Geri besleme yok |
| I-03 | STOP tuşu | Tüm eylemler durur, tuşlar bırakılır |
| I-04 | Basılı tuşta çıkış/çökme | Takılı tuş kalmaz |
| I-05 | Auto-repeat | Tetikleyici yeniden ateşlenmez |
| I-06 | Yükseltilmiş ön plan | Kanca olay almaz; rozet görünür ⚠️ |
| I-07 | MacroStat yönetici olarak | Yönetici hedefte çalışır |
| I-08 | UAC istemi / kilit ekranı | Kanca sessiz; eylem durur ⚠️ |
| I-09 | Unicode metin | Türkçe/emoji doğru |
| I-10 | `MOUSE_PATH` çoklu monitör | Negatif koordinatlar doğru |
| I-11 | DPI %200 | Fiziksel piksel doğru |
| I-12 | TR-Q / US düzeni | Tarama kodu aynı karakteri üretir |
| I-13 | Metin reddeden uygulama | Belgelenir; pano yedeği isteğe bağlı |

## 5. Yaşam döngüsü / IPC (Faz 3)

| ID | Senaryo | Beklenen |
|---|---|---|
| L-01 | `ping/reload/stop/status` | Doğru yanıt; bilinmeyen komut reddedilir |
| L-02 | GUI çökmesi | Motor temiz çıkar; tuş bırakır |
| L-03 | İkinci motor örneği | Reddedilir |
| L-04 | Kanal token'ı yanlış | Bağlantı reddedilir |
| L-05 | Profil değişikliği | `reload` ile uygulanır |
| L-06 | `--record-input` | Geçerli JSON; sınırlar uygulanır |

## 6. GUI / paketleme / güvenlik (Faz 4–7)

| ID | Senaryo | Beklenen |
|---|---|---|
| G-01 | Tüm sayfalar + TR/EN | Parite |
| G-02 | Profil içe/dışa | Şema korunur |
| G-03 | Onboarding (yönetici izni yok) | Kanca testi + bilgi |
| P-01 | Temiz VM kurulum/açılış/kaldırma | Kalıntı yok |
| P-02 | İmzasız SmartScreen | Belgelenir |
| S-01 | ACL/kanal/DLL/günlük | Kontrol listesi geçer |
| S-02 | Bağımlılık denetimi | Bilinen kritik açık yok |

### 6.1 Faz 4 — GUI kod düzeyi sonuçları (10 Kasım 2026)

| ID | Durum | Kanıt |
|---|---|---|
| G-01 | ✅ kod / ⏳ VM | 4 sayfa başsız (offscreen) kuruldu; Faz 4 tarihinde 45 birim testi (güncel: 55); çeviri kapsam testi (sabit `T()` metinlerinin %100'ü `_EN`'de) |
| G-02 | ✅ kod / ⏳ VM | `transfer` modülü Linux ile aynı; profil sözleşmesi `schemaVersion 1`; `default_profile.json` bayt kopyası |
| G-03 | ✅ kod / ⏳ VM | Sihirbaz yönetici istemez; 5 kontrol (motor, veri klasörü, profil, Job Object, yetki) ve Windows notları sayfası; başsız kuruldu |
| G-04 | ✅ kod | Tek örnek: `QLockFile` (`%LOCALAPPDATA%\MacroStat\macrostat-gui.lock`); ikinci `tryLock` başarısız |
| G-05 | ✅ kod | DPI farkındalığı `QApplication`'dan önce; koyu başlık çubuğu `DwmSetWindowAttribute(20)` |
| G-06 | ✅ kod | Günlük paneli döndürme algısı `st_ino` yoksa `st_mtime_ns` gerilemesine düşer |

⏳ = gerçek Windows VM/CI doğrulaması Faz 6'da yapılacak.

### 6.2 Faz 5 — paketleme kod düzeyi sonuçları (10 Kasım 2026)

| ID | Durum | Kanıt |
|---|---|---|
| P-01 | ⏳ VM | Kurulum sözleşmesi hazır: `PrivilegesRequired=lowest`, `%LOCALAPPDATA%\Programs\MacroStat`, kaldırmada veri için varsayılan "Hayır". Sacayağı `build.ps1` + `MacroStat.iss` + `verify.ps1`; gerçek kurulum Faz 6. |
| P-02 | ✅ belge / ⏳ VM | İmza opsiyonel ve varsayılan kapalı; `docs/CODE-SIGNING.md` + kurulum README'sinde SmartScreen notu. Gerçek uyarı ekranı Faz 6'da fotoğraflanacak. |
| P-03 | ✅ kod | PyInstaller onedir spec'i motor ve `assets/` yollarını `_internal/` altına yerleştirir; motor yoksa derleme bilinçli olarak durur (`common.find_engine()` adaylarıyla uyumlu). |
| P-04 | ✅ kod | Sürüm tek kaynak: `windows/VERSION` = `engine/Cargo.toml` = `version_info.txt`; `build.ps1` uyumsuzlukta durur. |
| P-05 | ✅ kod | `SHA256SUMS.txt` üretimi ve `verify.ps1` doğrulaması aynı biçimi (küçük harf `hash  ad`) kullanır. |
| P-06 | ✅ kod | CI: `core`, `gui-tests`, `build`, `sign` işleri; imza yalnızca `SIGNING_ENABLED` + korumalı ortamla. YAML ayrıştırma doğrulandı. |
| P-07 | ✅ yerel / ⏳ CI | GUI `py_compile` + Faz 5 tarihinde 45/45 test yerelde geçti (güncel: 55); gerçek cargo/PyInstaller/ISCC Faz 6'da `windows-latest` üzerinde. |

### 6.3 Faz 6 — güvenlik kod düzeyi sonuçları (11 Kasım 2026)

| ID | Durum | Kanıt |
|---|---|---|
| S-01 | ✅ kod / ⏳ VM | `SetDefaultDllDirectories`; kanalda `FILE_FLAG_FIRST_PIPE_INSTANCE` + `SECURITY_IDENTIFICATION`; `runtime.json` korumalı DACL + PID/oluşturma zamanı doğrulaması; günlükte denetim karakteri temizliği; `verify.ps1` ikiliyi yalnızca denetimler temizse çalıştırır. Ayrıntı: [`reports/06-security-review.md`](../reports/06-security-review.md) |
| S-02 | ⏳ CI | Kaynak düzeyinde bilinen kritik açık yok; `cargo audit` ve `pip-audit` çevrimdışı ortamda çalıştırılamadı, CI'ya eklenecek. Actions etiketlerinin SHA sabitlemesi de aynı adımda yapılacak. |
| S-03 | ✅ kod | Dolu kuyrukta panik tuşu: kanca geri çağrısı atomik mandal kurar (`hook::take_panic_press`), motor her turda tüketir. |
| S-04 | ✅ kod | Profil sınırları: `sanitize_limits` + iki regresyon testi (kırpma, tekerlek büyüklüğünün korunması). |
| S-05 | ✅ kod / ⏳ VM | Reparse point reddi: ayarlar, profil, `runtime.json` ve kayıt yazımı. Linux'ta sembolik bağ testi geçti. |

Faz 6 tarihindeki test toplamı: Rust **93/93**, Python **47/47** (güncel toplam
için sayfa başındaki nota bakın).

### 6.4 Faz 7 — uçtan uca (E2E) kabul senaryoları

Koşucu: `windows/tests/test_e2e_windows.py` (yalnızca Windows; Linux'ta atlanır).
CI işi: `e2e` (`ci/windows.yml`), `build` artefaktını indirir ve gerçek
`macrostat-engine.exe` üzerinde çalışır.

| ID | Senaryo | Beklenen |
|---|---|---|
| U-01 | `--version`, geçersiz profil | Sürüm yazılır; geçersiz profil çıkış kodu 2 |
| U-02 | Motor + `runtime.json` + `ping/status` | PID ve kanal adı eşleşir; JSON yanıtı `ok` |
| U-03 | Profil değişikliği + `reload` | Günlükte "Profil yeniden yüklendi" |
| U-04 | İkinci motor örneği | Çıkış kodu 7; ilk motor çalışmaya devam eder |
| U-05 | Panik tuşu (F24, enjekte) | Motor 0 ile kapanır; "Acil durdurma tetiklendi"; `runtime.json` silinir |
| U-06 | `--ipc stop` | Motor 0 ile kapanır; "Motor durdu" |
| U-07 | `--test-macro` (yalnızca DELAY) | Çıkış kodu 0; `[TEST]` satırı |
| U-08 | `--record-input` turu | Geçerli JSON; `KEY_A` basışı kaydedilmiş |
| U-09 | `--self-test` | Kanca, imza yalıtımı, dış olay, panik mandalı, takılı tuş; çıkış 0 |

Not: U-05, U-08 ve U-09 gerçek girdi enjeksiyonu gerektirir. Etkileşimli olmayan
bir CI oturumunda kanca olay alamazsa `MACROSTAT_E2E_SKIP_INPUT=1` ile bu üç
senaryo atlanır ve yalnızca yaşam döngüsü senaryoları koşulur; atlama test
çıktısında `skipped` olarak görünür ve "geçti" sayılmaz.

### 6.5 Faz 8 — sürüm kontrolü

| Kontrol | Durum |
|---|---|
| Sürüm tek kaynak (`VERSION` = `Cargo.toml` = `version_info.txt` = `.iss`) | ✅ `ci/preflight.py` |
| CI iş akışı bütünlüğü (dosyalar, işler, imza koşulu) | ✅ `ci/preflight.py` |
| Yerel test toplamı | ✅ Rust 99 + GUI 55 (14 Kasım 2026) |
| Windows üretimi ve VM kabulü | ⏳ `reports/08-final-report.md` §4 |

## 7. Faz 0'da doğrulanacak bilinmeyenler (Faz 2'ye taşındı)

| ID | Soru | Ölçüm |
|---|---|---|
| Q-01 | Yükseltilmiş ön planda `WH_KEYBOARD_LL` olay alır mı? | Beklenti: hayır |
| Q-02 | `RIDEV_INPUTSINK` yükseltilmiş ön planda olay alır mı? | Belgede yok; ölçülecek |
| Q-03 | Bütünlük düzeyi önsezisi güvenilir mi? | Yanlış pozitif oranı |
| Q-04 | `SendInput` UIPI engelinde sayaç yanıltıcı mı? | Belge: evet; saha doğrulaması |
| Q-05 | 1 ms timer altında jitter | 1000 olaylık ölçüm |
