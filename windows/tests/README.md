# Windows'a özgü Python testleri — Faz 4–5

`test_gui_windows.py` tek dosyada **45 test**; Linux'taki sistem testlerinin Windows karşılıkları:

- Yol/ACL/PID/sinyal yerine kanal, mutex, Job Object, `%LOCALAPPDATA%` yolları.
- `sanitize`, `transfer`, `i18n` bütünlüğü (sabit `T()` metinlerinin tamamı `_EN`'de ve `{}` sayıları eşit), `settings` atomikliği.
- `runtime.json` tanımı, `--ipc` istemcisi (mock motora karşı), motor çalışıyor algısı, özel dosya okuma/yazma, giriş birleştirme, yol/anahtar yardımcıları, onboarding kontrolleri.
- Paket sözleşmesi: PyInstaller düzeni (`_internal/`), donmuş uygulama dizini, sürüm tutarlılığı (`VERSION` = `Cargo.toml` = `version_info.txt`), kurulumun kullanıcı kapsamlı olduğu.

Koşum (depo kökünden):

```
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v
```

Testler `APPDATA`/`LOCALAPPDATA` değişkenlerini geçici dizine yönlendirir; gerçek kullanıcı verisine dokunmaz. Motor gerektiren testler `unittest.mock` ile sahte süreç kullanır.
