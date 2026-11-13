# Faz 12 — Kapsamlı Güvenlik Denetimi ve Sertleştirme

Tarih: 13 Kasım 2026. Bu adımda motor (Rust), GUI (Python/PyQt6), kurulum
betikleri, CI/tedarik zinciri ve depo hijyeni güvenlik açısından incelendi;
doğrulanan düşük/orta riskli bulgular düzeltildi ve regresyon testleri
eklendi. **Kritik veya Yüksek önem dereceli sömürülebilir bir açık
bulunmadı.** Proje bu adımdan sonra da "yayıma hazır" ilan edilmedi: açık
kalan CI/VM doğrulamaları Bölüm 8'de listelenmiştir.

## 1. İncelenen dosyalar

| Alan | Dosyalar |
|---|---|
| Motor çekirdeği | `engine/src/{lib,main,runtime,record,ipc,hook,inject,actions,profile,trigger,keymap,mouse,log,winutil,selftest}.rs` |
| GUI | `gui/main.py`, `gui/macrostat/{app,common,core,settings,sanitize,transfer,recorder,system,winplatform,editor,onboarding,logpanel,widgets,theme,i18n,nav,macros}.py` |
| Paketleme | `gui/MacroStat.spec`, `gui/version_info.txt`, `installer/MacroStat.iss`, `scripts/{build,sign,verify}.ps1` |
| CI | `ci/windows.yml`, `.github/workflows/windows.yml`, `ci/preflight.py`, `ci/requirements.txt`, `gui/requirements-win.txt`, `gui/requirements-build.txt` |
| Depo | `.gitignore`, `.gitattributes`, `README.md`, `LICENSE`, `VERSION` |
| Önceki raporlar | `reports/{06,09,10,11}-*.md`, `docs/{SECURITY-WINDOWS,WINDOWS-ARCHITECTURE,CI-RUNBOOK,PROFILE-CONTRACT}.md` |

İnceleme yöntemi: kaynak okuma, `grep` tabanlı yüzey taraması (kabuk
çağrıları, `shell=True`, `subprocess`, `Popen`, `QProcess`, `SendInput`,
`unsafe`, dosya açma/yazma, ağ komutları, gizli bilgi desenleri), ardından
düzeltme + `cargo fmt/clippy/test` ve Python test paketi.

## 2. Zaten var olan ve korunan önlemler

Bu adımda hiçbiri kaldırılmadı veya zayıflatılmadı:

- 18 adet `uses:` referansının tamamı 40 karakterli commit SHA'sına sabit;
  preflight bunu zorunlu kılıyor.
- `permissions: contents: read`; `persist-credentials: false`; `sign` işi
  yalnızca `push`/`workflow_dispatch` + `vars.SIGNING_ENABLED` +
  `environment: signing`.
- `audit` işi yalnızca `pull_request` olayında `continue-on-error`; push,
  etiket ve haftalık taramada engelleyici.
- `ci/preflight.py`: iş grafiği, `needs`, tetikleyiciler, yol filtreleri,
  `v*` etiketi, `schedule`, SHA sabitlemesi, yasak ağ komutları ve
  `.github/workflows/windows.yml` ↔ `ci/windows.yml` bayt eşitliği.
- Adlandırılmış kanal: kullanıcı SID DACL'i, `FILE_FLAG_FIRST_PIPE_INSTANCE`,
  `PIPE_REJECT_REMOTE_CLIENTS`, rastgele 128-bit token, istemcide
  `SECURITY_IDENTIFICATION`.
- `runtime.json`: 64 KiB sınırı, korumalı DACL, atomik yazma, SID/kanal
  öneki/PID/oluşturma zamanı doğrulaması.
- GUI: sabit motor adayları (PATH araması yok), `mkstemp`+`fsync`+
  `os.replace` atomik yazma, reparse reddi, PID/oluşturma zamanı/görüntü adı
  doğrulaması, Job Object, günlük rotasyonu ve denetim karakteri temizliği.
- Profil sözleşmesi sınırları (32 MiB, 500 makro, 2000 eylem, 200k nokta,
  64 KiB metin, 600 s gecikme, 9999 tekrar).

## 3. Bulgular ve düzeltmeler

| # | Önem | Bulgu | Düzeltme |
|---|---|---|---|
| M-1 | Orta | CI'da `pip-audit` sürümsüz kuruluyordu (`pip install pip-audit`); tedarik zinciri kayması mümkündü. | `pip-audit>=2.7,<3` aralığına bağlandı. `cargo-audit` zaten `--version 0.23.0` ile sabit. |
| M-2 | Orta | `--test-macro` başarıyla biten bir makro sonrası basılı tuşları bırakmıyordu; `KEY_DOWN` ile biten bir test, masaüstünü tuş basılı bırakabilirdi (kullanılabilirlik + güvenlik: kullanıcı girdisi takılı kalır). | Başarı yolunda `release_all_keys` çağrılıyor; `pressed` boşsa etkisiz. İptal yolu zaten bırakıyordu. |
| L-1 | Düşük | Rust `reject_reparse`/`reject_link` yalnızca `is_symlink()` denetliyordu; Windows junction/mount point reparse noktası olup `is_symlink()` ile görünmez. GUI tarafı `FILE_ATTRIBUTE_REPARSE_POINT` denetliyordu; motor tarafı gerideydi. | `winutil::is_reparse_point` eklendi: Windows'ta `file_attributes() & 0x400`, POSIX'te `is_symlink()`. `runtime`, `record` ve profil okuma bunu kullanıyor. |
| L-2 | Düşük | Motor profil okuma (`runtime::load_profile` ve `main::load_profile`) `metadata` + sınırsız `read_to_string` kullanıyordu: `stat` ile okuma arasında büyüyen dosya belleğe sınırsız alınabilirdi (TOCTOU). | Tek açışta `read_text_limited`: tanıtıcı `metadata`'sı + `take(max+1)` + okunan baytlarda yeniden denetim. Hata mesajları korundu; ayrıca reparse reddi eklendi. |
| L-3 | Düşük | `ipc::start` SID'i doğrulamadan SDDL'e gömüyordu (`D:P(A;;GA;;;<sid>)`); `runtime::protect_runtime_acl` doğruluyordu. SID kaynağı güvenilir olsa da savunma derinliği eksikti. | `ipc::sid_is_safe` (boş değil; yalnızca ASCII alfanümerik ve `-`) eklendi; `start` ihlalde `IpcError::InvalidSid` döndürüyor; `runtime` aynı yardımcıyı kullanıyor. |
| L-4 | Düşük | GUI kayıt sonucu `open(path)` + `json.load` ile reparse/boyut denetimi olmadan okunuyordu. | `recorder.load_record_document`: `common.read_private_bytes` (reparse reddi + 32 MiB) + `json.loads`. |
| L-5 | Düşük | `transfer.load_payload` `stat` sonrası `read_bytes()` ile sınırsız okuyordu (TOCTOU). | Tek açışta `read(MAX_FILE_BYTES+1)` ve okunan uzunluk denetimi. |
| L-6 | Düşük | `common.read_private_bytes` `stat` sınırını uyguluyor ama okunan baytlarda yeniden denetlemiyordu. | Okuma sonrası `len(data) > max_bytes` denetimi eklendi. |
| B-1 | Bilgilendirici | `core.py` yorumu Windows'ta bulunmayan `O_NOFOLLOW`'u varmış gibi anlatıyordu; davranış doğruydu (reparse reddi). | Yorum gerçeğe uygun hale getirildi. |
| B-2 | Bilgilendirici | `sign.ps1` PFX parolasını `signtool /p` bağımsız değişkeniyle geçiriyor; süreç listesinde görülebilir. | Değiştirilmedi: imzalama yalnızca korumalı `signing` ortamında ve geçici runner'da çalışıyor; davranış değişikliği kanıtsız riskli. Öneri: `signtool` dosya tabanlı parola desteği varsa ona geçmek. |
| B-3 | Bilgilendirici | `verify.ps1`, `SHA256SUMS.txt` içindeki adları `Join-Path` ile birleştiriyor; yalnızca okuma yapıldığından etki düşük. | Değiştirilmedi; ileride `..` içeren adları reddetmek düşünülebilir. |
| B-4 | Bilgilendirici | `%LOCALAPPDATA%` yoksa veri dizini geçici dizine düşüyor; ACL beklentisi zayıflar. | Kabul: yalnızca bozuk/soyulmuş ortamda; kullanıcı önizlemesi ve raporlarda belgeli. |
| B-5 | Bilgilendirici | `Local\MacroStat.Engine.<sid>` mutex'i aynı kullanıcıdaki bir sürecin motoru kilitlemesine yol açabilir (DoS). | Kabul: aynı kullanıcı zaten kendi oturumunda girdi enjekte edebilir; yetki yükselmesi yok. |
| B-6 | Bilgilendirici | Bilinmeyen CLI bayrakları sessizce yok sayılıyor. | Güvenlik etkisi yok; davranış Linux ile aynı. |
| B-7 | Bilgilendirici | Bağımlılıklar hash ile kilitli değil (G-08); Dependabot yapılandırması yok. | Açık iş olarak bırakıldı; ilk yeşil CI koşusundan sonra `pip freeze`/hash kilidi önerilir. |

### Saldırı senaryoları (özet)

- **M-2:** Kullanıcı, `KEY_DOWN` ile başlayıp bırakma eylemi olmayan bir
  makroyu "Test" ile çalıştırır; motor çıkar, tuş mantıksal olarak basılı
  kalır. Düzeltmeden sonra çıkıştan önce tüm basılı tuşlar bırakılır.
- **L-2:** Profil dosyası `stat` ile küçük görünüp okuma sırasında büyütülürse
  motor belleğe sınırsız veri alabilirdi. Artık okuma `take(max+1)` ile
  sınırlı ve sonuç yeniden denetleniyor.
- **L-1:** Kullanıcı aldatılarak `profile.json` yerine bir junction
  yerleştirilirse motor başka bir hedefi okuyabilirdi. Artık reparse bayrağı
  reddediliyor (Windows'ta junction dahil; yerel Linux testi symlink'i
  kapsıyor).

## 4. Değişen dosyalar

| Dosya | Değişiklik |
|---|---|
| `engine/src/winutil.rs` | `is_reparse_point`, `reject_reparse`, `read_text_limited` + `ReadTextError`; 2 test |
| `engine/src/runtime.rs` | Profil okuma sınırlı/tek açış + reparse reddi; SID doğrulaması ortak yardımcıya bağlandı |
| `engine/src/record.rs` | Junction farkındalıklı reddetme; symlink regresyon testi |
| `engine/src/ipc.rs` | `sid_is_safe`, `IpcError::InvalidSid`; 1 test |
| `engine/src/main.rs` | Sınırlı profil okuma; `--test-macro` çıkışında tuş bırakma |
| `gui/macrostat/common.py` | Okuma sonrası boyut denetimi |
| `gui/macrostat/recorder.py` | `load_record_document` (sınırlı + reparse reddi) |
| `gui/macrostat/transfer.py` | Sınırlı içe aktarma okuması |
| `gui/macrostat/core.py` | Yanıltıcı `O_NOFOLLOW` yorumu düzeltildi |
| `windows/tests/test_gui_windows.py` | `RecordDocumentTests`, `TransferLoadTests`, büyüme testi (+8 test) |
| `ci/windows.yml`, `.github/workflows/windows.yml` | `pip-audit>=2.7,<3` (bayt-eşit kopya) |
| `docs/SECURITY-WINDOWS.md` | Reparse satırı junction'ı da kapsayacak şekilde netleştirildi |
| `README.md` (windows) | Bu raporun dizin satırı |

Ürün davranışında kullanıcıya görünen değişiklik yalnızca M-2 (test makrosu
sonunda tuş bırakma) ve güvenlik reddi yollarıdır; mevcut testlerden hiçbiri
silinmedi veya zayıflatılmadı.

## 5. Eklenen/güncellenen testler

| Test | Kapsam |
|---|---|
| `winutil::tests::reparse_helpers_detect_links_but_allow_regular_files` | Normal dosya kabul; symlink reddi; eksik yol |
| `winutil::tests::text_reader_enforces_the_byte_limit` | Tam sınır kabul; sınır+1 red; eksik dosya hatası |
| `record::tests::writing_through_a_symlink_is_refused` | Kayıt hedefi symlink ise yazma reddi, hedef değişmiyor |
| `ipc::tests::sddl_interpolation_rejects_unsafe_sids` | SDDL meta karakterleri, boşluk, boş SID |
| `PrivateFileTests::test_growth_between_stat_and_read_is_bounded` | `stat`→`read` büyümesi |
| `RecordDocumentTests` (4 test) | Geçerli belge, bozuk JSON, aşırı boyut, symlink |
| `TransferLoadTests` (3 test) | Geçerli JSON, aşırı boyut, bozuk JSON |

## 6. Gerçek test sonuçları

| Komut | Sonuç |
|---|---|
| `cargo fmt --all -- --check` | **Temiz** |
| `cargo clippy --all-targets --offline -- -D warnings` | **Temiz** |
| `cargo test --locked --offline` | **99/99 geçti** (önceki 95 + 4 yeni) |
| `QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v` | **62 test, 7 atlandı, 0 hata** (önceki 54 + 8 yeni) |
| `python ci/preflight.py` | **Geçti** (0 hata) |
| `cmp ci/windows.yml .github/workflows/windows.yml` | **Eşit** |
| YAML yapısal ayrıştırma (PyYAML 6.0.3) | 4 tetikleyici, 6 iş, 18 `uses:` |
| Gizli bilgi taraması (özel anahtar/token desenleri, `.env`, `*.pfx`, `*.p12`, `*.pem`) | **Bulgu yok** |
| `cargo audit` / `pip-audit` | **Çalıştırılmadı** — bu ortamda araçlar kurulu değil ve çevrimdışı; CI'da `audit` işi çalıştırır |
| `git status` / `git diff` | **Çalıştırılamadı** — bu çalışma dizininde `.git` yok; depo oluşturulmadı |

Yerelde `pip install -r ci/requirements.txt` PEP 668 nedeniyle reddedilir;
PyYAML 6.0.3 zaten kurulu olduğundan preflight çalıştı. Atlanan 7 Python
testi Windows'a özgüdür (gerçek Win32/süreç yolları); "geçti" sayılmadı.

## 7. Yalnızca Windows'ta doğrulanması gerekenler

1. **Junction reddi:** `FILE_ATTRIBUTE_REPARSE_POINT` yolu Linux'ta
   derlenmiyor; Windows'ta `mklink /J` ile hedef dizine junction kurulup
   `profile.json`, `runtime.json` ve kayıt hedefinin reddedildiği
   görülmelidir. Symlink yolu Linux testinde kapsandı.
2. **`--test-macro` tuş bırakma:** Gerçek `SendInput` ve klavye durumu
   gerektirir; Windows'ta `KEY_DOWN` ile biten bir makro çalıştırılıp tuşun
   basılı kalmadığı gözlenmelidir.
3. **IPC SDDL doğrulaması:** `sid_is_safe` saf birim testiyle doğrulandı;
   gerçek kanal oluşturma ve erişim reddi yalnızca Windows'ta sınanabilir.
4. **CI `audit` işi:** `cargo audit --deny warnings` ve
   `pip-audit --strict` ilk gerçek koşuda yeşil olmalı; yerelde çalıştırılmadı.
5. **Kurulum/kaldırma:** Kullanıcı verisi silme istemi ve imza doğrulaması
   VM'de denenmeli (önceki fazlardan açık).

## 8. Düzeltilmeyenler ve gerekçeleri

- **PFX parolasının komut satırında olması (B-2):** Davranış değişikliği
  imzalama akışını bozabilir; ortam zaten korumalı ve geçici. Kanıtsız
  değişiklik yapılmadı.
- **Bağımlılık hash kilidi / Dependabot (B-7):** Sürüm aralıkları bilinçli;
  ilk yeşil CI koşusundan sonra `pip freeze` ile tam kilitleme planı G-08'de.
  Dependabot yapılandırması otomatik PR üretir; kullanıcı onayı olmadan
  eklenmedi.
- **Kurulum betiğinde reparse varsayımı (B-3/B-4):** Kaldırıcı yalnızca kendi
  dizinlerini siliyor; aynı kullanıcının dizin değiştirmesi tehdit modeli
  dışında.
- **Aynı kullanıcı içi kanca/kanal tehditleri:** Windows oturum içi sınırları
  gereği kabul; belgelenmiş.

## 9. Kalan riskler ve yayım durumu

- Yeni düzeltmelerin Windows'a özgü yolları bu ortamda **çalıştırılmadı**
  (Bölüm 7). Bu nedenle sertleştirme "kaynak + Linux'ta doğrulanabilen
  kısımlar" düzeyinde tamamdır; Windows VM/CI kanıtı yoktur.
- Açık işler: G-04 (gerçek CI koşusu), G-05 (VM kabulü), G-08 (tam sürüm
  kilidi), G-10 (kod imzası), G-11 (dayanıklılık).
- **Proje yayıma hazır ilan edilmedi.** Kritik/Yüksek bulgu yok; ancak
  yukarıdaki Windows doğrulamaları tamamlanmadan sürüm onayı verilmemelidir.
