# Linux ↔ Windows Uyumluluğu

Amaç: **işlevsel parite** + Windows'a özgü iyileştirmeler. Profil dosyası iki yönlü taşınabilir.

## 1. Yetenek eşlemesi

| Yetenek | Linux | Windows | Durum |
|---|---|---|---|
| Sistem geneli klavye okuma | `evdev` (udev `uaccess`) | `WH_KEYBOARD_LL` | ✅ Parite |
| Fare okuma | `evdev` | `WH_MOUSE_LL` | ✅ Parite |
| Tuş enjeksiyonu | `uinput` | `SendInput` (`KEYEVENTF_SCANCODE`) | ✅ Parite |
| Fare enjeksiyonu | `uinput` | `SendInput` (`MOUSEEVENTF_*`) | ✅ Parite |
| Metin yazma | US-ASCII (`uinput`) | Unicode (`KEYEVENTF_UNICODE`) | ✅ + iyileştirme |
| Kayıt | Yalnızca pencere içi | Sistem geneli (kanca) | ✅ + iyileştirme |
| İmleç konumu | X11 / KDE Wayland koşullu | `GetCursorPos` her zaman | ✅ + iyileştirme |
| Yükseltilmiş pencereler | — (kavram yok) | **Çalışmaz** (UIPI) | ⚠️ Belgelenmiş sınır |
| Güvenli masaüstü | — | **Çalışmaz** | ⚠️ Belgelenmiş sınır |
| Sanal HID | `uinput` | Yok (`SendInput` sentetik) | ⚠️ Kapsam dışı |
| Çoklu monitör | X11/Wayland'a bağlı | Sanal masaüstü koordinatları | ✅ Parite |
| Yüksek DPI | Ortam değişkenleri | PerMonitorV2 | ✅ + iyileştirme |
| Tek örnek | `flock` | Adlandırılmış mutex | ✅ Parite |
| Sinyal ile yeniden yükleme | `SIGUSR1` | Kanal `reload` | ✅ Parite |
| Ebeveyn çökünce temiz çıkış | `PR_SET_PDEATHSIG` | Job Object + PID izleme | ✅ Parite |
| Profil izleme | `inotify` | `ReadDirectoryChangesW` | ✅ Parite |
| Günlük | Dosya + rotasyon | Dosya + rotasyon + kullanıcı ACL | ✅ Parite |
| Otomatik başlatma | `.desktop` | `HKCU\...\Run` (isteğe bağlı) | ➕ Opsiyonel |
| Sistem tepsisi | — | `QSystemTrayIcon` (isteğe bağlı) | ➕ Opsiyonel |

## 2. Anahtar adları

- Profil içinde **Linux evdev adları** (`KEY_*`, `BTN_*`) korunur; bunlar sözleşmenin parçasıdır.
- Windows motoru bu adları VK/tarama koduna eşler. Eşleme tablosu Faz 1'de `engine/src/keymap.rs` + `docs/KEY-DICTIONARY.tsv` üzerinden üretilir.
- Eşlemesi olmayan tuşlar (ör. bazı medya/tuş takımı tuşları) yüklenirken uyarı verir; makro devre dışı kalır, profil bozulmaz.
- Sayısal evdev kodları (`123`, `CODE_123`) Windows'ta desteklenmez.
- Çıplak rakam jetonları (`"0"`–`"9"`) Linux'ta sayısal kod sayıldığı için rakam tuşları `KEY_0`–`KEY_9` yazılmalıdır; Windows çözümleyicisi aynı sırayı izler ve çıplak rakamı reddeder.
- Sol/sağ eşdeğerliği korunur (Shift/Ctrl/Alt/Meta).

## 3. Dosya biçimi

| Öğe | Durum |
|---|---|
| JSON alan adları | Aynı (camelCase; okuma alias'lı) |
| `schemaVersion` | 1 |
| `virtualDeviceName` | Windows'ta kozmetik |
| `MOUSE_PATH.start` | Windows'ta her zaman yazılabilir |
| `TEXT` Unicode | Windows'ta yazılır; Linux'ta ASCII dışı atlanır |
| Yedekler | Aynı dizinde, `.bak`/zaman damgalı |

## 4. Kasıtlı farklar

| Fark | Gerekçe |
|---|---|
| Yönetici uygulamalarda çalışmama | UIPI; arayüzde rozet + günlük |
| UAC/kilit ekranında çalışmama | Güvenli masaüstü; güvenlik gereği |
| Gerçek sanal aygıt yok | VHF çekirdek modu; kapsam dışı |
| `TEXT` Unicode | Windows iyileştirmesi; Linux profili etkilenmez |
| Uzun yol desteği (`longPathAware`) | Windows'a özgü manifest |
| Koyu başlık çubuğu | Windows'a özgü kozmetik |

## 5. Test edilecek uyumluluk noktaları

1. Linux'ta üretilen varsayılan profil Windows'ta hatasız yüklenir.
2. Windows'ta üretilen profil Linux'ta (ASCII dışı metin yoksa) yüklenir.
3. TR-Q ve US klavye düzenlerinde tarama kodu tabanlı enjeksiyon aynı karakteri üretir.
4. Unicode metin Notepad/Chrome/Terminal'de doğru yazılır.
5. `MOUSE_PATH` çoklu monitörde negatif koordinatlarla doğru oynatılır.
