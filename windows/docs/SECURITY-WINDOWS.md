# Windows Güvenlik Modeli — Baseline

Bu belge Linux `SECURITY.md` tehdit modelinin Windows karşılığını ve Windows'a özgü ek yüzeyleri tanımlar. Faz 6'da ayrıntılı inceleme ve test kanıtlarıyla güncellenir.

## 1. Varsayımlar

- Uygulama **kullanıcı oturumunda**, `asInvoker` olarak çalışır; yönetici izni istemez.
- Kurulum kullanıcı bazlıdır; `Program Files` yazma izni gerekmez.
- Ağ erişimi yoktur; telemetri, otomatik güncelleme, uzak kanal yoktur.
- Çekirdek sürücüsü yüklenmez; `uiAccess` kullanılmaz.
- Profil dosyası kullanıcı tarafından düzenlenebilir kabul edilir (güvenilmeyen girdi).

## 2. Varlıklar

| Varlık | Gizlilik | Bütünlük |
|---|---|---|
| Profil (makrolar, tetikleyiciler) | Düşük | **Yüksek** — kötü niyetli düzenleme tuş enjeksiyonuna dönüşebilir |
| Kanal token'ı (`runtime.json`) | **Yüksek** | Yüksek — ele geçerse motor kontrol edilir |
| Günlükler | Orta — tuş içeriği yazılmaz | Orta |
| Motor/GUI ikilileri | — | **Yüksek** — DLL/EXE değiştirilirse kod çalıştırma |

## 3. Tehditler ve önlemler

| Tehdit | Etki | Önlem | Durum |
|---|---|---|---|
| Aynı kullanıcıdaki kötü niyetli süreç kancayı okur/enjekte eder | Girdi gözetleme | Windows'ta aynı oturumda engellenemez; belgelenir; ek ayrıcalık iddiası yok | Kabul |
| Kötü niyetli profil (`TEXT` ile komut, devasa alan) | İstenmeyen eylem / DoS | Boyut sınırları (32 MiB, 200k nokta, 64 KiB metin), kimlik doğrulama, `TEXT` yalnızca yazı; komut çalıştırma eylemi yok | Var |
| Sahte kanal sunucusu / istemcisi | Motor kontrolü | Rastgele token + kullanıcı SID DACL + `PIPE_REJECT_REMOTE_CLIENTS`; `FILE_FLAG_FIRST_PIPE_INSTANCE`; istemcide `SECURITY_IDENTIFICATION`; PID/oluşturma zamanı doğrulaması | Var |
| Panik tuşunun dolu kuyrukta yutulması | Motor durdurulamaz | Kanca geri çağrısında atomik mandal (`take_panic_press`) | Var |
| Kötü niyetli profil sınır taşması | DoS / istenmeyen eylem | `sanitize_limits`: 500 makro, 2000 eylem, kırpılmış metin/süre/koordinat | Var |
| DLL arama sırası istismarı | Kod çalıştırma | `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 \| LOAD_LIBRARY_SEARCH_APPLICATION_DIR)`, tam yol `LoadLibraryEx`, PATH araması yok | Faz 2 |
| Profil/ayar dosyasına sembolik bağ/reparse point | Yetkisiz yazma | Reparse point reddi (symlink **ve** junction: `FILE_ATTRIBUTE_REPARSE_POINT`); atomik yazma; `runtime.json` için korumalı DACL | Var |
| Günlükten bilgi sızması | Gizlilik | Tuş içeriği loglanmaz; yalnızca olay adı/hata; kullanıcı ACL'i | Faz 3 |
| Yükseltilmiş hedefe enjeksiyon denemesi | Sessiz başarısızlık | UIPI farkındalığı: bütünlük düzeyi önsezisi + rozet + günlük; engel `GetLastError` ile algılanamaz | Faz 2 |
| Güvenli masaüstünde çalışma beklentisi | Yanlış güvenlik beklentisi | Belgelenir; UAC/kilit ekranında eylem durur, tuşlar bırakılır | Faz 2 |
| Anti-cheat/AV false positive | Kullanım engeli | İmza (sonraki sürüm), onedir, UPX yok, Defender'a gönderim, SHA-256, kaynak bağlantısı | Faz 5 |
| Kötü niyetli kurulum paketi | Kod çalıştırma | İmza (opsiyonel), SHA-256, kullanıcı bazlı kurulum, yalnızca kendi dosyalarını silen kaldırma | Faz 5 |
| `uiAccess`/hizmet ile yetki yükseltme | Sistem bütünlüğü | Bu sürümde **yok**; bilinçli karar | Kabul |
| Çekirdek sürücüsü talebi | Sistem bütünlüğü | Kapsam dışı; kurulmaz | Kabul |

## 4. Açıkça yapılmayacaklar

- Yönetici pencerelere gizlice enjeksiyon (UIPI aşma girişimi).
- Güvenli masaüstünde etkileşim veya `SendSAS` ile SAS benzetimi.
- Anti-cheat/EDR tespitini atlatma, kendini gizleme, süreç enjeksiyonu.
- Çekirdek sürücüsü, hizmet, `uiAccess`.
- Ağ üzerinden kontrol, güncelleme, telemetri.

## 5. En az yetki kontrol listesi

- [x] Manifest: `requestedExecutionLevel level="asInvoker" uiAccess="false"`
- [x] Motor yolu GUI yanında sabit; PATH araması yok
- [x] Kanal: SID DACL, yerel, `PIPE_REJECT_REMOTE_CLIENTS`, ilk örnek bayrağı, istemci kimliği koruması
- [x] Dosya: kullanıcı ACL, atomik yazma, reparse point reddi, boyut sınırı
- [x] Günlük: 1 MB rotasyon, hassas veri yok, denetim karakteri temizliği
- [x] DLL: güvenli arama dizinleri (`SetDefaultDllDirectories`)
- [x] Yetim süreç yok: Job Object + ebeveyn izleme
- [x] Takılı tuş temizliği: STOP, kapanış, kuyruk taşmasında panik mandalı
- [ ] Gerçek VM/CI doğrulaması (kanal ACL, DLL arama, yükseltilmiş hedef)

## 6. Doğrulama

Faz 6 kod düzeyi sonucu: [`reports/06-security-review.md`](../reports/06-security-review.md).
Açık kalanlar: `cargo audit` / `pip-audit`, GitHub Actions SHA sabitlemesi,
gerçek kanal ACL testi, dolu kuyrukta panik testi, yükseltilmiş/güvenli masaüstü
negatif testleri, kurulum/SmartScreen görsel kanıtı.
