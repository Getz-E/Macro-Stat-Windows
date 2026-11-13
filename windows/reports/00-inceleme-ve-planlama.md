# Adım Raporu 00 — İnceleme, Fizibilite ve Planlama

| | |
|---|---|
| **Tarih** | 9 Kasım 2026 |
| **Adım** | Linux ağacının incelenmesi + Windows EXE fizibilitesi + planlama |
| **Durum** | Tamamlandı |
| **Ayrıntılı rapor** | `windows/REPORT.md` |
| **Plan dosyası** | `~/.claude/plans/linux-dosyas-ndaki-her-eyi-incele-agile-raven.md` |

## Bu adımda yapılanlar

1. `linux/` ağacı salt okunur olarak incelendi: Rust motor (`src/main.rs`, `src/kwin_input.rs`, `protocols/fake-input.xml`), PyQt6 arayüzü (18 modül), 7 test dosyası, `README.md`, `SECURITY.md`, profil şeması, paketleme ve kurulum betikleri.
2. GitHub deposu kontrol edildi: `main`, 10 commit, yayın kaydı yok, Windows desteği yok.
3. Windows fizibilitesi araştırıldı: Win32 düşük seviye kancalar, `SendInput`, UIPI, güvenli masaüstü, DPI, IPC, Job Objects, PyInstaller, Inno Setup, Rust MSVC/cargo-xwin.
4. Dış kaynaklar doğrulandı: Windows 10 22H2 yaşam döngüsü (14 Ekim 2025 sonu), PyQt6 6.12.0 `win_amd64` tekerleği, Azure Artifact Signing uygunluğu/kotası, OV/EV donanım anahtarı koşulu.
5. Kullanıcı kararları plana işlendi: kurulum + uygulama EXE'leri; imzasız ilk sürüm + imza araştırması; normal yetki + UIPI belgelemesi; GitHub Actions `windows-latest`; parite + Windows iyileştirmeleri; Win10 22H2 + Win11 x64.

## Öne çıkan bulgular

- Windows'a taşıma **mümkün**; arayüz ve motor iş mantığı büyük ölçüde yeniden kullanılabilir.
- **İmkânsız/kapsam dışı:** yönetici pencerelere enjeksiyon (UIPI), güvenli masaüstü (UAC/kilit), gerçek sanal HID (imzalı sürücü), anti-cheat atlatma, otomatik güncelleme (çevrimdışı proje).
- **En riskli bölüm:** girdi backend'i — kanca zaman aşımı, geri besleme döngüsü, auto-repeat, takılı tuş temizliği.
- **Kod imzalama:** Azure Artifact Signing Public Trust Türkiye'yi kapsamıyor ve EV vermiyor; Türkiye için OV + bulut HSM/uzak imzalama gerçekçi yol. İlk sürüm imzasız.
- **Windows 10 22H2** artık destek dışı; uyumluluk korunacak ancak Windows 11 önerilecek.
- **Efor:** ~270–430 saat (~7–11 hafta).

## Üretilen dosyalar

| Dosya | İçerik |
|---|---|
| `windows/REPORT.md` | Tam inceleme, güvenlik değerlendirmesi, fizibilite matrisi, mimari, paketleme, riskler, efor, kararlar |
| `windows/reports/00-inceleme-ve-planlama.md` | Bu adım raporu |

## Doğrulama

- `linux/` altında bu oturumda hiçbir dosya yazılmadı/değiştirilmedi.
- Windows çıktıları yalnızca `windows/` altında üretildi.
- Ağ bağlantısı, telemetri veya yeni çalışma zamanı bağımlılığı eklenmedi.

## Sonraki adım

Faz 0 — Baseline: `windows/` iskeleti, profil/anahtar sözleşmesinin dondurulması, imza tekliflerinin karşılaştırılması, CI taslağı. Rapor: `windows/reports/00-baseline.md`.
