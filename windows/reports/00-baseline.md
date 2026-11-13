# Adım Raporu — Faz 0 Baseline

| | |
|---|---|
| **Tarih** | 9 Kasım 2026 |
| **Faz** | 0 — Baseline |
| **Durum** | Tamamlandı |
| **Kapsam** | `windows/` iskeleti, profil/anahtar sözleşmesinin dondurulması, mimari/güvenlik/uyumluluk/test belgeleri, imza karşılaştırması, CI taslağı |
| **Kod durumu** | Motor iskeleti derleniyor; iş mantığı yok (Faz 1) |

## 1. Özet

Faz 0'ın kabul ölçütü olan "profil şeması/anahtar sözlüğü belgelenmiş; `windows/` iskeleti hazır" sağlandı. Bu fazda **iş mantığı yazılmadı**; sonraki fazların üzerine kuracağı sözleşmeler ve yerleşim donduruldu. Linux ağacına dokunulmadı.

## 2. Üretilen yapı

```
windows/
  README.md
  docs/
    WINDOWS-ARCHITECTURE.md      süreç modeli, katmanlar, iş parçacıkları, D-01..D-12
    PROFILE-CONTRACT.md          donmuş profil/anahtar sözleşmesi
    profile.schema.json          makine-okunur şema (draft 2020-12)
    KEY-DICTIONARY.tsv           119 kanonik tuş + alias'lar (üretilmiş)
    COMPATIBILITY.md             Linux ↔ Windows eşlemesi
    SECURITY-WINDOWS.md          tehdit modeli ve en az yetki listesi
    TEST-MATRIX.md               kabul matrisi + Faz 0 doğrulamaları
    CODE-SIGNING.md              Azure/OV/EV karşılaştırması ve CI taslağı
  engine/
    Cargo.toml                   serde/serde_json; Windows crate Faz 2'de
    .gitignore
    src/{main,profile,keymap,trigger,actions,hook,inject,mouse,ipc,log,winutil,record}.rs
  gui/requirements-win.txt + README.md
  installer/README.md
  scripts/README.md
  tests/README.md
  ci/windows.yml                 GitHub Actions taslağı (çekirdek job aktif)
  reports/00-baseline.md         bu rapor
```

## 3. Dondurulan sözleşmeler

1. **Profil biçimi:** sarmalayıcı (`schemaVersion`, `activeProfileId`, `profiles`), profil, makro ve 11 eylem tipi; alan adları camelCase, okuma alias'lı. Ayrıntı: `docs/PROFILE-CONTRACT.md`.
2. **Anahtar dili:** `KEY_*`/`BTN_*` sembolik adları korunur; `+` ile kombinasyon; sol/sağ eşdeğerliği; sayısal evdev kodları yalnızca Linux'ta anlamlı. Sözlük Linux motorunun çözümleyicisinden **üretildi** (uydurulmadı).
3. **Mimari sınır:** saf çekirdek (`profile`, `keymap`, `trigger`, `actions`) Windows API çağırmaz; Windows katmanı `hook`/`inject`/`mouse`/`ipc`/`log`/`winutil`/`record`.
4. **Güvenlik:** `asInvoker`, sürücü yok, ağ yok, `uiAccess` yok; UIPI/güvenli masaüstü belgelenmiş sınır.
5. **İmza:** v1 imzasız; Türkiye için gerçekçi yol OV + bulut HSM; Azure Artifact Signing kapsam dışı.

## 4. Doğrulama

| Kontrol | Komut / yöntem | Sonuç |
|---|---|---|
| Biçim | `cargo fmt --all -- --check` | ✅ |
| Derleme | `cargo check --all-targets` | ✅ (13,9 s; Rust 1.99.1) |
| Şema JSON | `python3 -m json.tool docs/profile.schema.json` | ✅ |
| CI YAML | `yaml.safe_load(ci/windows.yml)` | ✅ |
| Profil sözleşmesi | Yapısal doğrulama betiği (`schemaVersion`, kimlikler, tetikleyici token'ları, eylem tipleri, metin/points sınırları) | ✅ 1 profil / 2 makro / 0 hata |
| Anahtar sözlüğü | Üretici betik → 119 kanonik tuş | ✅ |
| Bağlantılar | Göreli Markdown bağlantı taraması | ✅ |
| Linux dokunulmadı | `find linux -type f -newermt "2026-11-09 16:20:00"` | ✅ çıktı yok |

> `cargo check` Linux üzerinde yalnızca platformdan bağımsız iskeleti doğrular. Windows API'leri Faz 2'de ekleneceği için MSVC hedefiyle derleme Faz 2 kabulüdür.

## 5. Notlar ve kararlar

- **`windows` crate ertelendi:** sürüm/feature seti Faz 2'de doğrulanıp eklenecek; Faz 0'da yanlış sürüm kilitlememek için bilinçli olarak yok.
- **GUI kopyası Faz 4'te:** `linux/macrostat/` bu fazda kopyalanmadı; yalnızca bağımlılık ve taşıma tablosu donduruldu.
- **`schemaVersion`:** yazıcılar 1 üretir. Linux'ta test yükü `2` yazıyor ancak motor değeri doğrulamıyor; bu bir sözleşme sürümü değildir, Windows tarafı 1 üretir.
- **Şema toleransı:** şema yazıcı sözleşmesidir; motor bilinmeyen alanları yok sayar (ileri uyumluluk).
- **Sanal HID / UIPI / güvenli masaüstü:** ayrıntı ve alternatifler `reports/arastirma-01-windows-sinirlari-ve-alternatifler.md`.
- **CI taslağı:** depo köküne `.github/workflows/windows.yml` olarak taşınacak; paketleme/imzalama job'ları Faz 5'e kadar yorumlu.

## 6. Faz 1'e giriş

1. `profile.rs`: sarmalayıcı + sınırlar; Linux davranışıyla eşdeğerlik testleri.
2. `keymap.rs`: 119 kanonik tuş için soyut tuş tanımı ve Windows VK/tarama kodu tablosu; eksik eşleme listesi.
3. `trigger.rs`: 7 kip + mandal + double-tap + hold + press-release; Linux semantiği.
4. `actions.rs`: eylem kararı, `ACTION_LOCK`, jitter, iptal, takılı tuş kümesi.
5. Birim testleri Linux CI'da koşacak şekilde saf tutulur.

## 7. Değişmez kural kontrolü

- `linux/` altında bu fazda hiçbir dosya yazılmadı/değiştirilmedi.
- Tüm çıktılar `windows/` altında.
- Ağ bağlantısı eklenmedi (CI taslağı hariç; uygulama çevrimdışı).
