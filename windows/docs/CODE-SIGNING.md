# Kod İmzalama — Seçenekler ve Öneri

Durum: **Sertifika yok; ilk sürüm imzasız yayınlanacak** (kullanıcı kararı, 9 Kasım 2026). Bu belge Faz 0 karşılaştırmasını ve sonradan eklenebilir imza altyapısının tasarımını kaydeder. Fiyatlar **tahmini aralıktır**; teklif alınmadan kesinleşmez.

## 1. Seçenekler

| Seçenek | Uygunluk (Türkiye) | EV | CI/otomasyon | Yaklaşık maliyet | SmartScreen |
|---|---|---|---|---|---|
| **Azure Artifact Signing — Basic** | ❌ Public Trust kapsamı dışı (kuruluşlar: ABD, Kanada, AB, UK, AU, NZ, JP, KR, SG, CH, NO, IL; bireyler yalnızca ABD/Kanada) | ❌ | ✅ (Azure/Entra, 5.000 imza/ay) | Azure aboneliği + kullanım | İtibar zamanla oluşur |
| **Azure Artifact Signing — Premium** | ❌ Aynı kapsam | ❌ | ✅ (100.000 imza/ay) | Daha yüksek | Aynı |
| **OV Authenticode** | ✅ | ❌ | ⚠️ Donanım anahtarı veya bulut HSM/uzak imzalama | ~200–500 USD/yıl + HSM | İtibar zamanla oluşur |
| **EV Authenticode** | ✅ | ✅ | ⚠️ Donanım anahtarı / bulut HSM | ~300–700 USD/yıl + HSM | Başlangıçta daha iyi |
| **Kendi imzası (self-signed)** | ✅ | — | ✅ | Ücretsiz | **İşe yaramaz**; yalnızca iç test |

### Notlar

- CA/B Forum gereği OV/EV özel anahtarı **FIPS 140-2 Level 2** donanımda tutulmalıdır. CI'da imzalama için bulut HSM / uzak imzalama hizmeti (CA'nın sunduğu) veya kendi HSM'iniz gerekir; anahtar dosya olarak repoya konmaz.
- Azure Artifact Signing **EV vermez** ve Public Trust kapsamı Türkiye'yi içermez; Türkiye'de tüzel kişilik olsa bile uygun ülke şartı sağlanmıyorsa kullanılamaz.
- İmzasız dağıtımda SmartScreen "Bilinmeyen yayıncı" gösterir; itibar indirme hacmiyle oluşur. README'de SHA-256 ve kaynak bağlantısı sunulur.
- Zaman damgası (RFC 3161) sertifika süresi dolsa bile eski sürümlerin geçerliliğini korur.

## 2. Öneri

1. **v1:** imzasız yayın; SHA-256 + kaynak + `THIRD_PARTY.md`.
2. **Sonraki sürüm:** Türkiye'de uygulanabilir tek gerçekçi yol **OV + bulut HSM/uzak imzalama**. Uygun ülkede tüzel kişilik varsa Azure Artifact Signing Basic yeniden değerlendirilir.
3. `uiAccess` **yalnızca** imzalı + `Program Files` altında çalışan EXE'de işler; bu proje v1'de `uiAccess` kullanmaz. İmza alınsa bile `uiAccess` ayrı bir ürün kararıdır.

## 3. CI entegrasyon taslağı

```
signtool sign /fd SHA256 /td SHA256 /tr <RFC3161_URL> /sha1 <thumbprint> <exe>
signtool verify /pa /all <exe>
sha256sum <çıktılar> > SHA256SUMS.txt
```

- İmzalanacaklar: `macrostat-engine.exe`, `MacroStat.exe`, `MacroStat-Setup-*.exe`.
- Sırlar **korumalı ortamda** (GitHub Environment + required reviewers) tutulur; fork PR'larında imzalama job'u çalışmaz.
- Sertifika yoksa job `if: vars.SIGNING_ENABLED == 'true'` ile atlanır; build yine üretilir.
- Zaman damgası URL'si CA'ya göre değişir; yapılandırılabilir secret.
- Doğrulama: `signtool verify /pa` + `WinVerifyTrust` (motor `--self-test` içinde opsiyonel).

## 3.1 Uygulama durumu (Faz 5)

Tasarım **uygulandı**: `../scripts/sign.ps1` (PFX veya mağaza parmak izi, RFC 3161, sırları
yazdırmaz), `../../ci/windows.yml` içinde `sign` işi (imzala → yeniden paketle → kurulumu imzala →
toplamları yenile → `verify.ps1 -RequireSignature`). Sertifika henüz alınmadığı için iş
varsayılan olarak **kapalıdır**; `SIGNING_ENABLED=true` ve `SIGNING_PFX_BASE64` /
`SIGNING_PFX_PASSWORD` sırları tanımlandığında etkinleşir. Faz 6'da iş koşulu
sıkılaştırıldı: `sign` yalnızca `push` veya `workflow_dispatch` olayında çalışır
(pull request'te asla) ve checkout `persist-credentials: false` kullanır. `verify.ps1`
artık ikiliyi çalıştırmadan önce SHA-256 ve Authenticode denetimlerini yapar.

## 4. Yeniden değerlendirme tetikleyicileri

- İndirme sayısı/dağıtım kanalı büyürse ve SmartScreen uyarısı kullanıcı kaybına yol açarsa.
- Kurumsal dağıtım/AppLocker/WDAC senaryosu gerekirse.
- `uiAccess` ürün gereksinimi hâline gelirse (imza zorunlu olur).
