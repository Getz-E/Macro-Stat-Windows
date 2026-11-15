# Faz 8 — Nihai Rapor ve Sürüm Kontrol Listesi (11 Kasım 2026)

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


Bu rapor, Windows taşımasının kaynak düzeyindeki nihai durumunu, doğrulama
kanıtlarını ve sürüm öncesi açık işleri tek yerde toplar. `linux/` ağacına
hiçbir aşamada yazılmadı.

## 1. Yönetici özeti

Windows sürümü **kaynak ve yapılandırma düzeyinde tamamdır**: motor, girdi
backend'i, IPC/yaşam döngüsü, GUI, paketleme, güvenlik sertleştirmesi ve
uçtan uca kabul koşucusu hazırdır. Eksik olan tek şey **gerçek Windows
ortamında üretim ve kabul kanıtıdır**: CI ilk koşusu, kurulum/kaldırma VM
testi, SmartScreen görsel kanıtı ve imza (sertifika dış bağımlılığı).

Bu, kapsam dışı bir eksik değil, ortam sınırıdır: çalışma ortamı çevrimdışı
Linux'tur, Windows ikilisi burada üretilemez ve çalıştırılamaz.

## 2. Teslim envanteri

| Alan | Konum | Durum |
|---|---|---|
| Motor (Rust) | `windows/engine/` | ✅ kaynak + 95 test |
| GUI (PyQt6) | `windows/gui/` | ✅ kaynak + 47 test |
| Paketleme | `windows/gui/MacroStat.spec`, `windows/installer/MacroStat.iss` | ✅ tanımlı |
| Derleme/imza/doğrulama | `windows/scripts/{build,sign,verify}.ps1` | ✅ hazır |
| CI | `ci/windows.yml` + `ci/preflight.py` | ✅ hazır, ⏳ koşulmadı |
| Güvenlik | `windows/docs/SECURITY-WINDOWS.md` | ✅ kontrol listesi |
| Sözleşmeler | `windows/docs/PROFILE-CONTRACT.md`, `profile.schema.json`, `KEY-DICTIONARY.tsv` | ✅ donduruldu |
| Belgeler | `windows/README.md`, `windows/docs/*`, `windows/reports/00–08` | ✅ |
| Sürüm notları | `windows/RELEASE-NOTES-1.0.0.md` | ✅ |
| CI kılavuzu | `windows/docs/CI-RUNBOOK.md` | ✅ |

## 3. Doğrulama kanıtı (yerel, çevrimdışı)

| Kontrol | Komut | Sonuç |
|---|---|---|
| Biçim | `cargo fmt --all -- --check` | ✅ temiz |
| Lint | `cargo clippy --all-targets --locked --offline -- -D warnings` | ✅ temiz |
| Rust testleri | `cargo test --locked --offline` | ✅ **95/95** |
| Python sözdizimi | `python -m py_compile …` | ✅ temiz |
| GUI birim testleri | `python -m unittest discover -s windows/tests -p "test_gui*.py"` | ✅ **47/47** |
| E2E keşfi | aynı komut, `-p "test_e2e*.py"` | ⏳ **9 senaryo** yalnızca Windows'ta |
| CI ön kontrolü | `python ci/preflight.py` | ✅ dosya/sürüm/iş akışı tutarlı |
| İş akışı ayrıştırma | PyYAML | ✅ `core, gui-tests, build, e2e, sign` |
| Sürüm tek kaynak | `VERSION` = `Cargo.toml` = `version_info.txt` = `.iss` | ✅ 1.0.0 |

## 4. Sürüm kontrol listesi

### Kaynak ve yapılandırma

- [x] Motor release profili: `panic = "abort"`, LTO, strip, `--locked` derleme
- [x] Manifest `asInvoker` + `uiAccess=false` (GUI EXE)
- [x] Profil sözleşmesi v1 ve şema dosyası
- [x] Türkçe/İngilizce çeviri kapsamı %100 (testli)
- [x] Lisans ve üçüncü taraf bildirimi (`LICENSE`, `THIRD_PARTY.md`)
- [x] Sürüm notları (`RELEASE-NOTES-1.0.0.md`)
- [x] `--self-test` tanı kipi (kanca, imza yalıtımı, panik mandalı, takılı tuş)
- [x] `SHA256SUMS.txt` üretimi ve `verify.ps1` doğrulaması aynı biçimde
- [x] Kurulum kullanıcı kapsamlı; kaldırma veriyi varsayılan olarak silmez

### Üretim ve kabul (Windows gerekir)

- [ ] `core`, `gui-tests`, `build`, `e2e` işleri yeşil — `docs/CI-RUNBOOK.md`
- [ ] `MacroStat-Setup-1.0.0.exe` ve `MacroStat-1.0.0-portable.zip` üretildi
- [ ] `SHA256SUMS.txt` değerleri sürüm notlarına işlendi
- [ ] Temiz VM'de kurulum → çalıştırma → kaldırma; kalıntı yok
- [ ] Windows 10 22H2 ve Windows 11 x64'te açılış
- [ ] %100/%150/%200 DPI ve çoklu monitör (negatif koordinat) testi
- [ ] TR-Q ve US klavye düzeni
- [ ] Notepad/Explorer/Chrome, yönetici CMD (UIPI negatif), UAC istemi
- [ ] Uyku/uyanma, RDP, hızlı kullanıcı değiştirme
- [ ] Defender taraması; SmartScreen uyarısı belgelendi (görsel)
- [x] GitHub Actions `uses:` satırları doğrulanmış SHA'lara sabitlendi
- [ ] `cargo audit` / `pip-audit` sonuçları temiz (`audit` CI işi eklendi, henüz koşmadı)

### İmza (opsiyonel, sertifika sağlanınca)

- [ ] OV sertifika + bulut HSM/uzak imzalama
- [ ] `SIGNING_ENABLED=true`, `SIGNING_PFX_BASE64`, `SIGNING_PFX_PASSWORD`
- [ ] `sign.ps1` + RFC 3161 zaman damgası; `verify.ps1 -RequireSignature` geçer
- [ ] İmzalı artefaktlarda toplamlar yeniden üretildi

## 5. Açık işler ve gerekçeleri

| Açık iş | Neden açık | Kapanış ölçütü |
|---|---|---|
| CI ilk koşusu | Ortam çevrimdışı; depo `.github/workflows` içermiyor | 5 iş yeşil (imza hariç) |
| `cargo audit` / `pip-audit` | Çevrimdışı; paket indeksine erişim yok | `audit` işi temiz rapor verir |
| Kurulum/VM testleri | Windows VM yok | Kontrol listesi maddeleri |
| Kod imzası | Sertifika yok (dış bağımlılık) | `-RequireSignature` geçer |
| Uzun süreli kararlılık (10 dk) | CI süresi ve girdi oturumu | VM'de sel/döngü testi |

Hiçbiri kaynak hatası değildir; hepsi üretim/kabul ortamı gerektirir. Bu
nedenle sürüm, **"aday"** olarak işaretlenmiştir; ikili dosyalar CI'da
üretildiğinde 1.0.0 olarak yayınlanabilir.

## 6. Güvenlik duruşu (özet)

- Ağ yüzeyi yok; adlandırılmış kanal yalnızca yerel, SID DACL'i ve rastgele
  token'lı; ilk örnek bayrağı ve istemci kimliği koruması etkin.
- `runtime.json` doğrulanmadan hiçbir IPC komutu gönderilmez.
- Panik tuşu kuyruk taşmasında bile çalışır; kapanışta tuşlar bırakılır.
- Profil güvenilmeyen girdi kabul edilir ve sınırlarla evcilleştirilir.
- Artık riskler (aynı kullanıcı gözetlemesi, imza taklidi, imzasız dağıtım)
  `reports/06-security-review.md` §5'te açıkça listelendi.

## 7. Sonuç

Windows taşıması kaynak düzeyinde tamamlanmıştır. Sıradaki tek anlamlı adım,
`docs/CI-RUNBOOK.md` izlenerek ilk gerçek `windows-latest` koşusunu yapmak ve
çıkan artefaktlarla VM kabul kontrol listesini doldurmaktır. Bu rapordaki
işaretlenmemiş maddeler o koşudan gelen kanıtlarla kapatılacaktır.
