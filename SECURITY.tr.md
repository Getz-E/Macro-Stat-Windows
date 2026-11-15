# Güvenlik Politikası

MacroStat for Windows bir sürüm adayıdır (1.0.0). Bu politika güvenlik
sorunlarının nasıl bildirileceğini ve kapsamı açıklar.

## Desteklenen sürümler

Güvenlik düzeltmeleri **1.x** sürüm hattı için sağlanır. Daha eski veya
yayımlanmamış geliştirme anlık görüntüleri desteklenmez. 1.0.0 hâlâ sürüm adayı
olduğu için mevcut tek hat 1.x'tir; kararlı bir sürüm henüz yayımlanmamıştır.

## Güvenlik açığı bildirimi

**Güvenlik açıklarını herkese açık GitHub Issues üzerinden bildirmeyin.**
Teknik ayrıntıları, kanıt (proof of concept) kodunu veya başka hassas bilgileri
herkese açık bir issue'ya asla yazmayın.

GitHub Private Vulnerability Reporting / Security Advisories özelliğinin bu depo
için **etkin olduğu doğrulanmamıştır**. **Report a vulnerability** seçeneği
yalnızca bir bakımcı bu özelliği depo ayarlarında etkinleştirdiyse kullanılabilir.

**Report a vulnerability** seçeneği görünüyorsa:

1. Deponun **Security** sekmesini açın.
2. **Advisories** (Security Advisories) bölümüne gidin.
3. **Report a vulnerability** seçeneğiyle bildirimi özel olarak gönderin.

Seçenek görünmüyorsa özellik etkin değildir. Bu durumda:

1. **Güvenlik ayrıntısı içermeyen** normal bir GitHub Issue açın — etkilenen
   bileşeni, yeniden üretme adımlarını, kanıt kodunu veya başka hassas bilgileri
   yazmayın.
2. Bu issue'da bakımcılardan özel bir bildirim kanalı sağlamalarını isteyin.
3. Bir bakımcı özel kanalı sağlayıp nasıl kullanılacağını açıklayana kadar
   hiçbir ayrıntıyı paylaşmayın.

Özel kanaldan gönderilen bildirimde mümkünse şunlar yer almalıdır:

- etkilenen sürüm veya commit;
- Windows sürümü ve mimarisi;
- sorunun tanımı ve etkisi;
- yeniden üretme adımları veya bir kanıt (proof of concept);
- varsa önerilen önlem.

Güvenlik bildirimleri için yayımlanmış bir e-posta adresi veya başka bir özel
iletişim kanalı yoktur; var olduğunu varsaymayın. Sabit yanıt süresi, hizmet
düzeyi taahhüdü veya destek garantisi verilmez.

## Kapsam

Kapsam içi:

- Rust motoru (`windows/engine/`);
- PyQt6 arayüzü (`windows/gui/`);
- kurulum ve paketleme betikleri (`windows/installer/`, `windows/scripts/`);
- CI iş akışı ve bağımlılık denetimi yapılandırması (`ci/`,
  `.github/workflows/`);
- profil/IPC sözleşmesi ve doğrulaması.

Tasarım gereği kapsam dışı (tehdit modeline bakın):

- yönetici pencerelerine veya güvenli masaüstüne enjeksiyon (UIPI ve Windows
  güvenlik modeli tarafından engellenir);
- çekirdek sürücüsü, hizmet, `uiAccess`, anti-cheat atlatma veya süreç
  enjeksiyonu — bunların hiçbiri kullanılmaz;
- aynı kullanıcı olarak kod yürütmeyi gerektiren saldırılar; Windows bunları
  aynı oturumdaki süreçlerden yalıtamaz.

Windows tehdit modelinin tamamı (varlıklar, önlemler, artık riskler):
[`windows/docs/SECURITY-WINDOWS.md`](windows/docs/SECURITY-WINDOWS.md).

## İşleme süreci

1. Bir özel kanal sağlandıktan sonra, o kanaldan gelen bildirim incelenir ve
   yeniden üretilir.
2. Düzeltme hazırlanır ve ilgili testlerle doğrulanır (Rust birim testleri,
   Python GUI testleri ve uygun olduğunda Windows E2E).
3. Düzeltme ve varsa güvenlik duyurusu, inceleme ve doğrulamadan sonra
   yayımlanır.
4. Bildirimde bulunan kişi talep ederse ve bu kişiyi riske atmayacaksa adı
   anılır.

Sabit bir süre taahhüt edilmez. Bildirimler eldeki imkânlar ölçüsünde ele alınır.

## Genel hatalar

Güvenlikle ilgili olmayan hatalar ve öneriler
[GitHub Issues](../../issues) üzerinden bildirilir. Güvenlik ayrıntılarını,
kanıt kodunu veya başka hassas bilgileri oraya yazmayın.
