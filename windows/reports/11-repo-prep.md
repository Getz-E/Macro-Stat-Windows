# Faz 11 — GitHub Deposu Hazırlığı (13 Kasım 2026)

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


Bu adım, kaynak ağacını bağımsız bir GitHub deposu olarak yayımlamaya
hazırlar. Ürün kodunda davranış değişikliği yoktur; değişiklikler depo kök
dosyaları, CI iş akışı, ön kontrol betiği ve belgelerle sınırlıdır.
`linux/` ağacına dokunulmadı; `windows/engine/target/` silinmedi; hiçbir
test kaldırılmadı veya zayıflatılmadı.

## 1. Depo kökü

| Dosya | İçerik |
|---|---|
| `.gitignore` | `windows/engine/target/`, `windows/dist/`, `windows/build/`, `**/__pycache__/`, `*.pyc`, `.pytest_cache/`, `.venv/`, `*.pfx`, `linux/`, editör artıkları |
| `README.md` | Depo giriş belgesi; `windows/README.md` ile aynı durum ve sınırlar, yeni özellik iddiası yok |
| `LICENSE` | `windows/LICENSE` ile **bayt-eşit** (GPL-3.0-or-later, md5 `d32239bc…`); lisans metni değiştirilmedi |
| `.gitattributes` | Kaynak/veri LF, `*.ps1`/`*.iss` CRLF, ikili türler `binary` |

`windows/` altındaki mevcut dosyalar silinmedi. Kök `README.md`, Linux
ağacının depoya dahil **olmadığını** açıkça belirtir.

## 2. Etkin iş akışı

- `.github/workflows/windows.yml` oluşturuldu; `ci/windows.yml`'in bayt-eşit
  kopyası.
- `ci/preflight.py` artık **iki dosyanın eşitliğini** zorunlu kılar; kopya
  eksikse veya ayrışırsa çıkış kodu 1 olur.
- Mevcut 17 SHA sabitlemesine dokunulmadı; `core` işine eklenen
  `setup-python` aynı doğrulanmış SHA'yı (`a26af69b…`) kullanır. Toplam
  `uses:` sayısı 18, tamamı 40 karakterli küçük harf SHA.

## 3. Tetikleyiciler

| Olay | Filtre |
|---|---|
| `push` | tüm dallar (`**`), `v*` etiketleri, yollar: `windows/**`, `ci/**`, `.github/workflows/**` |
| `pull_request` | tüm dallar, aynı üç yol |
| `workflow_dispatch` | elle |
| `schedule` | `0 3 * * 1` (haftalık, Pazartesi 03:00 UTC) |

`v*` etiketi ve `schedule` yalnızca `audit` için değil, tam iş akışını
tetikler; imza işi olay koşulu gereği yine yalnızca push/dispatch'te
çalışır.

## 4. Audit politikası

```yaml
continue-on-error: ${{ github.event_name == 'pull_request' }}
```

| Olay | Davranış |
|---|---|
| `pull_request` | Bilgilendirici; iş kırmızı olsa da iş akışı yeşil |
| `push` | **Engelleyici** |
| `v*` etiketi | **Engelleyici** |
| `schedule` | **Engelleyici** |

Bulgular gizlenmez: PR dışındaki her başarısızlık iş akışını kırar.

Araç seçenekleri doğrulandı:
- **`cargo audit --deny warnings`**: `cargo-audit 0.23.0` kaynağında
  `deny: Vec<DenyOption>` (`-D/--deny`) ve "exit with an error on: warnings
  (any), unmaintained, unsound, yanked" tanımı doğrulandı. İş,
  `cargo install cargo-audit --locked --version 0.23.0` ile sabitlendi.
- **`pip-audit --strict`**: PyPI belgesine göre bağımlılık **toplama**
  hatalarında (çözümlenemeyen bağımlılık) hata verir; zafiyet bulgusu
  zaten varsayılan olarak çıkış kodu 1 üretir. Eklendi.

## 5. Preflight sertleştirmesi

`ci/preflight.py` yeniden yazıldı; korunan denetimler: gerekli dosyalar,
sürüm tek kaynağı, yasak ağ komutları.

Yeni denetimler:

1. `.github/workflows/windows.yml` var mı ve `ci/windows.yml` ile bayt-eşit mi.
2. `yaml.safe_load` ile yapısal ayrıştırma (`on` anahtarının PyYAML'da `True`
   olarak çözülmesi dahil).
3. `REQUIRED_JOBS` artık gerçek `jobs` anahtarları üzerinden denetleniyor
   (`core`, `gui-tests`, `build`, `e2e`, `audit`, `sign`).
4. `needs` grafiği: `build → {core, gui-tests}`, `e2e → {build}`,
   `sign → {build}` — tam küme eşitliği.
5. `sign`: `SIGNING_ENABLED`, `github.event_name`, `push` ve
   `pull_request` dışlama, `environment: signing` denetimi.
6. `audit`: `continue-on-error` yalnızca `pull_request`'e bağlı olmalı;
   `--deny warnings` ve `--strict` metinde bulunmalı.
7. `push`/`pull_request` yol filtrelerinde `windows/**`, `ci/**`,
   `.github/workflows/**`; `v*` etiketi; dal filtresi; `schedule` + `cron`.
8. `uses:` referanslarının tamamı `[0-9a-f]{40}`.
9. `windows/docs/KEY-DICTIONARY.tsv` varlığı (core işinin ilk adımı buna bağlı).
10. `ci/requirements.txt` varlığı.

PyYAML: yerel ortamda 6.0.3 kurulu; CI'da `gui-tests` işi preflight'tan önce
`python -m pip install -r ci/requirements.txt` çalıştırır. Modül yoksa
preflight açıklayıcı hata verip 1 döner.

Negatif testler (bellek içi mutasyonla): bozuk `sign.needs`,
koşulsuz `continue-on-error`, eksik yol filtresi ve eksik `v*` etiketi
**yakalandı**.

## 6. Core işi

`actions/setup-python@a26af69b…` (v5, Python 3.12) `core` işine eklendi;
`python tools/gen_keymap.py --check` artık runner'ın varsayılan Python'una
bağlı değil. `gen_keymap.py` biçimlendirme için `rustfmt` kullanır; bu,
`dtolnay/rust-toolchain` bileşenlerinde zaten var.

## 7. Tarihsel taslak

`windows/ci/windows.yml` silinmedi. Başına açık bir uyarı bloğu eklendi:
tarihsel Faz 0 taslağı, kullanılmıyor, `.github/workflows/` altına
kopyalanmamalı, `build/e2e/audit/sign` işlerini içermez. Dosyaya hiçbir
yerde referans verilmediği `grep` ile doğrulandı.

## 8. Doğrulama

| Komut | Sonuç |
|---|---|
| `python ci/preflight.py` | **Geçti** (0 hata) |
| YAML ayrıştırma | Geçti — 4 tetikleyici, 6 iş, `needs` doğru |
| Kopya eşitliği (`cmp`) | Eşit |
| SHA denetimi | 18/18 kırk karakterli küçük harf |
| `cargo fmt --all -- --check` | Temiz |
| `cargo clippy --all-targets --offline -- -D warnings` | Temiz |
| `cargo test --locked --offline` | **95/95** |
| `QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests` | **54 test, 7 atlandı** |

Test sayıları önceki adımla aynı; kod değişmediği için fark yok. Yerelde
`pip install -r ci/requirements.txt` PEP 668 nedeniyle reddedildi; PyYAML
6.0.3 zaten kurulu olduğundan preflight çalıştı. CI'da sanal ortam
kullanılmadığı için kurulum normal çalışacaktır.

## 9. Kalan işler

| Madde | Durum |
|---|---|
| G-04 gerçek Windows CI koşusu | Açık — depo henüz oluşturulmadı; bu ortamda `.git` yok |
| G-05 VM kabulü | Açık — Windows VM gerekir |
| G-08 tam sürüm kilitleme | Kısmi — ilk yeşil koşudan sonra `pip freeze` |
| G-10 kod imzası | Açık — sertifika dış bağımlılık |
| G-11 10 dk dayanıklılık | Açık — VM'de sel/döngü testi |

## 10. Gerçek depoya aktarılacaklar

Kök: `.gitignore`, `.gitattributes`, `README.md`, `LICENSE`,
`.github/workflows/windows.yml`, `ci/` (`windows.yml`, `preflight.py`,
`requirements.txt`), `windows/` (tümü; `engine/target/`, `**/__pycache__/`,
`dist/`, `build/` hariç). `linux/` **aktarılmayacak**.
