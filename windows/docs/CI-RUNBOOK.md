# Windows CI Koşu Kılavuzu (İlk Gerçek Koşu)

Bu belge, `ci/windows.yml` iş akışının Windows koşusunu adım adım açıklar. İş
akışı **çevrimdışı** hazırlandı; bu kılavuz uygulandığında koşucu yalnızca
bağımlılık kurulumu ve artefakt yüklemesi için ağ kullanır.

> **Doğrulanmış koşu (14 Kasım 2026, koşu 43638328015):** `Dependency audit`,
> `Engine core (fmt / clippy / test)`, `GUI unit tests`, `Build installer +
> portable ZIP` ve `E2E acceptance (real engine)` başarılı; `Sign artifacts
> (optional)` sertifika tanımlı olmadığı için atlandı. Kılavuz yeni ortamlar ve
> gelecekteki koşular için geçerlidir; VM kabulü (P-01) hâlâ yapılmamıştır.

## 0. Ön koşullar

- Etkin iş akışı `.github/workflows/windows.yml` altındadır ve kaynak
  `ci/windows.yml` ile **birebir aynı** olmalıdır. `ci/preflight.py` kopyaların
  eşitliğini, iş grafiğini ve SHA sabitlemelerini denetler.
- Depo kökünde `ci/preflight.py` çalışır durumda olmalı:
  `python ci/preflight.py` → `CI on kontrolu gecti`.
- `vars.SIGNING_ENABLED` tanımlı **değilse** imza işi atlanır; bu beklenen
  durumdur (sertifika yok).

## 1. İş akışını doğrula

Etkin dosya `.github/workflows/windows.yml` zaten yerindedir. Kaynak
`ci/windows.yml` değişirse kopyayı **yeniden üretin**; `preflight.py` iki
dosyanın birebir eşit olmasını zorunlu kılar:

```bash
cp ci/windows.yml .github/workflows/windows.yml
python ci/preflight.py
```

Tetikleyiciler `windows/**`, `ci/**` ve `.github/workflows/**` yollarını,
`v*` etiketlerini, tüm dalları ve haftalık `schedule` taramasını kapsar.
İş akışı `ci/preflight.py`'yi de çalıştırır; `ci/` altındaki betikleri silmeyin.

İsteğe bağlı: `workflow_dispatch` ile elle de başlatılabilir.

`core` işi, `dtolnay/rust-toolchain` bileşenlerine ek olarak açıkça
`rustup component add rustfmt` adımını çalıştırır. `tools/gen_keymap.py`
biçimlendirme için `rustfmt`'i PATH üzerinde arar (gerekirse
`rustup which rustfmt` ile bulur); bulamazsa hata verir ve `--check` bayat
`keymap.rs` dosyasında başarısız olur. Bu adım, Windows koşucusunda üretimin
deterministik olmasını sağlar; `--check` davranışı gevşetilmez.

## 2. Beklenen işler ve çıktılar

| İş | Süre (yaklaşık) | Başarı ölçütü |
|---|---|---|
| `core` | 6–10 dk | `rustup component add rustfmt`, `gen_keymap.py --check`, `cargo fmt --check`, `clippy -D warnings`, 99 test |
| `gui-tests` | 2–4 dk | `preflight.py` geçer (SHA sabitleme **zorunlu**); `test_gui*.py` 55 test |
| `build` | 10–15 dk | `MacroStat-Setup-1.0.0.exe`, `MacroStat-1.0.0-portable.zip`, `SHA256SUMS.txt`, `MacroStat/**` |
| `e2e` | 4–7 dk | 9 senaryo (girdi oturumu yoksa 6) |
| `audit` | 2–5 dk | `cargo audit --deny warnings` + `pip-audit --strict`; PR'da bilgilendirici, push/etiket/schedule'da **engelleyici** |
| `sign` | atlanır | `SIGNING_ENABLED` yoksa görünmez/atlanır |

Bütün `uses:` satırları 40 karakterlik commit SHA'larına sabittir
(`actions/checkout`, `setup-python`, `upload-artifact`,
`download-artifact`, `dtolnay/rust-toolchain`). Etiket veya dal
kullanılırsa `ci/preflight.py` **başarısız** olur. `audit` işi yalnızca
`pull_request` olayında `continue-on-error: true` ile bilgilendiricidir;
push, `v*` etiketi ve haftalık `schedule` koşularında başarısızlık iş
akışını kırar (bkz. `reports/11-repo-prep.md`).

## 3. İlk koşuda en olası sorunlar

1. **`core`: `cargo clippy` Windows'ta ek uyarı üretir.** Linux'ta `cfg(windows)`
   bloğu derlenmiyor; Windows'ta ilk kez derlenen FFI kodu uyarı verebilir.
   Uyarıyı düzeltin, `-D warnings` seviyesini düşürmeyin.
2. **`build`: PyInstaller motoru bulamaz.** Spec, `windows/engine/target/release/`
   altındaki ikiliyi arar; `build.ps1` önce cargo derlemesi yapar. Sıra
   bozulursa `common.find_engine()` adaylarına bakın.
3. **`build`: Inno Setup yolu.** `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`
   varsayılan; `windows-latest` imajında kurulu gelir; değilse `-Iscc` ile
   açık yol verin.
4. **`e2e`: kanca olay alamıyor.** Koşucu etkileşimli oturum sunmuyorsa U-05/U-08/U-09
   zaman aşımına uğrar. Geçici çözüm: işe `MACROSTAT_E2E_SKIP_INPUT: "1"`
   ekleyin; kalıcı çözüm etkileşimli bir VM'de koşmaktır. Atlama, test
   çıktısında `skipped` olarak görünür — "geçti" sayılmaz.
5. **`sign` beklenmedik şekilde çalıştı.** `SIGNING_ENABLED` yalnızca depo
   değişkeni olarak tanımlanmalı; fork PR'larında iş koşulu gereği çalışmaz.

## 4. Artefaktları doğrula

```powershell
# İndirilen dist klasöründe
powershell -ExecutionPolicy Bypass -File windows/scripts/verify.ps1
```

Beklenen: gerekli dosyalar, SHA-256 eşleşmeleri, `Authenticode NotSigned`
(imza yoksa) ve motor `--version` duman testi. `-RequireSignature` verilirse
imzasız yapı **başarısız** olur; bu yalnızca imzalı sürümde kullanılır.

## 5. Koşu sonrası

- [x] Beş işin durumu kaydedildi (14 Kasım 2026, koşu 43638328015: beşi başarılı, imza atlandı).
- [ ] `e2e` atlanan senaryo varsa nedeni `reports/08-final-report.md`'ye yazıldı.
- [ ] `SHA256SUMS.txt` değerleri sürüm notlarına eklendi.
- [ ] VM kurulum testi (P-01) ayrıca yapıldı.
