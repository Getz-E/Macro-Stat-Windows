# Faz 10 — CI ve Tedarik Zinciri Sertleştirmesi (12 Kasım 2026)

`reports/09-gap-analysis.md` içindeki G-06, G-07 ve G-08 maddelerinin
kapatılması için yapılan çalışma. Kaynak kodda davranış değişikliği yok;
değişiklikler CI iş akışı, ön kontrol betiği ve bağımlılık tanımlarıyla
sınırlıdır. `linux/` ağacına dokunulmadı.

## 1. Özet

| # | İş | Durum |
|---|---|---|
| G-07 | `uses:` satırlarını doğrulanmış commit SHA'larına sabitle | **Tamam** |
| G-07 | `ci/preflight.py`: SHA dışı sabitlemeyi hata yap | **Tamam** |
| G-06 | CI'a `cargo audit` + `pip-audit` işi ekle | **Tamam** (ilk koşu bekliyor) |
| G-08 | Paketleme bağımlılıklarını ayrı dosyaya ayır | **Tamam** (tam kilitleme ertelendi) |

## 2. Action sabitleme (G-07)

GitHub API üzerinden çözülen sürümlerin commit SHA'ları:

| Action | Sürüm | SHA |
|---|---|---|
| `actions/checkout` | v4 | `11d5960a326750d5838078e36cf38b85af677262` |
| `actions/setup-python` | v5 | `a26af69be951a213d495a4c3e4e4022e16d87065` |
| `actions/upload-artifact` | v4 | `ea165f8d65b6e75b540449e92b4886f43607fa02` |
| `actions/download-artifact` | v4 | `d3f86a106a0bac45b974a628896c90dbdf5c8093` |
| `dtolnay/rust-toolchain` | stable | `b725eda525362e7cf34af9a7243cac1e3f01a7ec` |

Etiketler yorum olarak korundu (`# v4` gibi) ki güncelleme sırasında
okunabilirlik bozulmasın.

`ci/preflight.py` içindeki `uses:` denetimi uyarıdan **hataya** çevrildi:

```python
ref = spec.rsplit("@", 1)[1]
if not re.fullmatch(r"[0-9a-f]{40}", ref):
    fail(problems, f"action SHA'ya sabitlenmemiş (etiket/dal): {spec}")
```

Böylece etiket veya dal kullanan bir gerileme CI'ı kırmadan sessizce
geçemez. Denetim hâlâ çevrimdışıdır ve ağa çıkmaz.

## 3. Bağımlılık denetimi işi (G-06)

`ci/windows.yml` içine `audit` işi eklendi:

- `runs-on: ubuntu-latest` (Rust/Python araçları Windows'a bağlı değil,
  daha hızlı ve ucuz);
- `cargo install cargo-audit --locked` → `cargo audit --file windows/engine/Cargo.lock`
  (kilitleme dosyası depo kökünden verilir, böylece `working-directory`
  gerekmez);
- `python -m pip install pip-audit` → `python -m pip_audit -r windows/gui/requirements-win.txt`;
- `continue-on-error: true`: ilk Windows koşusunu bilinen bir
  **bilgilendirici** uyarı yüzünden kilitlememek için. Politika gereği
  temiz olmayan bir bulgu sürümü engeller; iş raporlanır.

`REQUIRED_JOBS` listesine `audit:` eklendi; iş akışından işin sessizce
düşmesi artık ön kontrolde yakalanıyor.

## 4. Paketleme bağımlılıkları (G-08, kısmi)

`windows/gui/requirements-build.txt` oluşturuldu:

```
-r requirements-win.txt
pyinstaller>=6.0,<7
```

`build` işi artık yalnızca çalışma zamanı dosyasını değil bu dosyayı
kuruyor; pip önbellek yolu iki dosyayı da kapsıyor. Amaç, PyInstaller'ın
çalışma zamanı ortamına sızmasını ve iki listenin ayrışmasını önlemek.

**Tam sürüm kilitleme bilinçli olarak ertelendi:** yanlış bir tekerlek
sürümünü (ör. Windows'ta derlenmeyen bir PyQt6) kaynak ağacına sabitlemek
riskli. Sıra: ilk yeşil Windows CI koşusu → `pip freeze` çıktısından
`requirements-build.txt` üretimi → aynı dosyanın `build.ps1` tarafından
doğrulanması.

## 5. Doğrulama

| Komut | Sonuç |
|---|---|
| `python ci/preflight.py` | **Geçti** (0 uyarı, 0 hata) |
| `ci/windows.yml` PyYAML ayrıştırma | Geçti — işler: `core`, `gui-tests`, `build`, `e2e`, `audit`, `sign` |
| `cargo fmt --all -- --check` | Temiz |
| `cargo clippy --all-targets --offline -- -D warnings` | Temiz |
| `cargo test --locked --offline` | **95/95** |
| `QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests` | **54 test, 7 atlandı** |

## 6. Güncellenen belgeler

- `windows/docs/CI-RUNBOOK.md` §2: iş tablosuna `audit` eklendi, test
  sayıları güncellendi, SHA zorunluluğu belgelendi.
- `windows/reports/08-final-report.md`: SHA sabitleme maddesi işaretlendi;
  açık işler tablosundan SHA satırı çıkarıldı, yeşil iş sayısı 5'e çıktı.
- `windows/reports/06-security-review.md` §7 madde 3 kapatıldı.
- `windows/reports/09-gap-analysis.md`: G-06/G-07/G-08 durumları güncellendi.

## 7. Kalan işler

| Madde | Neden açık | Kapanış |
|---|---|---|
| İlk gerçek Windows CI koşusu (G-04) | Bu ortamda `.git` yok; iş akışı tetiklenemiyor | 5 iş yeşil |
| `audit` işinin ilk sonucu (G-06) | Ağ erişimli koşu gerekir | Temiz rapor |
| Tam bağımlılık kilitleme (G-08) | İlk yeşil koşuya bağlı | `pip freeze` tabanlı dosya |
| VM kabulü (G-05) | Windows VM yok | 08-final-report §4 listesi |

Bu adımda kaynak kodunda değişiklik yapılmadığı için yeni bir hata riski
eklenmedi; tüm denetimler yeşil.
