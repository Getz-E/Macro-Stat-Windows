# MacroStat for Windows'a Katkıda Bulunma

İlginiz için teşekkürler. Bu belge geliştirme ortamını, CI'nın kullandığı
komutları ve depo kurallarını açıklar. Aşağıdaki komutlar projenin kendi
dosyalarında tanımlı olanlardır; burada listelenmeyen bir komut proje kuralı
değildir.

## 1. Geliştirme ortamı

| Araç | Kullanılan sürüm | İş |
|---|---|---|
| Windows | 11 x64 (desteklenen); 10 22H2 derleme 19045 (yalnızca geliştirme/test) | Motor ve GUI yalnızca Windows'ta çalışır |
| Rust | 1.79+ ve MSVC toolchain (`stable-x86_64-pc-windows-msvc`) | `macrostat-engine.exe` derler; platformdan bağımsız çekirdek Linux'ta da test edilebilir |
| Python | 3.10+ (CI 3.12 kullanır) | GUI, testler, araçlar |
| PyQt6 | `windows/gui/requirements-win.txt` | GUI çalışma zamanı |
| PyInstaller 6 | `windows/gui/requirements-build.txt` | İsteğe bağlı: onedir paketi üretir |
| Inno Setup 6.3+ | Harici kurulum | İsteğe bağlı: `MacroStat-Setup-1.0.0.exe` üretir |
| PyYAML 6 | `ci/requirements.txt` | Yalnızca `ci/preflight.py` için gerekli; çalışma zamanı bağımlılığı değildir |

Windows 11 x64 resmi olarak desteklenen platformdur. Windows 10 22H2 (derleme
19045) normal destek süresini doldurmuştur; kök `README.md` bunu belirtir. Bu
sürüm yalnızca geliştirme ve test ortamı olarak tutulur ve burada resmi olarak
desteklenen bir sürüm olarak sunulmaz.

Windows'ta kurulum:

```powershell
python -m pip install -r windows/gui/requirements-win.txt
python -m pip install -r ci/requirements.txt
```

Paketleme için GUI dosyası yerine `windows/gui/requirements-build.txt` kurun; o
dosya GUI bağımlılıklarını da içerir.

## 2. Biçimlendirme, lint ve testler

Rust komutlarını `windows/engine` içinden çalıştırın:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

`keymap.rs`, `tools/gen_keymap.py` tarafından kaynaklardan üretilir ve çıktısını
`rustfmt` ile biçimlendirir. `windows/engine/src/keymap.rs` dosyasını elle
düzenlemeyin.

```bash
# windows/engine içinden; rustfmt kurulu olmalıdır
python tools/gen_keymap.py --check
```

`--check`, üretilen çıktıyı commit edilmiş dosyayla karşılaştırır ve dosya bayatsa
başarısız olur. Kasıtlı bir eşleme değişikliğinden sonra yeniden üretmek için:

```bash
python tools/gen_keymap.py
```

GUI testleri (depo kökünden):

```bash
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -p "test_gui*.py" -v
```

Ön kontrol (depo kökünden; PyYAML gerekir):

```bash
python ci/preflight.py
```

Beklenen çıktı: `CI on kontrolu gecti`. İş grafiğini, tetikleyicileri, action SHA
sabitlemelerini, yasaklı ağ komutlarını, sürüm tutarlılığını ve `ci/windows.yml`
ile `.github/workflows/windows.yml` dosyalarının bayt düzeyinde aynı olduğunu
denetler.

### Windows E2E testleri

`windows/tests/test_e2e_windows.py` gerçek `macrostat-engine.exe` ikilisini
çalıştırır ve **yalnızca Windows'ta** anlamlıdır; diğer platformlarda 7 E2E test
metodu atlanır. Girdi enjeksiyonu senaryoları için Windows masaüstü oturumu
gerekir.

```powershell
$env:MACROSTAT_E2E_ENGINE = "windows\engine\target\release\macrostat-engine.exe"
python -m unittest discover -s windows/tests -p "test_e2e*.py" -v
```

Ortam kanca olayı iletemiyorsa `MACROSTAT_E2E_SKIP_INPUT=1` ayarlayın; girdiye
bağlı senaryolar atlanır ve `skipped` olarak raporlanır. Atlama "geçti" anlamına
gelmez.

Platformdan bağımsız regresyon testleri dâhil tam takım:

```bash
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v
```

## 3. Sürekli entegrasyon

Etkin iş akışı `.github/workflows/windows.yml` dosyasıdır. Kaynağı
`ci/windows.yml`'dir; iki dosya **bayt düzeyinde aynı** olmalıdır. İş akışını
değiştirdikten sonra:

```bash
cp ci/windows.yml .github/workflows/windows.yml
python ci/preflight.py
```

CI işleri:

| İş | Çalıştırdığı |
|---|---|
| `core` | keymap `--check`, `cargo fmt --check`, `-D warnings` ile clippy, `cargo test --all` |
| `gui-tests` | `ci/preflight.py`, ardından GUI testleri (offscreen) |
| `build` | `windows/scripts/build.ps1 -SkipTests`; kurulum paketini, taşınabilir ZIP'i, toplamları ve ham paketi `macrostat-win-x64` artefaktı olarak yükler (30 gün saklanır) |
| `e2e` | `build` artefaktındaki gerçek motor üzerinde E2E takımı |
| `audit` | `cargo audit --deny warnings` ve `pip-audit --strict`; pull request'te bilgilendirici, push/`v*` etiketi/haftalık zamanlamada engelleyici |
| `sign` | Yalnızca `SIGNING_ENABLED` tanımlıysa; korumalı `signing` ortamını gerektirir |

Bütün `uses:` satırları 40 karakterlik tam commit SHA'larına sabit kalmalıdır;
`ci/preflight.py` etiket ve dal kullanımını reddeder.

`windows/ci/windows.yml` **tarihsel bir Faz 0 taslağıdır**. Kullanılmaz ve
`.github/workflows/` altına asla kopyalanmamalıdır. Yalnızca yukarıdaki etkin
dosyalar düzenlenir.

## 4. Depo kuralları

- Linux projesi **ayrı bir depodur**. Dosyalarını bu depoya kopyalamayın ve bu
  depodaki bir değişiklik kapsamında Linux projesini değiştirmeyin.
- Üretilen veya gizli dosyaları commit etmeyin: `windows/engine/target/`,
  `windows/dist/`, `windows/build/`, `**/__pycache__/`, `*.pyc`,
  `.pytest_cache/`, `.venv/`, `*.pfx`.
- `windows/engine/src/keymap.rs` dosyasını elle düzenlemeyin; yeniden üretin ve
  `--check` ile doğrulayın.
- Yukarıda açıklanan CI kopya eşitliğini koruyun; `ci/preflight.py` denetimlerini
  atlatmayın veya zayıflatmayın.
- Yeni kaynak dosyalar mevcut kodla aynı başlık biçimini taşımalıdır:
  `SPDX-License-Identifier: GPL-3.0-or-later` ve
  `Copyright (C) 2026 MacroStat contributors`.
- Bir değişikliği geçirmek için testleri veya güvenlik denetimlerini silmeyin,
  zayıflatmayın.

Bu proje için tanımlanmış bir commit mesajı kuralı, Developer Certificate of
Origin (DCO) zorunluluğu veya belgelenmiş dal koruma kuralı **yoktur**. Var
olduğunu varsaymayın.

## 5. Pull request

Pull request açmadan önce:

1. Bölüm 2'deki biçimlendirme, lint, test ve ön kontrol komutlarını çalıştırın.
2. Keymap değişikliğinden sonra `--check` komutunu yeniden çalıştırın.
3. İş akışını değiştirdiyseniz iki kopyayı güncelleyin ve ön kontrolü çalıştırın.
4. Neyin, neden değiştiğini ve neyi doğruladığınızı açıklayın. Yalnızca
   Windows'ta çalışan testlerden hangilerini çalıştırdığınızı, hangilerini
   çalıştıramadığınızı açıkça belirtin.

Güvenlik sorunları herkese açık issue takipçisi yerine
[`SECURITY.tr.md`](SECURITY.tr.md) sürecini izlemelidir.
