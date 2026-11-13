# Faz 5 — Paketleme ve CI (Windows)

Tarih: 10 Kasım 2026
Kapsam: `windows/installer/`, `windows/scripts/`, `windows/gui/{MacroStat.spec,version_info.txt}`, `windows/{VERSION,LICENSE,THIRD_PARTY.md}`, `ci/windows.yml` (depo kökü)

Bu fazda amaç, Faz 1–4'te tamamlanan motor + arayüzü **kurulabilir ve doğrulanabilir bir Windows ürününe** dönüştürmekti. Ağ, telemetri ve otomatik güncelleme içermeyen sınır aynen korundu.

---

## 1. Özet

| Konu | Sonuç |
|---|---|
| Motor ikilisi | `cargo build --release --locked` (MSVC, `windows/engine/Cargo.toml` sürümü 1.0.0) |
| Arayüz paketi | PyInstaller 6 **onedir**; `MacroStat.exe` + `_internal/` |
| Kurulum | Inno Setup 6, **kullanıcı kapsamlı** (`PrivilegesRequired=lowest`), yönetici istemez |
| Taşınabilir sürüm | `MacroStat-<sürüm>-portable.zip` |
| Bütünlük | `SHA256SUMS.txt` (küçük harf, `hash  dosya` biçimi) |
| İmza | `signtool` + RFC 3161; PFX (CI sırrı) veya sertifika deposu; **opsiyonel** ve varsayılan kapalı |
| CI | GitHub Actions `windows-latest`: 4 iş (`core`, `gui-tests`, `build`, `sign`) |
| Doğrulama | `verify.ps1` dosya varlığı + SHA-256 + `--version` duman testi + Authenticode durumu |

Sürüm tek kaynaktan türetilir: `windows/VERSION` (1.0.0) = `engine/Cargo.toml` = `version_info.txt` (1.0.0.0) = kurulum dosyası adı. `build.ps1` ilk ikisinin farklı olması durumunda **derlemeyi reddeder**.

---

## 2. Yapılan dosyalar

| Dosya | Görev |
|---|---|
| `gui/MacroStat.spec` | PyInstaller onedir tanımı; motoru `binaries` olarak `_internal/` içine koyar, `assets/` (profil + SVG + ICO) paketler, kullanılmayan Qt modüllerini dışlar. Motor yoksa **bilerek** başarısız olur. |
| `gui/version_info.txt` | Windows dosya sürümü bilgisi (`1.0.0.0`, ürün/şirket adı, GPL telif) |
| `gui/assets/macrostat.ico` | 16–256 px çok boyutlu simge (SVG'den üretildi) |
| `installer/MacroStat.iss` | Inno Setup betiği; UTF-8 BOM'lu, TR/EN diller, kaldırmada veri silme onayı |
| `scripts/build.ps1` | Sürüm doğrulama → cargo → (isteğe bağlı testler) → PyInstaller → lisans kopyalama → ZIP → ISCC → SHA-256 |
| `scripts/sign.ps1` | Authenticode imzalama (PFX veya mağaza parmak izi), sırları asla yazdırmaz |
| `scripts/verify.ps1` | Kurulum paketini değiştirmeden doğrular |
| `ci/windows.yml` | GitHub Actions iş akışı (depo kökünde `.github/workflows/windows.yml` olarak kullanılır) |
| `VERSION`, `LICENSE`, `THIRD_PARTY.md` | Sürüm, GPL metni, üçüncü taraf bileşenler |

---

## 3. Ürün yerleşimi

Kurulumdan sonra (`%LOCALAPPDATA%\Programs\MacroStat`):

```
MacroStat.exe                     arayüz (PyInstaller başlatıcı)
_internal/macrostat-engine.exe    Rust motoru
_internal/assets/default_profile.json
_internal/assets/macrostat.svg
_internal/assets/macrostat.ico
_internal/...                     Python, Qt6, çalışma zamanı DLL'leri
LICENSE  THIRD_PARTY.md  VERSION
```

Taşınabilir ZIP aynı dosyaların köküdür (kullanıcı verisi içermez). `common.find_engine()` bu düzeni zaten arıyordu (`_internal/macrostat-engine.exe`), bu yüzden **kaynak kod değişikliği gerekmedi**; yalnızca `core.py` simge aramasına `_internal/assets` adayı eklendi.

---

## 4. Derleme zinciri (`build.ps1`)

1. `-Version` verilmezse `windows/VERSION` okunur; `MAJOR.MINOR.PATCH` biçimi zorunlu.
2. `engine/Cargo.toml` sürümü ile karşılaştırılır; farklıysa durur.
3. Araçlar **açıkça** çözülür: verilen yol → bilinen adaylar → `Get-Command` → anlaşılır hata. PATH'e sessizce güvenilmez.
4. `cargo build --release --locked --manifest-path engine/Cargo.toml`; ikili yoksa durur.
5. `-SkipTests` yoksa GUI testleri `QT_QPA_PLATFORM=offscreen` ile koşar.
6. PyInstaller'ın kurulu olduğu doğrulanır; `dist/` ve `build/pyinstaller/` temizlenir; spec derlenir.
7. `LICENSE`, `THIRD_PARTY.md`, `VERSION` paket köküne kopyalanır.
8. `Compress-Archive` ile taşınabilir ZIP (LZMA yerine Optimal — PowerShell'in yerleşik desteği).
9. `-SkipInstaller` yoksa ISCC `/DMyAppVersion=<sürüm>`; çıktı yoksa hata.
10. `SHA256SUMS.txt`: her artefakt için `sha256  ad` (küçük harf), ASCII.

Kurulum paketi derlemesi ISCC'nin lisans dosyasını okumasını gerektirir; `LicenseFile=..\LICENSE` düz metin olarak gömülür. Bu dosya uzantısız olduğu için ilk gerçek derlemede (Faz 6, Windows VM) lisans sayfasının göründüğü ayrıca doğrulanacaktır.

---

## 5. İmzalama akışı (opsiyonel)

`ci/windows.yml` içindeki `sign` işi yalnızca `vars.SIGNING_ENABLED == 'true'` iken çalışır ve `signing` ortamına (korumalı sırlar) bağlıdır. Sıra:

1. İmzasız artefakt indirilir.
2. PFX, base64 sırdan `$RUNNER_TEMP`'e yazılır → **paket içi** iki EXE imzalanır → `finally` ile PFX silinir.
3. ZIP yeniden paketlenir, kurulum paketi **yeniden** derlenir (imzalı ikilileri gömer).
4. Kurulum paketi imzalanır (PFX yine geçici ve silinir).
5. SHA-256 toplamları yeniden üretilir (imza baytları değiştirdiği için).
6. `verify.ps1 -RequireSignature` — imza geçerli değilse iş başarısız olur.
7. `macrostat-win-x64-signed` artefaktı yüklenir.

Çatal (fork) PR'larında `vars`/`secrets` bulunmadığı için imza işi hiç çalışmaz; sır sızdırma yüzeyi yoktur. `sign.ps1` parola ve PFX içeriğini hiçbir zaman ekrana basmaz, geçici dosya bırakmaz.

---

## 6. CI işleri

| İş | İçerik | Bağımlılık |
|---|---|---|
| `core` | `gen_keymap.py --check`, `cargo fmt --check`, `clippy -D warnings`, `cargo test --all` (working dir `windows/engine`) | — |
| `gui-tests` | Python 3.12 + pip önbelleği, `unittest discover -s windows/tests`, `offscreen` | — |
| `build` | Rust + Python + `pyinstaller>=6,<7`; `build.ps1 -SkipTests`; `macrostat-win-x64` yüklenir | core, gui-tests |
| `sign` | Yukarıdaki imza zinciri; `macrostat-win-x64-signed` yüklenir | build, `SIGNING_ENABLED` |

`QT_QPA_PLATFORM=offscreen` iş akışı düzeyinde tanımlıdır. İş akışı hiçbir adımda dış servise veri göndermez; tek ağ kullanımı bağımlılık kurulumu ve yapı artefaktı aktarımıdır.

---

## 7. Doğrulama (`verify.ps1`)

| Kontrol | Davranış |
|---|---|
| Zorunlu dosyalar | `MacroStat.exe`, `_internal/macrostat-engine.exe`, profil şablonu, `LICENSE`, `THIRD_PARTY.md` |
| SHA-256 | `SHA256SUMS.txt` satırları yeniden hesaplanır; eksik/uyumsuz dosya hata |
| Duman testi | `macrostat-engine.exe --version` çalışır ve çıktı verir |
| Authenticode | Her EXE için durum yazdırılır; `-RequireSignature` ile `Valid` şartı |
| Çıkış kodu | Herhangi bir sert hata → 1, aksi halde 0 |

Bu betik **hiçbir dosyayı değiştirmez**; yayın öncesi salt okunur kapıdır.

---

## 8. Test ve doğrulama durumu

| Kontrol | Durum |
|---|---|
| Tüm GUI modülleri `py_compile` | ✅ |
| `windows/tests` — 45 test | ✅ 45/45 (offscreen) |
| Paket sözleşmesi testleri | ✅ `_internal/` motor + profil adayları, donmuş dizin, sürüm tutarlılığı, kurulum ilkeleri |
| `MacroStat.spec`, `version_info.txt` sözdizimi | ✅ |
| `ci/windows.yml` YAML ayrıştırma + iş/adım listesi | ✅ |
| `.iss` UTF-8 BOM | ✅ (Türkçe yorum/mesajlar bozulmaz) |
| Gerçek `cargo build`/PyInstaller/ISCC | ⏳ Windows VM veya CI (Faz 6) |
| İmzalı yayın zinciri | ⏳ sertifika sağlanınca (Faz 8) |

Bu makinede MSVC bağlayıcısı ve Inno Setup bulunmadığı için EXE üretimi burada **çalıştırılamaz**; doğrulama `windows-latest` üzerinde yapılacaktır. Bu bir eksik değil, planlanmış sınırdır (bkz. `docs/COMPATIBILITY.md`: Wine ile derleme desteklenmez).

---

## 9. Bilinen sınırlar ve riskler

1. **İmzasız dağıtım:** SmartScreen "bilinmeyen yayıncı" uyarısı verir; kurulum belgesinde açıklanır (P-02).
2. **PyInstaller tek dosya değil:** onedir seçildi — açılış hızlı, DLL çıkarma yok, imza tek tek dosyalara uygulanabilir; kurulum yine tek EXE.
3. **Lisans sayfası biçimi:** uzantısız düz metin; ilk gerçek derlemede doğrulanacak (madde 4).
4. **Kurulum simgesi** `gui/assets/macrostat.ico` gerektirir; ICO silinirse ISCC derlemesi durur (istenen davranış).
5. **Kaldırmada veri:** varsayılan **hayır** (`MB_DEFBUTTON2`); profil/günlük yalnızca açık onayla silinir.
6. **Zaman damgası sunucusu** varsayılan `http://timestamp.digicert.com`; tamamen çevrimdışı imzalama istenirse `-TimestampUrl` boş bırakılıp imza süresi sertifikayla sınırlanır.

---

## 10. Sonraki faz

**Faz 6 — gerçek Windows doğrulaması:** temiz Win10 22H2 ve Win11 VM'de kurulum/açılış/kaldırma, SmartScreen davranışı, UIPI önsezisi ölçümleri, `docs/TEST-MATRIX.md` P-01/P-02/S-01 satırlarının doldurulması.
