# Adım Raporu — Faz 1 Motor Çekirdeği

> **Tarihsel belge.** Bu belge, hazırlandığı tarihteki durumu ve planları
> yansıtır; güncel proje durumunun veya güncel mimarinin kesin kaynağı
> değildir. Uygulanmamış planlar güncel özellik olarak
> değerlendirilmemelidir. Güncel durum için depo kökündeki `README.md` ve
> `windows/README.md`; güncel mimari için `windows/docs/WINDOWS-ARCHITECTURE.md`
> belgelerine bakın. İçerik geriye dönük güncellenmemiştir; metinde geçen
> `linux/` yolları ayrı Linux projesine aittir ve bu depoda bulunmaz.


| | |
|---|---|
| **Tarih** | 10 Kasım 2026 |
| **Faz** | 1 — Motor çekirdeği |
| **Durum** | Tamamlandı |
| **Kapsam** | `profile`, `keymap`, `trigger`, `actions` (platformdan bağımsız); Windows VK/tarama tablosu; 57 birim testi |
| **Kod durumu** | Çekirdek derleniyor ve test ediliyor; Windows API katmanı yok (Faz 2) |

## 1. Özet

Faz 1'in kabul ölçütü olan "profil, keymap, trigger, actions platformdan bağımsız Rust olarak yazıldı; birim testleri yeşil; Linux mantığıyla eşdeğerlik tablosu var" sağlandı. Tüm çekirdek Linux üzerinde `cargo test` ile doğrulanıyor; Windows'a özgü tek veri, Faz 2'de doğrulanacak bildirimsel VK/tarama tablosudur. `linux/` ağacına dokunulmadı.

**Sonuçlar:** `cargo fmt --check` ✅ · `cargo clippy --all-targets -- -D warnings` ✅ (0 uyarı) · `cargo test --all` ✅ **57/57** · üretici `--check` ✅.

## 2. Üretilen/değişen dosyalar

| Dosya | İçerik |
|---|---|
| `engine/src/lib.rs` | Kütüphane kökü; saf çekirdek + Faz 2 iskelet modülleri |
| `engine/src/main.rs` | Faz 1 bilgi çıktısı; varsayılan profili derleyip tetikleyici sayısını yazar |
| `engine/src/profile.rs` | Linux ile aynı serde modeli, camelCase + snake_case alias, varsayılanlar, çoklu profil seçimi, sınır sabitleri, gömülü varsayılan profil |
| `engine/src/keymap.rs` | **Üretilmiş** 119 kanonik tuş + alias çözümleyici + Windows VK/tarama tablosu + sol/sağ eşdeğerliği |
| `engine/src/trigger.rs` | Tetikleyici derleme ve 7 kipin saf durum makinesi; `TriggerCommand` sözleşmesi |
| `engine/src/actions.rs` | 11 eylem, `ActionGate`, iptal edilebilir uyku, jitter, takılı tuş temizliği; `Injector`/`Waiter`/`Cancel` trait'leri |
| `engine/tools/gen_keymap.py` | Sözlük + Windows eşlemesinden `keymap.rs` üreten, `--check` destekli üretici |
| `engine/assets/default_profile.json` | Linux asset'iyle **bayt-eş** kopya (test + yedek profil) |
| `engine/Cargo.toml` | `rand = "0.8"` eklendi (jitter; Linux ile aynı sürüm) |
| `docs/PROFILE-CONTRACT.md` | Çıplak rakam jetonu notu eklendi |
| `docs/COMPATIBILITY.md` | Aynı not + anahtar tablosu konumu güncellendi |
| `docs/TEST-MATRIX.md` | B-08…B-12 ve E-01…E-06 sonuçları işlendi |
| `ci/windows.yml` | `gen_keymap.py --check` adımı eklendi |
| `README.md` | Durum Faz 1, rapor bağlantısı, yerleşim güncellendi |

## 3. API sözleşmesi (Faz 2 için)

### 3.1 `trigger`

```rust
let (mut engine, compiled) = TriggerEngine::new(&profile);
let held = engine.held_handle();               // WHILE_HELD / HOLD işçileri için
match event {
    KeyDown => for cmd in engine.on_key(key, true, Instant::now()) { ... }
    KeyUp   => for cmd in engine.on_key(key, false, Instant::now()) { ... }
}
```

| Komut | Anlam / çalışma zamanı görevi |
|---|---|
| `Panic` | Enjekte edilen tuşları bırak, süreci sonlandır (Linux `exit(0)` karşılığı) |
| `RunOnce { .. }` | Eylem dizisini bir kez, `ActionGate` altında çalıştır |
| `StartLoop { repeat, flag, .. }` | İşçi başlat; `flag` düşünce dur ve tuşları bırak; `repeat = Some(n)` ise n kez |
| `StopLoop { .. }` | Aynı tetikleyiciye ikinci basış: işçiyi durdur, tuşları bırak |
| `StartWhileHeld { flag, .. }` | İşçi, gerekli tuşlar basılı oldukça döner; `flag` motor tarafından düşürülür |
| `StartHold { key, hold_ms, .. }` | Zamanlayıcı başlat; süre sonunda tuş hâlâ basılıysa çalıştır |
| `RunReleaseActions { .. }` | `releaseActions` dizisini bir kez çalıştır |

Motor iç durumu: basılı tuş kümesi (`Arc<RwLock<HashSet<Key>>>`), kombo mandalı, döngü bayrakları, WHILE_HELD çalışma kimlikleri, çift dokunma izleyicisi. Otomatik tekrar, "basılı tuşa yeniden basış" olarak yok sayılır — Linux'un `value == 2` filtresinin Windows karşılığı.

### 3.2 `actions`

```rust
let mut ctx = ActionContext { injector, pressed, waiter, cancel, jitter, diagnostics };
let completed = execute_actions(&macro.actions, &mut ctx); // false => iptal, tuşlar bırakıldı
```

Faz 2 `Injector`'ı `SendInput` ile, `Waiter`'ı QPC + waitable timer ile, `Cancel`'i panik/döngü bayrağıyla, `Jitter`'ı `RandJitter` ile uygular. Eylem dizileri `ActionGate::lock()` altında serileştirilir; kilit her dizi/iterasyon sonunda bırakılır.

## 4. Linux ↔ Windows eşdeğerlik tablosu

| Davranış | Linux | Faz 1 çekirdeği | Durum |
|---|---|---|---|
| Profil alanları/alias'ları | camelCase + snake_case | Aynı serde modeli | ✅ |
| Bilinmeyen alanlar | Yok sayılır | `serde` varsayılanı | ✅ |
| Çoklu profilde aktif seçim | `activeProfileId`, yoksa ilk | Aynı | ✅ |
| Panik tuşu yedeği | `KEY_PAUSE` | Aynı | ✅ |
| Tetikleyici dilbilgisi | kırp, büyüt, `KEY_` soy, alias | Aynı | ✅ |
| Sayısal evdev kodu | Kabul (`CODE_n`, `n`) | Reddedilir + uyarı | ⚠️ Belgelenmiş fark |
| Çıplak rakam `"0"`–`"9"` | Evdev kodu 0–9 | Reddedilir; `KEY_0`–`KEY_9` | ⚠️ Belgelenmiş fark |
| Geçersiz tetikleyici | Makro atlanır | Atlanır + yapılı uyarı | ✅ |
| Devre dışı makro | Derlenmez | Aynı | ✅ |
| `repeat_count`/çift dokunma/uzun basım tabanı | `max(1)` | Aynı | ✅ |
| 7 kipin hepsi | main loop | `TriggerEngine` | ✅ |
| Kombo mandalı | Tuş bırakınca açılır | Aynı | ✅ |
| Sol/sağ eşdeğerliği | Ctrl/Shift/Alt/Meta | Aynı | ✅ |
| Çift dokunma anahtarı | `macro:canonical_key` | `(macro, physical_class)` | ✅ |
| Çift dokunma saklama | `max(5 s, pencere)` | Aynı | ✅ |
| Otomatik tekrar filtresi | evdev `value == 2` | Basılı tuşa ikinci basış | ✅ |
| `KEY_TAP` süreleri | 25 ms (min 10) + 10 ms | Aynı | ✅ |
| `MOUSE_CLICK` süreleri | 20 ms (min 10) + 10 ms | Aynı | ✅ |
| `DELAY` varsayılan/jitter | 20 ms; `[-j,+j]`, min 1 ms | Aynı | ✅ |
| `MOUSE_MOVE`/`SCROLL` varsayılanı | 0 / 0 | Aynı | ✅ |
| `MOUSE_PATH` | 200k nokta, dt 0–600 000, `start` clamps | Aynı | ✅ |
| `TEXT` | Yalnız ASCII; 16 ms/karakter, Shift için 8 ms | Tüm Unicode; 16 ms/karakter, Shift yok | ➕ İyileştirme |
| İptalde tuş bırakma | `release_all_keys` | Aynı | ✅ |
| Eylem serileştirme | `ACTION_LOCK` | `ActionGate` | ✅ |
| Varsayılan profil asset'i | `linux/assets/default_profile.json` | Bayt-eş kopya | ✅ |

> Çekirdek, `TEXT` için `Injector::text_char` çağırır; Windows Unicode paketi kullanacağı için ASCII sınırı yoktur. Linux tarafı değişmedi.

## 5. Windows anahtar tablosu

`engine/tools/gen_keymap.py`, `docs/KEY-DICTIONARY.tsv`'yi (119 kanonik tuş, alias'lar Linux çözümleyicisinden) ve üreticideki bildirimsel eşlemeyi birleştirir; `src/keymap.rs`'yi üretir. Üretici alias çakışmasını ve eksik eşlemeyi hata olarak reddeder.

| Grup | Adet | Kaynak |
|---|---|---|
| Harfler (`A`–`Z`) | 26 | VK = ASCII; Set-1 tarama kodu |
| Rakamlar (`0`–`9`) | 10 | VK = ASCII; Set-1 |
| F1–F12 | 12 | VK `0x70`–`0x7B`; tarama F1–F10 `0x3B`–`0x44`, F11/F12 `0x57`/`0x58` |
| F13–F24 | 12 | VK `0x7C`–`0x87`; **tarama yok** (yalnız VK yolu) |
| Noktalama | 11 | `VK_OEM_*`; Set-1 |
| Özel (Enter, Esc, ok, nav, vb.) | 19 | VK + Set-1; genişletilmiş bayrağı işaretli |
| Numpad | 16 | `VK_NUMPAD*`; Set-1 |
| Modifier | 8 | Sol/sağ VK; sağ taraf genişletilmiş |
| Fare (`BTN_*`) | 5 | VK yok; `inject` katmanı `MOUSEEVENTF_*` kullanır |

- `scan_code = None` "desteklenmiyor" demek değildir: `SendInput` VK yoluyla enjekte edilir. Adaylar: `KEY_PAUSE` (E1 dizisi) ve F13–F24.
- **Faz 2 doğrulaması:** her tuş için `MapVirtualKeyW(vk, MAPVK_VK_TO_VSC)` ile karşılaştırma; farklar günlüğe yazılır ve tablo düzeltilir. Bu doğrulama yapılmadan tablo "kesin" sayılmaz (TEST-MATRIX notu).

## 6. Testler

| Modül | Adet | Kapsam |
|---|---|---|
| `actions` | 15 | 11 eylem, varsayılan/min süreler, jitter, iptal + takılı tuş, `MOUSE_PATH` clamp/limit, metin Unicode, `ActionGate` |
| `keymap` | 11 | 119 tuş, tüm alias'lar, `KEY_` öneki tuzağı (`KEY_LEFT` → `BTN_LEFT`), sayısal ret, eşdeğerlik, fare, VK nokta kontrolleri |
| `profile` | 10 | Wrapper/aktif seçim, legacy profil, alias'lar, bilinmeyen alan, tüm eylem/kip yazımları, gömülü asset, panik yedeği, geçersiz belge |
| `trigger` | 21 | Derleme/uyarılar, 7 kip, kombo mandalı, sol/sağ, çift dokunma penceresi, WHILE_HELD durdurma, panik, yeniden yükleme |
| **Toplam** | **57** | |

## 7. Doğrulama komutları

```bash
cd windows/engine
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
python3 tools/gen_keymap.py --check
cargo run          # varsayılan profil: 2 makro, 0 etkin tetikleyici
```

| Kontrol | Sonuç |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ 0 uyarı |
| `cargo test --all` | ✅ 57/57 |
| `gen_keymap.py --check` | ✅ güncel (rustfmt uygulanmış çıktı) |
| `cargo run` | ✅ "2 macro(s) … 0 trigger(s)" |
| `linux/` dokunulmadı | ✅ en yeni dosya 9 Kasım 2026, bu çalışma 10 Kasım |

## 8. Kararlar ve notlar

1. **Windows eşlemesi çekirdekte, enjeksiyon Faz 2'de.** Tablo veri olarak hazır; `SendInput` çağrısı yok. Böylece tablo Linux CI'da test edilebiliyor, gerçek doğrulama Faz 2'ye kalıyor.
2. **Sayısal evdev kodları reddedilir.** Sözleşme gereği; uyarı `CompileWarningKind::NumericKey` ile yapılıdır. Çıplak rakam jetonları da aynı yola girer; rakam tuşları `KEY_0`–`KEY_9`.
3. **`KEY_LEFT` tuzağı bilinçli korundu.** Linux `KEY_` önekini soyduğu için `KEY_LEFT` fare sol tuşuna çözülür; sol ok `KEY_LEFT_ARROW`/`ARROWLEFT` olarak yazılır. Bu davranış sözleşmenin parçasıdır ve testle sabitlendi.
4. **`TEXT` Unicode'a açıldı.** Windows'ta karakter başına `KEYEVENTF_UNICODE` kullanılacak; çekirdek yalnız `text_char` çağırır. ASCII sınırı yok.
5. **Uzun basım zamanlayıcısı çalışma zamanında.** Motor `StartHold` üretir; Faz 2 işçisi süre sonunda `held` anlık görüntüsünü kontrol eder. Karar mantığı testli, uyku/thread sahipliği Windows katmanında.
6. **Uyarılar yapılı.** Geçersiz tetikleyici artık yalnız stderr satırı değil; `CompileWarning { source_index, macro_name, trigger_key, token, kind }` olarak log/GUI'ye taşınabilir.
7. **`schemaVersion` taşınır ama doğrulanmaz.** Linux ile aynı hoşgörü; yazıcı sürümü engine'i kilitlemez.
8. **Üretilmiş dosya.** `keymap.rs` elle düzenlenmez; CI `--check` ile sapmayı yakalar.

## 9. Faz 2'ye taşınanlar

1. `windows` crate sürüm/feature setinin doğrulanması; `hook`/`inject`/`mouse` gerçek uygulamaları.
2. VK/tarama tablosunun `MapVirtualKeyW` ile çapraz kontrolü ve raporlanması.
3. `SendInput` paketleri: tarama kodu + genişletilmiş bayrak; `MOUSEEVENTF_*`; Unicode metin; mutlak sanal masaüstü koordinatları (`MOUSEEVENTF_ABSOLUTE|VIRTUALDESK`, 0–65535 normalizasyonu).
4. Kanca geri besleme filtresi (`dwExtraInfo` imzası + `LLKHF_INJECTED`) ve `WHILE_HELD`/`HOLD` işçileri.
5. UIPI önsezisi (`GetForegroundWindow` → bütünlük düzeyi) ve günlük/rozet bağlantısı.

## 10. Değişmez kural kontrolü

- `linux/` altında hiçbir dosya yazılmadı/değiştirilmedi (mtime denetimi).
- Tüm yeni dosyalar `windows/` altında.
- Ağ bağlantısı eklenmedi; yeni bağımlılık yalnız `rand` (yerel PRNG).
