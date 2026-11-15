# Windows'a özgü Python testleri

İki test dosyası vardır:

| Dosya | İçerik | Nerede koşar |
|---|---|---|
| `test_gui_windows.py` | **55** test. Linux'taki sistem testlerinin Windows karşılıkları: kanal, mutex, Job Object, `%LOCALAPPDATA%` yolları; `sanitize`, `transfer`, `i18n`, `settings`; `runtime.json`, `--ipc` istemcisi, motor algılama, dosya okuma/yazma, giriş birleştirme, onboarding; PyInstaller/kurulum paket sözleşmesi | Her platformda |
| `test_e2e_windows.py` | **12** test metodu: 7'si gerçek `macrostat-engine.exe` üzerinde **9** kabul senaryosu (U-01…U-09), 5'i platformdan bağımsız regresyon testi (UTF-8 çıktı çözümlemesi, alt süreç borusu kapatma) | 7 E2E metodu yalnızca Windows'ta; regresyon testleri her platformda |

GUI testleri:

```
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -p "test_gui*.py" -v
```

E2E testleri (Windows; gerçek ikili gerekir):

```powershell
$env:MACROSTAT_E2E_ENGINE = "C:\...\macrostat-engine.exe"
python -m unittest discover -s windows/tests -p "test_e2e*.py" -v
```

Tam takım (depo kökünden):

```
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v
```

Windows dışında E2E sınıfı `skipped` olarak raporlanır; atlama "geçti" sayılmaz.
Girdi enjeksiyonu yapılamayan oturumlarda `MACROSTAT_E2E_SKIP_INPUT=1` kullanılır.

Testler `APPDATA`/`LOCALAPPDATA` değişkenlerini geçici dizine yönlendirir; gerçek
kullanıcı verisine dokunmaz. Motor gerektiren GUI testleri `unittest.mock` ile
sahte süreç kullanır.
