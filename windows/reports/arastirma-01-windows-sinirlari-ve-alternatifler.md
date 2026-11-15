# Araştırma Raporu 01 — Windows Sınırları, Projeye Etkisi ve Alternatifler

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


| | |
|---|---|
| **Tarih** | 9 Kasım 2026 |
| **Adım** | Yönetici pencereler / güvenli masaüstü / gerçek sanal HID sınırlarının araştırılması + Windows'a özgü daha mantıklı yollar |
| **Durum** | Tamamlandı |
| **Kapsam** | Yalnızca araştırma ve tasarım kararları; kod yazılmadı, `linux/` değiştirilmedi |
| **İlgili belgeler** | `windows/REPORT.md`, `windows/reports/00-inceleme-ve-planlama.md` |

## 1. Yönetici özeti

Üç sınır da **Windows güvenlik mimarisinin bilinçli tasarımıdır**, bir API eksikliği değildir:

1. **Yönetici pencerelere enjeksiyon** — UIPI (User Interface Privilege Isolation) nedeniyle normal yetkili bir süreç, daha yüksek bütünlük düzeyindeki (integrity level) süreçlere girdi enjekte edemez. Araştırmada ek ve proje açısından önemli bir bulgu çıktı: **sınır yalnızca yazma yönünde değil, okuma yönünde de değil — yükseltilmiş bir pencere ön plandayken normal yetkili süreç o pencereye giden girdiyi de gözetleyemiyor.** Yani "yönetici pencerede makro çalışmıyor" kadar "yönetici pencerede kayıt da yapılmıyor" da geçerlidir. Bu, kullanıcıya **sessiz başarısızlık** olarak yansır; arayüzde farkındalık göstergesi gerekir.
2. **Güvenli masaüstü** — UAC onay/parola istemi, kilit ekranı ve Ctrl+Alt+Del ayrı bir masaüstünde ve **SYSTEM bütünlük düzeyinde** çalışır. Microsoft açıkça "güvenli masaüstüne yalnızca Windows süreçleri erişebilir" der; `uiAccess` dahi buraya erişemez. Bu sınır aşılamaz ve **aşılmaması gerekir** (UAC'nin son savunma hattı).
3. **Gerçek sanal HID** — Virtual HID Framework (VHF) yalnızca **çekirdek modu** HID kaynak sürücüsünü destekler. Kullanıcı modunda "gerçek aygıt" oluşturmanın Microsoft destekli yolu yoktur; üçüncü taraf çekirdek sürücüleri ise imza, bakım, güvenlik ve anti-cheat riskleri nedeniyle kapsam dışıdır.

**Projeye net etkisi:** Normal masaüstü uygulamalarında parite korunur; yönetici pencereler ve güvenli masaüstü belgelenmiş, farkındalık göstergeli sınırlar olur; sanal HID ihtiyacı `SendInput` ile karşılanır ve bu seçim mimariye zaten işlenmiştir. Yeni bir zorunlu bağımlılık, sürücü veya ağ özelliği eklenmez.

**Yeni ve düzeltici bulgu:** Daha önce planda yazılan "`SendInput` 0 döner ve `GetLastError() == ERROR_ACCESS_DENIED` ise UIPI engeli algılanır" yaklaşımı **yanlıştır**. Microsoft, UIPI engelinin ne dönüş değerinden ne de `GetLastError`'dan anlaşılamayacağını belirtir. Bu raporla birlikte algılama tasarımı "en iyi çaba + ön plandaki pencerenin bütünlük düzeyini önceden sezme" olarak değiştirilmiştir ve `windows/REPORT.md` ile plan dosyası düzeltilmiştir.

---

## 2. Yöntem ve kaynaklar

- Microsoft Learn birincil kaynak olarak kullanıldı: `SendInput`, `SendSAS`, `RAWINPUTDEVICE`, `timeBeginPeriod`, UAC "How it works", "Security Considerations for Assistive Technologies", "Application manifests", VHF belgeleri.
- Pratik doğrulama için AutoHotkey resmî SSS'i kullanıldı; AHK, Windows'ta global kanca + sentetik girdi kullanan en olgun açık kaynak örnektir ve sınırı kullanıcı diliyle belgeler.
- Sanal HID alternatifleri için ViGEmBus ve Interception depoları incelendi.
- Arama motorları bu oturumda CAPTCHA/API hatası verdi; bu nedenle **doğrudan belge sayfaları** ve depo README'leri esas alındı. Belgeyle doğrulanamayan noktalar §8'de "doğrulanacak" olarak işaretlendi.

---

## 3. Sınır 1 — Yönetici (elevated) pencereler

### 3.1 Neden olmuyor

Windows, süreçleri **bütünlük düzeyi** ile etiketler (Low / Medium / Medium+ / High / System). Normal kullanıcı oturumunda açılan uygulamalar Medium IL'de çalışır; "Yönetici olarak çalıştır" ile açılan uygulamalar High IL'dedir. UIPI, düşük IL'den yüksek IL'ye pencereler arası iletişimi ve girdi enjeksiyonunu engeller.

Microsoft `SendInput` belgesi:

> "This function is subject to UIPI. Applications are permitted to inject input only into applications that are at an equal or lesser integrity level."

Yani Medium IL'deki MacroStat, High IL'deki yönetici CMD / Task Manager / regedit / yükseltilmiş kurulum penceresine **girdi gönderemez**.

### 3.2 Okuma yönü de etkileniyor (proje için kritik)

Kullanıcı isteği "enjeksiyon" üzerineydi; ancak projenin **kayıt** ve **tetikleme** yetenekleri okuma yönüne bağlıdır. AutoHotkey SSS'i bu davranışı net biçimde belgeler:

> "Hotkeys/hotstrings don't work when a program running as admin is active."
>
> "Hotkeys are also blocked, so for instance, a non-elevated program cannot spy on input intended for an elevated program."

Sonuç: yönetici bir pencere ön plandayken normal yetkili motorun kancası ne tetikleyici görür ne de kayıt yapabilir. Kullanıcı açısından bu, hata mesajı olmadan "hiçbir şey olmuyor" görünümüdür. Bu nedenle:

- Arayüzde **"yönetici pencere ön planda — girdi yalıtımı etkin"** durum rozeti gösterilmeli,
- günlüğe tek satır nedensellik yazılmalı,
- kullanıcıya "MacroStat'ı yönetici olarak çalıştır" önerisi sunulmalı (varsayılan değil).

### 3.3 Engelin algılanması neden mümkün değil

`SendInput` belgesi:

> "This function fails when it is blocked by UIPI. Note that neither GetLastError nor the return value will indicate the failure was caused by UIPI blocking."

Bu, API'nin engeli **bildirmediği** anlamına gelir; dönüş değeri gönderilen olay sayısıdır ve engellenen olaylar için güvenilir bir sinyal yoktur. Dolayısıyla tasarım:

1. **Önsezi (önerilen):** her enjeksiyon öncesi/sırasında ön plandaki pencereyi `GetForegroundWindow` + `GetWindowThreadProcessId` ile bul; hedef sürecin bütünlük düzeyini sorgula (`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `OpenProcessToken(TOKEN_QUERY)` + `GetTokenInformation(TokenIntegrityLevel)`). Erişim reddi veya `High`/`System` sonucu → "hedef yükseltilmiş" uyarısı.
2. **En iyi çaba:** `SendInput` dönüş sayısını istenen sayıyla karşılaştır; fark varsa genel "girdi kısmen iletilemedi" kaydı. UIPI'ye özel teşhis iddiası yok.
3. **Yanlış pozitif yönetimi:** sorgu başarısız olursa durum "bilinmiyor" olarak işaretlenir; kullanıcıya kesin hüküm verilmez.

### 3.4 Alternatifler ve gerçekçilik

| Alternatif | Ne yapar | Gereksinim | Değerlendirme |
|---|---|---|---|
| **MacroStat'ı yönetici olarak çalıştırma** | Aynı IL'de çalışır; `SendInput` ve kanca yönetici hedeflerde çalışır | Kullanıcı onayı (UAC), her açılışta | **En basit ve gerçekçi yol.** Varsayılan değil; belgelenir, isteğe bağlı sunulur. Kalıcı yükseltme önerilmez (saldırı yüzeyi büyür). |
| **`uiAccess="true"`** | UIPI'yi aşarak yüksek IL pencerelere girdi sürme; her zaman üstte olma | Authenticode imza + `Program Files` gibi güvenli konum + manifest; **yönetici kullanıcı tarafından başlatılmalı** | Teknik olarak mümkün; ancak imza ve kurulum koşulu nedeniyle **bu sürümde yok**, yol haritası. Ayrıca "erişilebilirlik" amacı dışında kullanılmaması önerilir. |
| **UI Automation (ValuePattern/InvokePattern)** | Hedef denetimin değerini/eylemini doğrudan ayarlar; girdi enjeksiyonu değildir | Hedef uygulama UIA sağlamalı; istemci de yükseltilmiş olmalı (High IL) | Yönetici hedeflerde **kısmi** çözüm; genel girdi yerine geçmez. v1 kapsamı dışı, ileride "akıllı eylem" olarak değerlendirilebilir. |
| **Hizmet (service) + SYSTEM** | SYSTEM IL'de çalışarak yönetici hedeflere girebilir | Yönetici kurulum, hizmet sözleşmesi, güvenlik incelemesi | **Reddedildi.** Çevrimdışı, kullanıcı düzeyi bir yardımcı program için orantısız yetki ve risk. |
| **Üçüncü taraf çekirdek filtre sürücüsü** | Girdiyi çekirdek düzeyinde yakalar/enjekte eder | İmzalı sürücü, yönetici kurulum, bakım | **Reddedildi** (bkz. §5). |

### 3.5 Projeye etkisi

- Tetikleyiciler, makrolar, metin ve fare eylemleri **yönetici pencerelerde çalışmaz** (uygulama normal yetkideyken).
- **Kayıt modu** da yönetici pencerede çalışmaz.
- Kullanıcı deneyimi: sessiz başarısızlık yerine **rozet + günlük + araç ipucu**.
- Kabul testlerinde negatif senaryo olarak yer alır (yönetici CMD'de tetikleyici → çalışmamalı, uyarı görünmeli).
- Efor etkisi: **+8–16 saat** (bütünlük sorgusu, rozet, günlük, testler, belge).

---

## 4. Sınır 2 — Güvenli masaüstü

### 4.1 Neden olmuyor

UAC belgesi: onay ve parola istemleri varsayılan olarak **güvenli masaüstünde** gösterilir ve

> "Only Windows processes can access the secure desktop."

Güvenli masaüstü, kullanıcı masaüstünden ayrı bir masaüstü nesnesidir; kanca zinciri, `SendInput`, pencere mesajları ve pano bu masaüstüne taşınmaz. Windows Server 2019 ve desteklenen istemcilerde güvenli masaüstüne **pano yapıştırma dahi** engellenmiştir.

Erişilebilirlik belgesi, `uiAccess` için bile kesin konuşur:

> "None of the previously listed scenarios provide access to UI running under the system IL. This is only possible if the process is launched in the user account control (UAC) desktop under SYSTEM (and system IL). In this case, setting UIAccess has no effect."

Yani sanal HID, `uiAccess`, yönetici olarak çalıştırma veya hizmet — hiçbiri güvenli masaüstüne **kullanıcı oturumu uygulaması** olarak erişemez; yalnızca güvenli masaüstünde SYSTEM olarak başlatılan bir süreç erişebilir ki bu, UAC'nin amacına aykırıdır.

### 4.2 Ctrl+Alt+Del özel durumu

`SendSAS` ile SAS (Secure Attention Sequence) **benzetimi** yapılabilir; ancak koşullar: hizmet olmak **veya** manifestte `uiAccess="true"` + Authenticode imza + `\Program Files\` / `system32` konumu + UAC etkin + yerel ilkede yazılım SAS'ına izin. Bu yalnızca Ctrl+Alt+Del **ekranını açar**; o ekranla etkileşim yine mümkün değildir. Proje için fayda/risk oranı düşüktür; **v1 kapsamı dışı**.

### 4.3 Projeye etkisi

- Makro çalışırken UAC istemi, kilit ekranı veya Ctrl+Alt+Del açılırsa eylem **durur/kesilir**; basılı tuşlar bırakılmalıdır (takılı tuş güvenliği devrede).
- Otomasyonla UAC onayı veya oturum açma **yapılamaz** — bu bir güvenlik kazanımıdır, eksiklik değil.
- Belgelenir; kabul testinde "UAC istemi açıkken kanca olay almamalı, motor çökmemeli, tuşlar bırakılmalı" senaryosu yer alır.
- Efor etkisi: **+2–4 saat** (belge + test); yeni kod neredeyse yok.

---

## 5. Sınır 3 — Gerçek sanal HID

### 5.1 Neden olmuyor

Microsoft VHF belgeleri: sanal HID aygıtı, `Vhf.sys` alt filtre sürücüsüne bağlanan bir **KMDF/WDM HID kaynak sürücüsü** ile oluşturulur (`VhfCreate`/`VhfStart`/`VhfReadReportSubmit`, `vhfkm.lib`). Belge açıkça:

> "In this release, VHF supports a HID source driver only in kernel mode."

Çekirdek sürücüsü ise şu zinciri gerektirir: WDK ile derleme, test imzası, üretim için Microsoft donanım/attestation imzası (Partner Center), kurulum/yükseltme, bakım, her Windows sürümünde uyumluluk doğrulaması ve güvenlik açısından tam ayrıcalık. Bu, çevrimdışı bir masaüstü yardımcı programı için orantısız bir taahhüttür.

### 5.2 Üçüncü taraf alternatifler

| Proje | Durum | Neden uygun değil |
|---|---|---|
| **ViGEmBus** | Emekliye ayrılmış; "production-signed" ikili dağıtıyor | Yalnızca **oyun kolu** (Xbox 360 / DualShock 4) emüle eder; klavye/fare sanal aygıtı sağlamaz. |
| **Interception** | WDK 7.1 tabanlı çekirdek filtre sürücüsü; kurulum yönetici hakkı ister; belge yalnızca Windows XP–10 aralığını belirtir | İmza/modern Windows uyumluluğu belgelenmemiş; bakımı belirsiz; anti-cheat ve güvenlik yazılımları tarafından işaretlenme riski; sistem genelinde girdi zincirine müdahale. **Kapsam dışı.** |
| **Kendi sürücümüz** | — | İmza, WDK, dağıtım ve güvenlik yükü; anti-cheat atlatma amacına hizmet edecekse etik olarak da reddedilir. |

### 5.3 "Gerçek sanal HID" ne kaybettirir?

- Enjekte olaylar `LLKHF_INJECTED` / `LLMHF_INJECTED` işaretini taşır; bazı uygulamalar/oyunlar/anti-cheat bunları reddeder veya işaretler.
- Gerçek aygıt kimliği, seri numarası, HID rapor tanımlayıcısı taklit edilemez; donanım kimliğine bakan yazılımlar sentetik girdiyi ayırt edebilir.
- Buna karşılık **normal masaüstü uygulamalarının tamamı** (tarayıcı, ofis, editör, terminal, uzak masaüstü istemcisi) `SendInput` ile çalışır; projenin hedef kitlesi için kayıp küçüktür.

### 5.4 Alternatifler

| İhtiyaç | Çözüm | Uygunluk |
|---|---|---|
| Genel klavye/fare/metin | `SendInput` (tarama kodu + Unicode) | **v1 birincil** |
| Uygulamaya özel metin | `KEYEVENTF_UNICODE`; desteklenmeyen yerlerde panoya kopyala + Ctrl+V (isteğe bağlı) | Yüksek |
| Anlamsal eylem (düğme tıklama) | UI Automation | Orta; v2 |
| Oyun/anti-cheat gerektiren senaryo | Gerçek donanım veya kullanıcı tercihi | **Desteklenmez**, belgelenir |

> Not: `SendInput` ile üretilen olaylar **Raw Input tarafından görülebilir**; ancak `RAWINPUT` yapılarında "injected" bayrağı veya `dwExtraInfo` yoktur. Bu yüzden Raw Input, motorun kendi enjeksiyonunu ayırt edemez; geri besleme döngüsünü önlemek için kanca (`dwExtraInfo` imzası) birincil okuma yolu olmalıdır. Bu, §6'daki tasarım kararının gerekçesidir.

### 5.5 Projeye etkisi

- Mimari değişmez: `SendInput` zaten seçilmişti; bu araştırma seçimi doğruladı.
- Uyumluluk matrisine "sentetik girdiyi reddeden uygulamalar" satırı eklenir.
- Efor etkisi: **0 saat** (kapsam dışı); yalnızca belge.

---

## 6. Windows'ta proje araçları için daha mantıklı yollar

Aşağıdaki tablo, Linux'taki mekanizmaların Windows karşılıklarını ve **neden** o yolun seçildiğini özetler.

| Linux mekanizması | Windows'ta en mantıklı yol | Gerekçe / not |
|---|---|---|
| `evdev` okuma | **`WH_KEYBOARD_LL` + `WH_MOUSE_LL`** (birincil) | Tetikleyici "tüketme" (suppression), `dwExtraInfo` ile kendi enjeksiyonunu ayırt etme ve `LLKHF_INJECTED` bilgisi yalnızca kancada var. |
| — (alternatif okuma) | `Raw Input` (`RegisterRawInputDevices`, `RIDEV_INPUTSINK`) | Arka planda da olay alır, aygıt kimliği verir; ancak **bastırma yok, injected bayrağı yok, `dwExtraInfo` yok**. Kayıt v2 / teşhis için opsiyonel; geri besleme riski nedeniyle varsayılan değil. |
| `uinput` enjeksiyonu | `SendInput`: klavye `KEYEVENTF_SCANCODE`, metin `KEYEVENTF_UNICODE`, fare `MOUSEEVENTF_*` | Düzen bağımsız tarama kodu; Unicode metin; imleç için mutlak sanal masaüstü koordinatı. |
| İmleç konumu (KWin fake-input) | `GetCursorPos` / `SetCursorPos` + `SendInput` mutlak hareket | `SetCursorPos` yalnızca hizalama/başlangıç için; asıl hareket `SendInput` ile üretilmeli ki uygulamalar gerçek fare olayı görsün. |
| Metin (`TEXT` eylemi) | `KEYEVENTF_UNICODE` birincil; pano + Ctrl+V isteğe bağlı yedek | Unicode çoğu uygulamada çalışır; pano yedeği pano içeriğini değiştirir → **kullanıcı onayı + geri yükleme** şartı. UIA `ValuePattern` v2. |
| `inotify` + `SIGUSR1` | `ReadDirectoryChangesW` (debounce) + adlandırılmış kanal `reload` | Aynı sözleşme; kanal mesajı atomik ve doğrulanabilir. |
| `flock` tek örnek | Adlandırılmış mutex (`Local\MacroStat-<SID>`) | Kullanıcı oturumu kapsamı; terk edilmiş mutex durumu ele alınır. |
| `PR_SET_PDEATHSIG` | Job Object (`KILL_ON_JOB_CLOSE`) + ebeveyn PID izleme + kanal kopması | Üç katmanlı: normalde temiz çıkış, son çare sert sonlandırma; her durumda basılı tuşları bırak. |
| Unix soketi | Adlandırılmış kanal: `\\.\pipe\macrostat-<SID>-<token>`, JSON satırları, SID DACL, `PIPE_REJECT_REMOTE_CLIENTS` | Kimlik doğrulama + uzak erişimin reddi. |
| Zamanlayıcı / jitter | `QueryPerformanceCounter` + `timeBeginPeriod(1)` + yüksek çözünürlüklü waitable timer; son <1 ms için kısa spin | `timeBeginPeriod` Win10 2004'ten itibaren **süreç bazlı**; Win11'de pencere tamamen örtülürse çözünürlük garantisi kalkar → motorun görünür/aktif olması gerekmez, ancak uyku/askıya alma senaryoları test edilir. |
| Etkin pencere / süreç kimliği | `GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageNameW` + token SID + süreç oluşturma zamanı | PID yeniden kullanımına karşı oluşturma zamanı; SID ile aynı kullanıcı doğrulaması. |
| DPI / çoklu monitör | `PerMonitorV2` (manifest + `SetProcessDpiAwarenessContext`), fiziksel piksel, `SM_XVIRTUALSCREEN` vb. | Negatif koordinatlar ve karışık DPI. |
| Uygulama başlatma | `ShellExecuteEx` / `CreateProcessW`, açık çalışma dizini, kabuk yorumu yok | Argümanlar dizi olarak; kabuk enjeksiyonu yok. |
| Günlük | Dönüşümlü dosya (1 MB), kullanıcıya özel ACL, UTF-8 | Event Log zorunlu değil; çevrimdışı ve taşınabilir. |
| Otomatik başlatma | İsteğe bağlı, kullanıcı bazlı `Run` anahtarı veya Başlangıç klasörü | Varsayılan kapalı; kaldırma betiği temizler. |
| Paketleme | PyInstaller onedir + Inno Setup (kullanıcı bazlı) | Değişmedi. |

### 6.1 Kanca mı Raw Input mu? — karar

| Ölçüt | Düşük seviye kanca | Raw Input |
|---|---|---|
| Tetikleyiciyi tüketme (bastırma) | **Var** | Yok |
| Kendi enjeksiyonunu ayırt etme | **Var** (`dwExtraInfo`, `LLKHF_INJECTED`) | Yok |
| Arka planda okuma | Var (global kanca) | `RIDEV_INPUTSINK` ile var |
| Aygıt kimliği | Sınırlı | **Var** |
| Yükseltilmiş ön plan penceresi | Erişilebilirlik SSS'ine göre normal yetkide çalışmaz | Belgede açık hüküm yok; **test edilecek** |
| Gecikme / zaman aşımı riski | `LowLevelHooksTimeout` riski | Düşük |

**Karar:** Motor varsayılan olarak kancayı kullanır; callback'te yalnızca kuyruğa yazar. Raw Input, yalnızca kayıt modu için opsiyonel bir "cihaz bilgisi" kaynağı olarak değerlendirilir; geri besleme döngüsü nedeniyle varsayılan yapılmaz.

---

## 7. Güncellenen tasarım kararları

| No | Karar | Durum |
|---|---|---|
| D-01 | UIPI engeli `GetLastError` ile algılanmaz. Ön plandaki pencerenin bütünlük düzeyi **önsezilir**; rozet + günlük; `SendInput` sayaç karşılaştırması yalnızca genel uyarı üretir. | **Kabul** (önceki tasarım düzeltildi) |
| D-02 | Yönetici hedefler için varsayılan davranış: çalışmama + açık uyarı. Kullanıcı isterse MacroStat'ı yönetici olarak çalıştırır; kalıcı yükseltme önerilmez. | **Kabul** |
| D-03 | `uiAccess` v1'de yok; imza + güvenli konum + yönetici başlatma koşullarıyla yol haritası. | **Kabul** |
| D-04 | Güvenli masaüstü erişimi hedeflenmez; `SendSAS` kullanılmaz; takılı tuş temizliği ve çökme dayanıklılığı test edilir. | **Kabul** |
| D-05 | Gerçek sanal HID ve çekirdek sürücüleri kalıcı olarak kapsam dışı; `SendInput` tek enjeksiyon yolu. | **Kabul** |
| D-06 | Okuma: kanca birincil; Raw Input opsiyonel/teşhis. Metin: Unicode birincil; pano isteğe bağlı yedek. Zamanlama: QPC + 1 ms periyot + waitable timer. | **Kabul** |

---

## 8. Doğrulanacaklar (Faz 0 kabul testleri)

1. **Yükseltilmiş ön planda kanca davranışı** — Win10 22H2 ve Win11 x64'te yönetici CMD/Notepad açıkken: kanca olay alıyor mu, tetikleyici çalışıyor mu, kayıt duruyor mu? (Beklenti: hayır.)
2. **Yükseltilmiş ön planda Raw Input** — `RIDEV_INPUTSINK` ile olay geliyor mu? Belge sessiz; ölçülecek.
3. **MacroStat yönetici olarak çalışırken** — kanca + `SendInput` yönetici hedefte çalışıyor mu? (Beklenti: evet.)
4. **Bütünlük düzeyi önsezisi** — `OpenProcess`/`OpenProcessToken` erişim reddi ile High IL tespiti güvenilir mi? Yanlış pozitif oranı.
5. **`SendInput` sayaç davranışı** — UIPI engelinde dönüş değeri gerçekten yanıltıcı mı? (Belgeye göre evet; saha doğrulaması.)
6. **Metin yolları** — Notepad, Chrome/Edge, Word, VS Code, Windows Terminal, yönetici CMD: Unicode ile mi, panoyla mı?
7. **Zamanlama** — 1 ms periyot altında 1000 olaylık jitter ölçümü; Win11'de pencere örtük/minimize iken sapma.
8. **Güvenli masaüstü** — UAC istemi ve kilit ekranı açıkken kanca olay almıyor, motor çökmüyor, basılı tuşlar bırakılıyor.
9. **Anti-cheat/AV** — Windows Defender ve yaygın oyunlarda imzasız `SendInput` davranışı; tespit **atlatılmayacak**, yalnızca belgelenecek.

---

## 9. Açık noktalar

- Raw Input'un yükseltilmiş ön plan penceresinde çalışıp çalışmadığı **belgelenmemiş**; ampirik sonuç mimariyi değiştirmez (kanca birincil) ama kayıt v2 için önemli.
- Interception'ın modern Windows imza durumu upstream belgede yok; üçüncü taraf sürücü kapsam dışı olduğu için takip edilmeyecek.
- Bütünlük düzeyi önsezisi, korumalı süreçlerde (PPL) yanlış "bilinmiyor" sonucu verebilir; kabul edilebilir.
- `uiAccess` yol haritası, kod imzalama kararına bağlıdır (Türkiye'de OV + bulut HSM).

---

## 10. Kaynaklar

- Microsoft Learn — `SendInput`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput
- Microsoft Learn — `SendSAS`: https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-sendsas
- Microsoft Learn — `RAWINPUTDEVICE`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawinputdevice
- Microsoft Learn — `timeBeginPeriod`: https://learn.microsoft.com/en-us/windows/win32/api/timeapi/nf-timeapi-timebeginperiod
- Microsoft Learn — How UAC works: https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/how-it-works
- Microsoft Learn — Security Considerations for Assistive Technologies (`uiAccess`): https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-securityoverview
- Microsoft Learn — Application manifests (`requestedExecutionLevel`, `uiAccess`): https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests
- Microsoft Learn — Virtual HID Framework: https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/virtual-hid-framework--vhf-
- AutoHotkey v2 SSS (yükseltilmiş pencerelerde kanca/enjeksiyon sınırı): https://www.autohotkey.com/docs/v2/FAQ.htm
- ViGEmBus (emekli, oyun kolu): https://github.com/nefarius/ViGEmBus
- Interception (çekirdek filtre sürücüsü): https://github.com/oblitum/Interception

---

**Rapor sonu.** Bu adımda kod yazılmadı; yalnızca araştırma yapıldı ve tasarım kararları güncellendi. `linux/` ağacında hiçbir dosya değiştirilmedi.
