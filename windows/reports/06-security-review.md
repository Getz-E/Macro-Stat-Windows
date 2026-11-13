# Faz 6 — Güvenlik İncelemesi ve Tehdit Modeli (11 Kasım 2026)

Bu rapor Faz 6 kapsamında `windows/` altındaki motor, GUI, IPC, kayıt, günlük,
paketleme ve CI zincirinin güvenlik incelemesini ve bulgulara uygulanan
düzeltmeleri kaydeder. Ana kurala uyulmuştur: `linux/` altına **hiçbir
değişiklik yapılmadı**; tüm düzeltmeler `windows/` altındadır.

## 1. Kapsam ve yöntem

İnceleme üç bağımsız geçişte yapıldı; her bulgu dosya/satır düzeyinde
doğrulandı, ardından düzeltme uygulandı ve testlerle korunur hâle getirildi.

| Geçiş | Kapsam | Yaklaşım |
|---|---|---|
| Girdi yolu | `hook.rs`, `inject.rs`, `actions.rs`, `trigger.rs`, `record.rs`, `runtime.rs` | Kanca geri çağrısı, kuyruk taşması, iş parçacığı yaşam döngüsü, profil sınırları |
| IPC / yaşam döngüsü | `ipc.rs`, `runtime.rs`, `main.rs`, `winutil.rs`, `log.rs` | Kanal DACL'i, ilk örnek, istemci kimliği, `runtime.json`, temiz kapanış |
| Bağımlılık / paketleme | `ci/windows.yml`, `windows/scripts/*.ps1`, `Cargo.lock`, `requirements-win.txt` | Tedarik zinciri, imza işi, doğrulama sırası, araç sürümleri |

Doğrulama ortamı: çevrimdışı Linux (ağ yok, Windows hedefi kurulu değil).
Bu nedenle:

- `cfg(windows)` kodu derlenemedi; her FFI çağrısı yerel `windows-sys 0.61.2`
  kaynağındaki imza ve dönüş sözleşmesiyle karşılaştırıldı.
- Linux `cargo check --locked --offline` ve `cargo test` ile platformdan
  bağımsız mantık çalıştırıldı.
- `cargo audit`, `pip-audit`, gerçek kurulum ve SmartScreen denemesi bu
  ortamda yapılamaz; CI/VM adımı olarak işaretlendi.

## 2. Özet

| Önem | Bulgu | Durum |
|---|---|---|
| Yüksek | Panik tuşu dolu kuyrukta düşebiliyordu | **Düzeltildi** — kancada atomik mandal |
| Yüksek | Profil sınırları (metin/gecikme/nokta) uygulanmıyordu | **Düzeltildi** — `sanitize_limits` |
| Yüksek | Kanal `INVALID_HANDLE_VALUE` testi fail-open idi | **Düzeltildi** — doğru hata testi |
| Yüksek | İlk örnek bayrağı yoktu; sahte sunucu yarışı | **Düzeltildi** — `FILE_FLAG_FIRST_PIPE_INSTANCE`, örnek sayısı 1 |
| Yüksek | İstemci varsayılan kimliğe bürünme düzeyiyle bağlanıyordu | **Düzeltildi** — `SECURITY_IDENTIFICATION` |
| Yüksek | `runtime.json` ACL'siz, sembolik bağa açık, sınırsız okunuyordu | **Düzeltildi** — korumalı DACL, reparse reddi, 64 KiB sınırı, PID/oluşturma zamanı doğrulaması |
| Yüksek | CI imza işi pull request'te çalışabilirdi | **Düzeltildi** — olay türü koşulu |
| Orta | Gecikme toplamı taşabiliyordu | **Düzeltildi** — doygun aritmetik |
| Orta | `TEXT` karakteri günlüğe düşebiliyordu | **Düzeltildi** — içeriksiz uyarı |
| Orta | Kayıt durdurma tuşu fare düğmelerini kapsamıyordu | **Düzeltildi** — `keys_are_equivalent` |
| Orta | Kayıt dosyası olay sayısı sınırsızdı | **Düzeltildi** — 50 000 geçiş sınırı |
| Orta | Günlüğe denetim karakteri enjeksiyonu | **Düzeltildi** — `sanitize` |
| Orta | `verify.ps1` imzayı doğrulamadan ikiliyi çalıştırıyordu | **Düzeltildi** — çalıştırma en sona alındı |
| Orta | Hizalama varsayımıyla UB riski (`TOKEN_*` yapıları) | **Düzeltildi** — `read_unaligned` + boyut kontrolü |
| Orta | Kapanışta işçi enjeksiyonu bırakmayı yarışabiliyordu | **Düzeltildi** — kapı + çift `release_all` |

## 3. Yüksek önemli bulgular ve çözümler

### H1 — Panik tuşu dolu kuyrukta yutuluyordu

`hook.rs` geri çağrısı `try_send` kullanır ve kuyruk dolduğunda olayı
düşürür. Panik tuşu tek basışlık olduğundan (Pause otomatik tekrarlamaz),
düşen olay motoru hiç durduramazdı.

Çözüm: geri çağrı, olayı kuyruğa vermeden **önce** panik tuşunu
`PANIC_PRESSED: AtomicBool` içine mandallar. `hook::set_panic_key` motor
başlangıcında ve her profil yeniden yüklemesinde çağrılır; ana döngü her
turda `take_panic_press()` ile mandalı tüketir ve normal `TriggerCommand::Panic`
yoluyla aynı kapanışı yapar. Klavye ve fare düğmesi yollarının ikisi de
mandallar (`keyboard_proc`, `mouse_proc`).

### H2 — Profil sınırlarının uygulanmaması

Sözleşmedeki `32 MiB`, `64 KiB TEXT`, `200 000 nokta` ve zaman aşımı
sınırları belgelenmiş ama uygulanmıyordu; 32 MiB belge içinde milyonlarca
eylem veya taşan gecikme değerleri taşınabiliyordu.

Çözüm: `profile::sanitize_limits` hem sarmalayıcı hem tek profil yolunda
çağrılır. Kimlik/ad/tetikleyici adları karakter bazında, `TEXT` bayt
bazında (UTF-8 sınırına saygılı) kırpılır; makro (500) ve eylem (2000)
sayıları sınırlanır; `repeatCount`, `doubleTapTimeoutMs`,
`holdDurationMs`, `delayMs`, `jitterMs`, `dx/dy`, `MOUSE_PATH` noktaları
ve `start` koordinatları kırpılır. Tekerlek değeri **işareti ve büyüklüğü
korunarak** ±120 kademeye sınırlanır (önceki `signum()` yaklaşımı çok
kademeli kaydırmayı bozuyordu).

### H3/H4/H5 — Kanal oluşturma, ilk örnek ve istemci kimliği

- Sunucu başarısızlığı yalnızca `is_null()` ile sınıyordu; Win32 bu çağrıda
  `INVALID_HANDLE_VALUE` döndürür. Hatalı test, kanalın "kuruldu" sayılıp
  kontrol yüzeyinin ölü kalmasına yol açabilirdi. Artık iki değer de
  reddediliyor.
- `FILE_FLAG_FIRST_PIPE_INSTANCE` ve örnek sayısı `1` ile adı önceden
  kapmış sahte sunucuya bağlanma yarışı kapatıldı.
- İstemci `SECURITY_IDENTIFICATION | SECURITY_SQOS_PRESENT` ile bağlanır:
  sahte sunucu istemcinin kimliğine bürünemez. Sunucu meşgulse
  `WaitNamedPipeW` + kısa bekleme ile en fazla 5 deneme yapılır.

### H6 — `runtime.json` bütünlüğü ve kimliği

Dosya reparse point reddi, benzersiz `create_new` geçici dosya, `sync_all`,
atomik yeniden adlandırma ve `D:P(A;;FA;;;<SID>)` korumalı DACL ile yazılır.
Okuma 64 KiB ile sınırlıdır. `validate_runtime_info` şunları doğrular:

- SID geçerli kullanıcıyla aynı (büyük/küçük harf duyarsız),
- kanal adı `\\.\pipe\macrostat-<SID>-` önekiyle başlar,
- PID sıfırdan farklı ve süreç gerçekten yaşıyor,
- saklanan oluşturma zamanı `process_creation_time(pid)` ile birebir aynı.

Böylece geri dönüştürülmüş PID veya elle yazılmış dosya ile başka sürece
komut gönderilemez.

### H7 — CI imza işi

`sign` işi yalnızca `push` veya `workflow_dispatch` olayında ve
`SIGNING_ENABLED=true` iken çalışır; pull request'te asla çalışmaz. Checkout
`persist-credentials: false` ile yapılır. Böylece çatal PR'ının denetlenmemiş
ağacı imzalama sırlarına erişemez.

## 4. Orta ve düşük bulgular

| # | Bulgu | Düzeltme |
|---|---|---|
| M1 | `delay_ms + jitter` toplamı `i64` taşmasına açıktı | `try_from` + `saturating_add` + `u64::try_from` |
| M2 | `TEXT` enjeksiyon hatası karakteri günlüğe yazıyordu | Uyarı içeriksiz: "bir metin karakteri enjekte edilemedi" |
| M3 | `--record-input` durdurma tuşu fare düğmelerini yok sayıyordu | `Key` ve `MouseButton` aynı `keys_are_equivalent` kuralıyla |
| M4 | Kayıt belgesi 600 sn içinde sınırsız büyüyebiliyordu | `MAX_RECORD_EVENTS = 50 000` |
| M5 | Kayıt dosyası sembolik bağa yazılabiliyordu | `reject_link` + benzersiz geçici + atomik rename |
| M6 | Günlük satırına `\r`, denetim karakteri enjeksiyonu | Satır sonu/sekme boşluk, diğer denetim karakterleri `?` |
| M7 | `verify.ps1` hash/imza denetiminden önce ikiliyi çalıştırıyordu | Sıra: dosyalar → SHA-256 → Authenticode → (yalnızca temizse) duman testi |
| M8 | `TOKEN_MANDATORY_LABEL`/`TOKEN_USER` hizasız `Vec<u8>` üzerinden okunuyordu | `read_unaligned` + `size_of` kontrolü |
| M9 | Kapanışta işçi, `release_all` sonrası tuş basabiliyordu | Kapı kilidi altında bırakma + 50 ms sonra ikinci bırakma |
| M10 | DLL arama sırası CWD/PATH içerebiliyordu | `SetDefaultDllDirectories` `main` başında |
| L1 | Günlük paneli bozuk UTF-8'de bayt konumunu kaydırıyordu | Bayt tabanlı ofset, `errors="replace"`, satır kırpma |
| L2 | Ayarlar dosyası sınırsız okunuyordu | 64 KiB sınırı + reparse reddi |
| L3 | Profil okuma TOCTOU'ya açıktı | Okuma tek dosya tanıtıcısı üzerinden, boyut sınırlı |

## 5. Kabul edilen artık riskler

| Risk | Neden kabul edildi |
|---|---|
| Aynı kullanıcıdaki zararlı süreç kancayı okuyabilir | Windows mimari sınırı; yükseltme veya sürücü kapsam dışı |
| Enjeksiyon imzası (`0x4154_534D`) taklit edilebilir | Saldırgan zaten aynı kullanıcıda girdi enjekte edebiliyor; imza yalnızca kendi çıktımızın döngüye girmesini engeller |
| `panic = "abort"` süreç çöküşünde bırakma garantisi vermez | Tuşlar işletim sistemi tarafından da bırakılır; panik tuşu ve normal kapanış önceliklidir |
| `TEXT` ile yazılan metin hedefe gider | Tasarım gereği; komut çalıştırma eylemi yoktur |
| İmzasız dağıtım SmartScreen uyarısı | Sertifika temini dış bağımlılık; imza yolu hazır |
| GitHub Actions etiketleri SHA'ya sabitlenmedi | Çevrimdışı ortamda doğrulanmış SHA alınamadı; CI'da ağ erişimi olan bir adımda sabitlenecek (bkz. §7) |
| `cargo audit` / `pip-audit` çalıştırılmadı | Çevrimdışı; CI'da çalıştırılacak |

## 6. Test kanıtı

| Komut | Sonuç |
|---|---|
| `cargo check --locked --offline` | Uyarısız |
| `cargo test --locked --offline` | **93/93 geçti** |
| `python -m unittest discover -s windows/tests` (`QT_QPA_PLATFORM=offscreen`) | **47/47 geçti** (Faz 7'de 6 Windows E2E senaryosu eklendi; toplam keşif 54, 7 atlandı) |

Yeni regresyon testleri:

- `profile::tests::sanitizer_clamps_timings_and_preserves_scroll_magnitude`
- `profile::tests::sanitizer_truncates_oversized_collections_and_names`
- `SettingsTests.test_oversized_file_is_not_read`
- `SettingsTests.test_reparse_point_is_refused`

## 7. Windows VM / CI'da yapılacaklar

1. `cargo clippy --all-targets -- -D warnings` ve `cargo test` Windows'ta.
2. `cargo audit` ve `pip-audit -r windows/gui/requirements-win.txt`.
3. ~~GitHub Actions `uses:` satırlarını doğrulanmış commit SHA'larına sabitle.~~
   **Yapıldı** (bkz. `reports/10-ci-hardening.md`); `preflight.py` artık SHA
   dışındaki sabitlemelerde başarısız oluyor.
4. Gerçek kanal DACL testi: farklı kullanıcı bağlanamamalı; ikinci motor reddedilmeli.
5. Dolu kuyrukta panik tuşu testi (yapay olay seli + Pause).
6. Yükseltilmiş hedef ve güvenli masaüstü negatif testleri.
7. Kurulum/kaldırma ve SmartScreen görsel kanıtı (P-01, P-02).
