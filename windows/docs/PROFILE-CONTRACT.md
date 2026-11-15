# Profil ve Anahtar Sözleşmesi — Donmuş Baseline (v1)

Bu sözleşme **Faz 0'da dondurulmuştur**. Windows motoru ve arayüzü bu biçimi üretmek ve okumak zorundadır. Linux sürümüyle dosya alışverişi iki yönlü korunur.

- Makine-okunur şema: [`profile.schema.json`](profile.schema.json)
- Anahtar sözlüğü: [`KEY-DICTIONARY.tsv`](KEY-DICTIONARY.tsv) (Linux motorunun `string_to_key` çözümleyicisinden üretilmiştir)
- Kaynak: Bu sözleşme, MacroStat Linux projesinin motor çözümleyicisi, profil
  modeli, içe/dışa aktarma kodu ve varsayılan profili temel alınarak
  dondurulmuştur. Linux projesi ayrı bir depodur ve bu depoda bulunmaz; dosya
  yolları burada tekrarlanmaz.

## 1. Dosya yerleşimi

| Dosya | Windows yolu |
|---|---|
| Profil | `%APPDATA%\MacroStat\profile.json` |
| Ayarlar | `%APPDATA%\MacroStat\settings.json` |
| Günlük, PID, kanal token'ı, yedekler | `%LOCALAPPDATA%\MacroStat\` |
| İçe/dışa aktarma | Kullanıcının seçtiği yol |

Motor yalnızca kendisine argümanla verilen profil dosyasını okur; dizinleri kendisi keşfetmez.

## 2. Sarmalayıcı (üst düzey)

```json
{
  "schemaVersion": 1,
  "activeProfileId": "profile-default",
  "profiles": [ { ... } ]
}
```

| Alan | Tip | Zorunlu (yazan) | Not |
|---|---|---|---|
| `schemaVersion` | int | Evet | Yazıcılar **1** üretir. Motor değeri doğrulamaz (ileri uyumluluk). |
| `activeProfileId` | string \| null | Hayır | Geçerli bir profil `id`'sine işaret eder. Yoksa/geçersizse ilk profil kullanılır. |
| `profiles` | array | Evet | İçe aktarma sınırı: 100 profil. |

Motor bilinmeyen üst düzey alanları yok sayar (serde varsayılanı).

## 3. Profil nesnesi

| Alan | Tip | Varsayılan | Sınır / not |
|---|---|---|---|
| `id` | string | `""` | `[A-Za-z0-9_-]{1,64}` (bkz. §7) |
| `name` | string | `""` | Arayüzde 200 karaktere kırpılır |
| `description` | string | — | Arayüz alanı; motor yok sayar; 4000 karakter |
| `panicKey` | string | `""` | Tek tuş veya kombinasyon; STOP tuşu |
| `restartKey` | string | — | Arayüz alanı; motor yok sayar (GUI `--watch-restart` için kullanır) |
| `virtualDeviceName` | string | `""` | Windows'ta **kozmetik**; aygıt oluşturulmaz |
| `defaultJitterMs` | int | `2` | Arayüz alanı (0–1000); yeni `DELAY` eylemlerine öneri |
| `macros` | array | `[]` | İçe aktarma sınırı: 500 makro |

Motorun `ProfileConfig` yapısı yalnızca `id`, `name`, `panicKey`, `virtualDeviceName`, `macros` alanlarını okur; diğerleri korunur ve GUI tarafından yönetilir.

## 4. Makro nesnesi

| Alan | Tip | Varsayılan | Not |
|---|---|---|---|
| `id` | string | `""` | Boşsa motor `idx-<sıra>` üretir |
| `name` | string | `"Yeni Makro"` | |
| `description` | string | — | Arayüz alanı |
| `enabled` | bool | `true` | `false` ise motor derlemez |
| `triggerKey` | string | `""` | §6 dilbilgisi; geçersizse makro atlanır + uyarı |
| `triggerMode` | enum | `ONCE` | §5 |
| `repeatCount` | int | `1` | Motor `max(1)`; arayüz 1–9999 |
| `doubleTapTimeoutMs` | int | `300` | Motor `max(1)`; arayüz 50–5000 |
| `holdDurationMs` | int | `300` | Motor `max(1)`; arayüz 50–10000 |
| `actions` | array | `[]` | §5 eylemleri |
| `releaseActions` | array | `[]` | `WHILE_HELD` bırakılışında yürütülür |
| `createdAt` | int (ms) | — | Arayüz alanı |
| `updatedAt` | int (ms) | — | Arayüz alanı |

## 5. Eylemler

`type` (SCREAMING_SNAKE_CASE, alias'lar kabul edilir):

| `type` | Kullanılan alanlar | Davranış / varsayılan |
|---|---|---|
| `KEY_DOWN` / `KEY_UP` | `key` (veya `mouseButton`) | Basılı tut / bırak |
| `KEY_TAP` | `key`, `delayMs` | Bas–bekle–bırak; `delayMs` varsayılan **25 ms**, en az 10 ms; sonra 10 ms bekleme |
| `MOUSE_DOWN` / `MOUSE_UP` | `mouseButton` (veya `key`) | Basılı tut / bırak |
| `MOUSE_CLICK` | `mouseButton`, `delayMs` | Varsayılan **20 ms**, en az 10 ms; sonra 10 ms bekleme |
| `MOUSE_MOVE` | `dx`, `dy` | Göreli hareket; eksik alan 0 |
| `MOUSE_SCROLL` | `scrollDelta` | Dikey tekerlek; eksik alan 0; birim "çentik" |
| `MOUSE_PATH` | `points`, `start` | §5.1 |
| `TEXT` | `text` | UTF-8, en fazla **65 536 bayt**. Linux yalnızca US-ASCII yazar; Windows Unicode yazar (bkz. §8). |
| `DELAY` | `delayMs`, `jitterMs` | Varsayılan 20 ms; jitter varsayılan 0; `±jitterMs` düzgün dağılım |

Tüm eylemlerde `id` isteğe bağlıdır; içe aktarmada yeniden üretilir.

`key`/`mouseButton` çözümlenemezse eylem **sessizce atlanır** (motor mevcut davranışı). Faz 2'de Windows tarafında da aynı davranış korunur ve günlüğe tek satır yazılır.

### 5.1 `MOUSE_PATH`

```json
{"type":"MOUSE_PATH","start":[100,200],"points":[[0,0,0],[16,5,-2],[16,0,0]]}
```

- `points`: `[geçen_ms, dx, dy]` üçlüleri. İlk noktanın `geçen_ms` değeri kayıt başlangıcından itibaren beklemedir.
- En fazla **200 000** nokta işlenir; fazlası kırpılır.
- `geçen_ms` 0–600 000 aralığına sıkıştırılır; `dx/dy` i32 aralığına.
- `dx == 0 && dy == 0` olan noktalar yalnızca bekleme olarak işlenir.
- `start` varsa oynatma başında imleç oraya taşınır (Windows'ta `SetCursorPos` ile hizalama).
- Kayıt sınırları: 600 s, 200 000 nokta, 4 ms örnekleme.

## 6. Tetikleyici dilbilgisi (donmuş)

```
triggerKey := part ("+" part)*
part       := boşluk kırpılır, büyük harfe çevrilir
             | "KEY_" öneki isteğe bağlı
             | sembolik ad (KEY-DICTIONARY)
             | "CODE_<n>" veya "<n>"   (Linux evdev kodu; yalnızca Linux'ta anlamlı)
```

- En az bir, en çok pratikte 8 tuş; boş parça yok sayılır.
- Herhangi bir parça çözümlenemezse **tüm tetikleyici geçersizdir** (makro atlanır, uyarı).
- Sol/sağ eşdeğerliği: `LEFTSHIFT` ↔ `RIGHTSHIFT`, `LEFTCTRL` ↔ `RIGHTCTRL`, `LEFTALT` ↔ `RIGHTALT`, `LEFTMETA` ↔ `RIGHTMETA`.
- Örnekler: `KEY_F8`, `KEY_LEFTSHIFT+KEY_LEFTCTRL`, `BTN_LEFT`, `KEY_F9+MOUSE1`.

> Windows'ta sayısal evdev kodları (`123`, `CODE_123`) **desteklenmez**; yükleyici uyarı verip makroyu devre dışı bırakır. Yazıcılar her zaman sembolik ad üretir.
>
> **Rakam tuşları:** Linux çözümleyicisi sayısal kodu sembolik addan önce denediği için çıplak `"0"`–`"9"` jetonları evdev kodu olarak yorumlanır; rakam tuşları her iki platformda da `KEY_0`–`KEY_9` yazılmalıdır. Windows çözümleyicisi aynı sırayı izler ve çıplak rakamı reddeder (bkz. `keymap.rs`).

## 7. Kimlikler

- Biçim: `^[A-Za-z0-9_-]{1,64}$`
- Kimlikler komut satırına geçebilir; kontrol/bidi karakteri yasak.
- İçe aktarmada çakışan kimlikler yeniden üretilir ve makrolar **devre dışı** yüklenir.
- Windows motoru `id + "#" + index` biçiminde iç kimlik kullanır (Linux ile aynı).

## 8. Taşınabilirlik ve sınırlar

| Konu | Kural |
|---|---|
| Şema | Windows ve Linux aynı JSON'u okur/yazar. |
| Bilinmeyen alanlar | Motor yok sayar; GUI düzenlerken korur; içe aktarma süzgeci atar. |
| `virtualDeviceName` | Windows'ta kozmetik; alan korunur. |
| Unicode `TEXT` | Windows'ta desteklenir; Linux yalnızca ASCII. Windows'ta yazılan Unicode metin Linux'ta açıldığında desteklenmeyen karakterler uyarıyla atlanır. |
| Sayısal tuş kodu | Linux'a özgü; Windows'ta devre dışı + uyarı. |
| Dosya boyutu | Motor en fazla 32 MiB profil okur; içe aktarma 5 MiB ile sınırlıdır. |
| Metin | 64 KiB (UTF-8) üstü içe aktarmada reddedilir. |

## 9. Değişmezler

1. Alan adları **camelCase**'dir; okuma tarafı `snake_case` alias'larını kabul eder.
2. `triggerMode` ve `type` değerleri **SCREAMING_SNAKE_CASE** yazılır; okuma alias'ları esnektir.
3. Varsayılanlar yukarıdaki tablolarla aynıdır; Windows tarafı yeni varsayılan **uydurmaz**.
4. `schemaVersion` artırılmadan alan anlamı değiştirilemez; yeni alanlar geriye uyumlu olmalıdır.
