# Adım Raporu — Faz 3 IPC, Yaşam Döngüsü ve Günlük

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


| | |
|---|---|
| **Tarih** | 10 Kasım 2026 |
| **Faz** | 3 — IPC, yaşam döngüsü, günlük, uzun süreli motor |
| **Durum** | Kod tamamlandı; gerçek Windows E2E doğrulaması bekliyor |
| **Kapsam** | `ipc`, `log`, `runtime`, `winutil` (tek örnek/SID/runtime), `record` (`--record-input`), `trigger` (runtime geri çağrıları), `main` (CLI + motor girişi) |
| **Kabul ölçütü** | `reload`/`stop`/`status`; Job Object; `--record-input` çıktısı geçerli |

## 1. Özet

Faz 3'ün kabul ölçütü karşılandı: motor artık Linux'taki gibi **uzun süreli** çalışıyor; kancadan gelen olayları tetikleyici motoruna besliyor, her tetikleme için kısa ömürlü iş parçacıkları açıyor, adlandırılmış kanal üzerinden `ping`/`reload`/`stop`/`status` komutlarını kabul ediyor, `%LOCALAPPDATA%\MacroStat\runtime.json` ile kendini GUI'ye tanıtıyor ve temiz kapanışta enjekte ettiği tüm tuşları bırakıyor.

`--record-input` artık fare hareketi + fare düğmeleri + **sistem geneli klavye** olaylarını Linux ile aynı `MOUSE_PATH` belgesine yazıyor; `--record-mouse` çıktısı değişmedi (`keys` alanı boşken belgeden tamamen çıkarılır, böylece Linux dosyalarıyla bayt uyumu korunur).

**Sonuçlar:** Linux `fmt` ✅ · Linux `clippy -D warnings` ✅ · `cargo test --all` ✅ **91/91** · üretici `--check` ✅ · Windows hedefi `cargo check --all-targets` ✅ · Windows hedefi `clippy -D warnings` ✅ · `linux/` dokunulmadı ✅.

> Kanca, `SendInput` ve adlandırılmış kanal davranışı bu ortamda (Linux) çalıştırılamaz; bu rapordaki "doğrulandı" ifadeleri aksi belirtilmedikçe derleme/statik doğrulama ve saf protokol testlerini kasteder. Gerçek Windows doğrulaması CI `windows-latest` + TEST-MATRIX §5'te yapılacaktır.

## 2. Üretilen/değişen dosyalar

| Dosya | İçerik |
|---|---|
| `engine/src/ipc.rs` | Protokol (`ping/reload/stop/status`), satır birleştirici, `ControlState`, `StatusSnapshot`, SID DACL'li sunucu, `--ipc` istemcisi |
| `engine/src/log.rs` | 1 MB'ta dönen, kullanıcı dizininde, satır başına UTC zaman damgalı günlük |
| `engine/src/runtime.rs` | **Yeni**: uzun süreli döngü, tetikleyici dağıtımı, işçi iş parçacıkları, reload/stop/status, ebeveyn izleyici, `runtime.json` |
| `engine/src/winutil.rs` | `data_dir`, `current_user_sid_string`, `process_creation_time`, `wait_for_process_exit`, `SingleInstance` (adlandırılmış mutex) |
| `engine/src/record.rs` | `keys` alanı, `on_key`, `record_keys` parametresi, `RecordSummary.keys`, 2 yeni test |
| `engine/src/trigger.rs` | `while_held_finished`, `running_loops` (runtime durumu için) |
| `engine/src/main.rs` | `--ipc`, `--log`, `--parent-pid`, `--watch-restart`, `--record-input`; tek örnek, runtime.json, çıkış kodları |
| `engine/src/lib.rs` | `#[cfg(windows)] mod runtime;` |
| `engine/Cargo.toml` | `Win32_System_IO` feature'ı (`ReadFile`/`WriteFile`/`ConnectNamedPipe` için) |
| `reports/03-ipc-lifecycle.md` | Bu rapor |
| `docs/TEST-MATRIX.md` | Faz 3 sonuçları |
| `README.md` | Durum Faz 3, 91 test |

## 3. Süreç ve iş parçacığı modeli

```
GUI / kullanıcı
   │  (isteğe bağlı) Job Object KILL_ON_JOB_CLOSE
   ▼
macrostat-engine.exe  (asInvoker)
   ├─ ana iş parçacığı: TriggerEngine + kanca kuyruğu + done kanalı
   ├─ macrostat-ipc      : adlandırılmış kanal sunucusu (ControlState)
   ├─ macrostat-parent-watch : --parent-pid çıkışını bekler
   └─ işçi iş parçacıkları: macrostat-once-*, macrostat-loop-*, macrostat-held-*, macrostat-hold
```

- Ana iş parçacığı **hiçbir zaman** eylem dizisi çalıştırmaz; kanca kuyruğunu 20 ms'lik `recv_timeout` ile boşaltır ve `reload`/`stop` bayraklarını bu sınır içinde görür.
- Her tetikleme kısa ömürlü bir işçi açar; `ActionGate` eylem dizilerini sıraya koyar, böylece iki işçi `KEY_DOWN`/`KEY_UP` durumunu iç içe geçiremez.
- İşçiler enjekte ettikleri tuşları paylaşılan `SharedInjector` üzerinden yansıtır; panik/`stop`/`reload`/çıkış herhangi bir iş parçacığından **tüm** tuşları bırakabilir.
- Linux paritesi: tek seferlik (`ONCE`, `DOUBLE_TAP`, `HOLD`, `PRESS_RELEASE`) işçiler çıkışta tuş bırakmaz; döngüler ve `WHILE_HELD` çıkışta bırakır; `StopLoop` tüm tuşları bırakır; panik tümünü bırakıp motoru kapatır.

## 4. IPC sözleşmesi

### 4.1 Kanal

| Öğe | Değer |
|---|---|
| Ad | `\\.\pipe\macrostat-<SID>-<128-bit rastgele hex token>` |
| Erişim | `PIPE_ACCESS_DUPLEX`, `PIPE_TYPE_BYTE`, `PIPE_WAIT`, `PIPE_REJECT_REMOTE_CLIENTS` |
| DACL | `D:P(A;;GA;;;<SID>)` — yalnızca geçerli kullanıcı |
| Tampon | 8 KiB giriş/çıkış |
| İstek sınırı | 4.096 bayt/satır; aşan bağlantı düşürülür |
| Boşta kalma | 5 sn yanıt yoksa istemci atılır |
| Hata politikası | SID alınamaz / DACL kurulamaz / kanal açılamaz → motor **başlamaz** (fail-closed) |

### 4.2 Mesajlar

Satır sonu ayrımlı JSON:

```json
{"cmd":"ping"}      → {"ok":true,"cmd":"ping"}
{"cmd":"status"}    → {"ok":true,"cmd":"status","pid":…,"uptimeMs":…,"pipe":…,"log":…,
                       "elevated":false,"profile":…,"macros":…,"triggers":…,"loops":…,
                       "hookDropped":…,"injectedAttempts":…,"injectedSent":…,"injectedShortfalls":…}
{"cmd":"reload"}    → {"ok":true,"cmd":"reload"}   (ana döngü bayrağı tüketir)
{"cmd":"stop"}      → {"ok":true,"cmd":"stop"}      (yanıttan sonra bağlantı kapanır)
{"cmd":"x"}         → {"ok":false,"error":"unknown command"}
```

`cmd` alanının `command` takma adı vardır. CLI: `macrostat-engine.exe --ipc status` (runtime.json'dan kanalı okur, yanıtı basar; başarısızlıkta çıkış 6).

### 4.3 Durum anlık görüntüsü

Ana döngü durumu 250 ms'de bir tazeler; `status` yanıtı bu önbelleği okur, böylece IPC iş parçacığı hiçbir zaman motor mutex'ini beklemez. Sayaçlar: kanca kuyruğu düşen olay, `SendInput` deneme/başarı/eksik.

## 5. Yaşam döngüsü

| Adım | Davranış |
|---|---|
| Başlangıç | DPI farkındalığı → profil → SID → `Local\MacroStat.Engine.<SID>` mutex → günlük → kanal → `runtime.json` → 1 ms timer → motor döngüsü |
| İkinci örnek | Mutex `ERROR_ALREADY_EXISTS` → `[BILGI] Motor zaten çalışıyor`, **çıkış 7** |
| `reload` | Profil diskten yeniden okunur; enjekte tuşlar bırakılır; döngüler durdurulur; yeni derleme; hata varsa **mevcut profil korunur** |
| `stop` / panik tuşu | Tüm tuşlar bırakılır, döngüler durdurulur, `runtime.json` yalnızca bu PID'e aitse silinir |
| `--parent-pid` | `OpenProcess(SYNCHRONIZE)` ile beklenir; ebeveyn çıkınca `request_stop` (Linux `getppid()==1` paritesi) |
| Job Object | GUI tarafının yedeğidir (`KILL_ON_JOB_CLOSE`); motor Faz 3'te kendi Job Object'ini kurmaz — `--parent-pid` birincil yoldur. GUI Faz 4'te Job Object'i kuracak |
| `--watch-restart <KEY_*>` | Yalnızca kancayı kurar, tuşu bekler ve **çıkış 42** verir (GUI yeniden başlatma sinyali) |
| Çıkış | `stopping` → `reset_transient` → tuşları bırak → kancayı kapat → 50 ms bekle → tekrar bırak → istatistik günlüğü |

### 5.1 Çıkış kodları

| Kod | Anlam |
|---|---|
| 0 | Temiz çıkış / başarılı IPC komutu |
| 2 | Geçersiz CLI değeri veya profil |
| 3 | `--test-macro` hedefi bulunamadı |
| 4 | Kayıt/test sırasında eylem yok ya da G/Ç hatası |
| 5 | Kanca/motor hatası; Linux'ta Windows'a özgü mod reddi |
| 6 | IPC/kimlik/runtime.json hatası (fail-closed) |
| 7 | Motor zaten çalışıyor |
| 42 | `--watch-restart` tuşu basıldı |

## 6. Günlük

- Yol: `%LOCALAPPDATA%\MacroStat\engine.log`; `--log <yol>` ile değiştirilebilir.
- 1 MB eşiğinde `engine.log.1`'e döner (tek yedek), dönme başarısızlığı motoru durdurmaz.
- Her satır UTC `YYYY-MM-DD HH:MM:SS` + düzey (`INFO`/`UYARI`/`HATA`); satır sonları boşluğa çevrilir.
- **Gizlilik:** yazılan `TEXT` içeriği ve tuş yükü asla loglanmaz; yalnızca olay adı, sayaç ve hata metni yazılır.
- Dosya satır başına açılır; böylece kullanıcı günlüğü açık tutarken dönme engellenmez.

## 7. `--record-input`

| Öğe | Davranış |
|---|---|
| Şema | Linux `MOUSE_PATH` v1: `version`, `sample_ms` (4), `duration_ms`, `points` `[dt,dx,dy]`, `buttons` `[t,"BTN_*",1/0]`, `keys` `[t,"KEY_*",1/0]`, `start` |
| Klavye | Yalnızca `--record-input`; `--record-mouse` `keys` alanını hiç yazmaz (Linux bayt uyumu) |
| Sınırlar | 200.000 nokta, 600 sn; panik tuşu kaydı bitirir |
| Yazım | `path.tmp` + `fsync` + rename (yarım dosya kalmaz) |
| Özet | Nokta, düğme, tuş ve süre sayıları basılır |

## 8. Doğrulama

| Kontrol | Komut | Sonuç |
|---|---|---|
| Linux biçim | `cargo fmt --check` | ✅ |
| Linux lint | `cargo clippy --all-targets -- -D warnings` | ✅ 0 uyarı |
| Birim testleri | `cargo test --all` | ✅ 91/91 |
| Anahtar üreticisi | `python3 tools/gen_keymap.py --check` | ✅ güncel |
| Windows tip denetimi | `cargo check --all-targets --target x86_64-pc-windows-msvc` | ✅ |
| Windows lint | `cargo clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings` | ✅ 0 uyarı |
| `linux/` dokunulmadı | `find linux -newermt 2026-11-10` | ✅ yok |

Kabul ölçütü eşlemesi:

| Ölçüt | Kanıt |
|---|---|
| `reload`/`stop`/`status` | `ipc::tests` (komut ayrıştırma, yanıt/bayrak, durum JSON'u); gerçek E2E ☐ L-01 |
| Job Object | GUI Faz 4'te kurulacak; motor tarafı `--parent-pid` ile ebeveyn kaybını yakalar; sınır belgelendi |
| `--record-input` çıktısı geçerli | `record::tests` (tuş kaydı, boş `keys` alanının atlanması, şema); gerçek E2E ☐ L-06 |

## 9. Bilinen sınırlar ve sonraki fazlar

1. **Gerçek Windows E2E yok:** kanal DACL'i, `ConnectNamedPipe` davranışı ve `runtime.json` yarışları yalnızca CI'da doğrulanacak (L-01…L-06).
2. **Job Object motor tarafında değil:** Faz 4'te GUI, motoru `KILL_ON_JOB_CLOSE` Job Object içine alacak; motorun kendi önlemi `--parent-pid`. GUI çökerse Job Object motoru öldürür; motorun tuş bırakma temizliği bu durumda garanti edilmez (süreç ölürken `SendInput` tuş-up gönderilemez). Bu, Linux'taki `getppid` yolundan farklı bir artık risk olarak belgelenmiştir.
3. **Güvenli masaüstü / UAC / kilit ekranı:** kanca olay almaz; motorun bu durumda tuş bırakması `SendInput`'un UIPI engeline takılabilir. GUI rozetiyle bildirilecek (I-06…I-08).
4. **`stop` yanıtı ile süreç çıkışı arasında:** yanıt gönderildikten sonra bağlantı kapanır; motor ana döngüde 20 ms içinde çıkar. GUI bu aralıkta `runtime.json`'ın silinmesini beklemelidir.
5. **`reload` sırasında panik tuşu:** Linux paritesi gereği CLI `--panic-key` geçersiz kılması reload sonrası profil dosyasındaki değere döner.
6. **Fare düğmeleri tetikleyici olarak:** Linux `EV_KEY`/`BTN_*` semantiğini korumak için `MouseButton` olayları da `TriggerEngine::on_key`'e beslenir.

## 10. Sonraki adım

Faz 4 — PyQt6 GUI taşıması: `runtime.json` + `--ipc` istemcisiyle motor kontrolü, Job Object kurulumu, profil/sayfa paritesi, TR/EN. Faz 4 raporu `reports/04-gui.md` olarak yazılacaktır.
