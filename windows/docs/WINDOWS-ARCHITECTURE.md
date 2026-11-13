# Windows Mimarisi — Faz 0 Baseline

Bu belge Faz 0'da **dondurulan** mimariyi tanımlar. Uygulama ayrıntıları fazlarda eklenir; buradaki süreç, katman ve sözleşme sınırları değişmez.

## 1. Süreç modeli

```
MacroStat.exe  (PyQt6 GUI, asInvoker)
  │  CreateProcessW / subprocess
  ├── macrostat-engine.exe <profil.json>            normal mod
  ├── macrostat-engine.exe --test-macro <id>        tek makro, kilit/tek örnek yok
  ├── macrostat-engine.exe --record-input <dosya>   sistem geneli kayıt
  ├── macrostat-engine.exe --watch-restart <KEY_*>  sadece kanca; tuşta çıkış 42
  ├── macrostat-engine.exe --self-test              GUI'siz kanca/enjeksiyon tanısı
  └── macrostat-engine.exe --parent-pid <pid>       yaşam döngüsü bağlantısı
  │
  └── Adlandırılmış kanal: \\.\pipe\macrostat-<SID>-<token>
        ping | reload | stop | status   (satır tabanlı JSON)
```

- GUI motoru başlatır ve bir **Job Object**'e atar (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
- Nazik kapanış sırası: kanaldan `stop` → süreç bitişini bekle → gerekirse sonlandır.
- Motor ayrıca `--parent-pid` ile ebeveyni izler; GUI çökerse basılı tuşları bırakıp çıkar.
- Tek örnek: GUI `Local\MacroStat.Gui.<SID>`, motor `Local\MacroStat.Engine.<SID>` mutex'i.

Yetki modeli: her iki süreç de **asInvoker**. Yönetici yükseltme istenmez; kullanıcı isterse GUI'yi yönetici olarak başlatabilir (belgelenmiş sınır).

## 2. Katmanlar ve modül sınırları

| Katman | Modüller | Platform bağımlılığı |
|---|---|---|
| **Saf çekirdek** (Linux'ta test edilebilir) | `profile`, `keymap` (sembolik ad ↔ soyut tuş), `trigger`, `actions` (zamanlama/karar) | Yok |
| **Windows altyapısı** | `hook`, `inject`, `mouse`, `ipc`, `log`, `winutil`, `record`, `main` | Windows API |
| **GUI** | `gui/macrostat/*` (Faz 4) | PyQt6 + Windows yolları |

Kural: saf çekirdek Windows API'si **çağırmaz**. Girdi olayları soyut `InputEvent` olarak gelir; enjeksiyon `Injector` trait'i üzerinden yapılır. Böylece Faz 1'de birim testleri Linux CI'da da koşar.

## 3. Motor iş parçacıkları

| İş parçacığı | Görev | Kural |
|---|---|---|
| **Kanca/mesaj döngüsü** | `WH_KEYBOARD_LL` + `WH_MOUSE_LL`; olayı kuyruğa yazar | Callback'te **bloklayıcı iş yok**, yalnızca kuyruk + `dwExtraInfo` filtresi |
| **Tetikleyici işçisi** | Kuyruktan olay okur, tetikleyici durum makinesini çalıştırır | Düşük gecikme; kilit tutmaz |
| **Eylem işçisi** | `SendInput`, bekleme/jitter, iptal bayrağı | Tek eylem yürütücüsü; `ACTION_LOCK` semantiği |
| **IPC sunucusu** | Kanal bağlantısı, `ping/reload/stop/status` | Sabit komut kümesi, uzunluk/zaman aşımı sınırı |
| **Ebeveyn izleyici** | `OpenProcess(SYNCHRONIZE)` + bekleme | Ebeveyn çıkınca temiz kapanış |
| **Günlük/rotasyon** | 1 MB sınırı, kullanıcı ACL'i | Tuş içeriği loglanmaz |

Kanca durumu (kuruldu/kaldırıldı/zaman aşımı) GUI'ye `status` yanıtıyla bildirilir.

## 4. Veri akışı

```
 fiziksel girdi
      │
      ▼
 LL hook ──► kuyruk ──► tetikleyici derleyici/eşleştirici ──► eylem
      │                                                        │
      │  kendi enjeksiyonunu ayıkla                            ▼
      │  (dwExtraInfo == 0x4D535441)                     SendInput
      └──────────────────────────────────────────────► işletim sistemine
```

- Enjeksiyon imzası: `dwExtraInfo = 0x4D535441` ("MSTA"). Kanca bu imzayı taşıyan olayı **yok sayar**.
- `LLKHF_INJECTED` / `LLMHF_INJECTED` loglanır; başka araçların enjeksiyonu isteğe bağlı filtrelenir.
- Auto-repeat: kendi "basılı tuşlar" kümesi; kümede olan tuşun yeni `KEYDOWN`'ı tekrar sayılır.
- UIPI farkındalığı: enjeksiyon öncesi ön plandaki pencerenin bütünlük düzeyi sezilir; engel `GetLastError` ile **algılanamaz**.

## 5. Kalıcı kararlar (özet)

| Kod | Karar |
|---|---|
| D-01 | UIPI engeli teşhis edilemez → önsezi + rozet + günlük; `SendInput` sayaç karşılaştırması yalnızca genel uyarı |
| D-02 | Yönetici hedefler: varsayılan çalışmama + açık uyarı; isteğe bağlı yönetici çalıştırma |
| D-03 | `uiAccess` v1'de yok (imza + güvenli konum + yönetici başlatma) |
| D-04 | Güvenli masaüstü hedeflenmez; `SendSAS` kullanılmaz |
| D-05 | Sanal HID / çekirdek sürücüsü yok; tek enjeksiyon yolu `SendInput` |
| D-06 | Okuma: kanca birincil; Raw Input opsiyonel/teşhis |
| D-07 | Metin: `KEYEVENTF_UNICODE` birincil; pano yedeği yalnızca kullanıcı onayıyla |
| D-08 | Zamanlama: `QueryPerformanceCounter` + `timeBeginPeriod(1)` + yüksek çözünürlüklü waitable timer |
| D-09 | IPC: SID kısıtlı adlandırılmış kanal, `PIPE_REJECT_REMOTE_CLIENTS` |
| D-10 | Yaşam döngüsü: Job Object + ebeveyn izleme + kanal kopması |
| D-11 | DLL güvenliği: `SetDefaultDllDirectories` + tam yol `LoadLibraryEx` |
| D-12 | Profil şeması değişmez; `KEY_*`/`BTN_*` adları korunur |

## 6. Build çıktıları (Faz 5'te üretildi)

| Çıktı | Üreten |
|---|---|
| `macrostat-engine.exe` | `cargo build --release --locked` (MSVC hedefi, `build.ps1`) |
| `MacroStat.exe` + `_internal/` | PyInstaller 6 (onedir, `--windowed`, `gui/MacroStat.spec`) |
| `MacroStat-Setup-1.0.0.exe` | Inno Setup 6 (`PrivilegesRequired=lowest`) |
| `MacroStat-1.0.0-portable.zip` | `scripts/build.ps1` (`Compress-Archive`) |
| `SHA256SUMS.txt` | `scripts/build.ps1`; imza sonrası CI yeniden üretir |

Manifest (motor): `asInvoker`, `PerMonitorV2`, `longPathAware`, `supportedOS` Win10/11. İmza Faz 5+'ta `signtool` ile eklenebilir; ilk sürüm imzasız.

## 7. Faz sınırları

- **Faz 0:** sözleşmeler, iskelet, belgeler, CI taslağı. İş mantığı yok.
- **Faz 1:** saf çekirdek (`profile`, `keymap`, `trigger`, `actions`) + birim testleri.
- **Faz 2:** Windows girdi backend'i (`hook`, `inject`, `mouse`, `record`, `winutil`).
- **Faz 3:** IPC, yaşam döngüsü, günlük.
- **Faz 4:** GUI taşıma.
- **Faz 5:** paketleme/CI.
- **Faz 6–8:** güvenlik, E2E, sürüm.
