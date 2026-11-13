# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Windows arayüzü için ortak katman: yollar, motor denetimi ve dosya I/O.

Linux sürümündeki `/proc`, `fcntl`, `signal` ve udev kullanımının
karşılığıdır:

* Motorla iletişim, motor ikilisinin kendi ``--ipc`` istemcisi üzerinden
  yapılır; `runtime.json` kimlik doğrulaması (SID + kanal belirteci) Rust
  tarafında kalır.
* Süreç kimliği `winplatform` modülündeki sabit Win32 çağrılarıyla
  doğrulanır; PID yeniden kullanımı oluşturulma zamanıyla ayıklanır.
* Sembolik bağlantı/junction reddi `O_NOFOLLOW` yerine
  `FILE_ATTRIBUTE_REPARSE_POINT` denetimiyle yapılır.
* Dosyalar yine atomik yazılır (geçici dosya + `fsync` + `os.replace`);
  Windows'ta 0600 yerine kullanıcıya özel `%LOCALAPPDATA%` ACL'leri geçerlidir.

Bu modül kullanıcı girdisini komut satırına geçirmez: yalnızca sabit
motor seçenekleri ve doğrulanmış yollar kullanılır.
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from . import winplatform
from .i18n import LANGUAGES, T, available_languages, get_language, set_language
from .sanitize import (
    TRIGGER_MODES, coerce_int, is_plain_key_name, is_valid_id,
    is_valid_key_name, is_valid_trigger, sanitize_display_text)
from .settings import (
    CONFIG_DIR, DATA_DIR, SETTINGS_PATH, THEME_MODES, ensure_dirs,
    load_settings, save_settings)

#: Motorun yürütülebilir dosya adı (`windows/engine`, MSVC hedefi).
DAEMON_NAME = "macrostat-engine.exe"

#: Motor süreç tutamacı; `system.py` tarafından atanır (QProcess).
DAEMON_PROC = None

#: Günlük bu boyutu aşınca döndürülür (motorun kendi eşiğiyle aynı).
LOG_MAX_BYTES = 1_000_000

#: Profil dosyası için üst sınır (sözleşmedeki 32 MiB).
MAX_PROFILE_BYTES = 32 * 1024 * 1024

#: `runtime.json` küçük bir tanıtım dosyasıdır.
RUNTIME_MAX_BYTES = 64 * 1024

#: STOP sonrası motoru yeniden başlatan tuşun varsayılanı.
DEFAULT_RESTART_KEY = "KEY_F12"
#: `--watch-restart` çıkış kodu: GUI motoru yeniden başlatır.
RESTART_WATCH_EXIT_CODE = 42

#: Panik (her şeyi durdur) tuşunun varsayılanı.
PANIC_KEY = "KEY_PAUSE"


def _app_dir():
    """Uygulama dizini: PyInstaller'da EXE'nin yanı, geliştirmede `gui/`."""
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parent.parent


#: Depo/geliştirme düzeninde `windows/gui`, paketlenmiş uygulamada kurulum
#: dizini. Varlıklar (`assets/`) ve motor adayları buna göre aranır.
SCRIPT_DIR = _app_dir()

#: Motorun günlük ve `runtime.json` dosyalarını yazdığı dizin.
RUNTIME_DIR = DATA_DIR
RUNTIME_FILE = RUNTIME_DIR / "runtime.json"

#: Motor günlüğü (motor varsayılan olarak aynı yolu kullanır).
LOG_FILE = RUNTIME_DIR / "engine.log"

#: Kullanıcı profili; ilk açılışta şablondan tohumlanır.
PROFILE_PATH = DATA_DIR / "profile.json"
PROFILE_BACKUP_PATH = DATA_DIR / "profile.json.bak"


def engine_candidates():
    """Motor ikilisi için sabit, PATH'ten bağımsız aday listesi.

    ``shutil.which`` kullanılmaz: PATH ele geçirilirse arayüz rastgele bir
    çalıştırılabiliri motor sanıp başlatabilirdi.
    """
    candidates = [SCRIPT_DIR / DAEMON_NAME]
    candidates.append(SCRIPT_DIR / "_internal" / DAEMON_NAME)
    candidates.append(SCRIPT_DIR / "engine" / DAEMON_NAME)
    candidates.append(SCRIPT_DIR / "target" / "release" / DAEMON_NAME)
    candidates.append(SCRIPT_DIR / "target" / "debug" / DAEMON_NAME)
    parent = SCRIPT_DIR.parent
    candidates.append(parent / "engine" / "target" / "release" / DAEMON_NAME)
    candidates.append(parent / "engine" / "target" / "debug" / DAEMON_NAME)
    candidates.append(parent / "engine" / DAEMON_NAME)
    return candidates


def _trusted_engine(path):
    """Aday yol gerçek bir dosyaysa ve yeniden ayrıştırma noktası değilse döner."""
    try:
        if not path.is_file():
            return None
        if winplatform.file_has_reparse_point(path):
            return None
    except OSError:
        return None
    return path.resolve()


def find_engine():
    """İlk geçerli motor ikilisi; bulunamazsa ``None``."""
    for candidate in engine_candidates():
        resolved = _trusted_engine(candidate)
        if resolved is not None:
            return resolved
    return None


def _default_profile_candidates():
    parent = SCRIPT_DIR.parent
    return (
        SCRIPT_DIR / "assets" / "default_profile.json",
        SCRIPT_DIR / "_internal" / "assets" / "default_profile.json",
        parent / "engine" / "assets" / "default_profile.json",
        parent / "assets" / "default_profile.json",
        PROFILE_PATH,
    )


def default_profile_path():
    """Tek kanonik profil şablonu; bulunamazsa hedef profil yolu."""
    for candidate in _default_profile_candidates():
        try:
            if candidate.is_file():
                return candidate
        except OSError:
            continue
    return _default_profile_candidates()[0]


#: Tek kanonik şablon; `core.load_profile` eksik profili bundan üretir.
DEFAULT_PROFILE_PATH = default_profile_path()


def normalize_restart_key(value):
    """Yeniden başlatma tuşunu doğrular; geçersizse güvenli varsayılana döner."""
    if isinstance(value, str):
        candidate = value.strip().upper()
        if is_valid_key_name(candidate, allow_button=False):
            return candidate
    return DEFAULT_RESTART_KEY


def restart_watch_command(engine_bin, key):
    """STOP sonrası tuş dinleyicisinin komut satırı."""
    return [str(engine_bin), "--watch-restart", normalize_restart_key(key)]


# -- Özel dosya I/O (atomik yazma, yeniden ayrıştırma noktası reddi) -------

def write_private_bytes(path, data):
    """Dosyayı atomik yazar; POSIX'te 0600, Windows'ta kullanıcı ACL'i."""
    path = Path(path)
    if winplatform.file_has_reparse_point(path):
        raise OSError(T("Sembolik bağlantı reddedildi: {}").format(path))
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(dir=str(path.parent), prefix=".tmp-")
    try:
        os.chmod(tmp_name, 0o600)
    except OSError:
        pass
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp_name, path)
    finally:
        if os.path.exists(tmp_name):
            try:
                os.unlink(tmp_name)
            except OSError:
                pass
    return path


def read_private_bytes(path, max_bytes=MAX_PROFILE_BYTES):
    """Dosyayı sınırlı boyutta okur; sembolik bağlantıyı reddeder.

    Windows'ta `O_NOFOLLOW` bulunmadığından önce yeniden ayrıştırma noktası
    denetlenir; ardından normal dosya ve boyut sınırı doğrulanır.
    """
    path = Path(path)
    if winplatform.file_has_reparse_point(path):
        raise OSError(T("Sembolik bağlantı reddedildi: {}").format(path))
    try:
        info = os.stat(path)
    except OSError as exc:
        raise OSError(str(exc)) from exc
    if not os.path.isfile(path):
        raise OSError(T("Düzenli dosya değil: {}").format(path))
    if info.st_size > max_bytes:
        raise OSError(T("Dosya çok büyük: {} bayt > {}").format(
            info.st_size, max_bytes))
    with open(path, "rb") as handle:
        data = handle.read(max_bytes + 1)
    # `stat` ile `read` arasında dosya büyüyebilir; sınırı okunan baytlarda
    # da uygula, böylece çağıranlar aşırı büyük veri görmez.
    if len(data) > max_bytes:
        raise OSError(T("Dosya çok büyük: {} bayt > {}").format(
            len(data), max_bytes))
    return data


def read_private_text_file(path, max_bytes=MAX_PROFILE_BYTES):
    """UTF-8 metin okur; bozuk kodlamada ``None`` döner."""
    try:
        data = read_private_bytes(path, max_bytes=max_bytes)
    except OSError:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def atomic_write_private(path, data):
    """Bayt verisini atomik yazar (``write_private_bytes`` takma adı)."""
    return write_private_bytes(path, data)


def backup_private_file(source, destination, max_bytes=MAX_PROFILE_BYTES):
    """Dosyayı ``destination`` konumuna atomik ve özel izinlerle kopyalar.

    Kaynak okunamazsa ``OSError`` yükselir; çağıran yedek hatasını profilin
    kendisini bozmadan yutabilir.
    """
    data = read_private_bytes(source, max_bytes=max_bytes)
    write_private_bytes(destination, data)
    return destination


def open_rotated_log(path, max_bytes=LOG_MAX_BYTES):
    """Günlüğü döndürür: eşik aşılmışsa ``.1`` kopyasına taşır."""
    path = Path(path)
    try:
        if path.is_file() and path.stat().st_size > max_bytes:
            target = path.with_name(path.name + ".1")
            os.replace(path, target)
    except OSError:
        pass
    return path


# -- Çalışan motorla iletişim ---------------------------------------------

def _hidden_run(args, timeout):
    """Sabit Win32 bayraklı, gizli pencereli alt süreç çalıştırır."""
    creationflags = 0
    if winplatform.is_windows():
        creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    return subprocess.run(
        args, capture_output=True, text=True, timeout=timeout,
        creationflags=creationflags)


def read_runtime_info():
    """Doğrulanmış `runtime.json` içeriği; yok/bozuksa ``None``.

    Alanlar motorun `RuntimeInfo` yapısıyla aynı camelCase adları taşır.
    """
    raw = read_private_text_file(RUNTIME_FILE, max_bytes=RUNTIME_MAX_BYTES)
    if not raw:
        return None
    try:
        data = json.loads(raw)
    except (ValueError, TypeError):
        return None
    if not isinstance(data, dict):
        return None
    pid = data.get("pid")
    creation = data.get("creationTime")
    pipe_name = data.get("pipeName")
    if not isinstance(pid, int) or isinstance(pid, bool) or pid <= 0:
        return None
    if not isinstance(creation, int) or isinstance(creation, bool) or creation < 0:
        return None
    if not isinstance(pipe_name, str) or not pipe_name:
        return None
    for field in ("exePath", "sid", "logPath"):
        if not isinstance(data.get(field), str):
            return None
    return data


def engine_pid():
    """`runtime.json`'daki PID; dosya yoksa ``None``."""
    info = read_runtime_info()
    return info["pid"] if info else None


def engine_running():
    """Motor gerçekten yaşıyor mu? PID yeniden kullanımı ayıklanır."""
    info = read_runtime_info()
    if not info:
        return False
    pid = info["pid"]
    if not winplatform.process_alive(pid):
        return False
    created = winplatform.process_creation_time(pid)
    if created is not None and created != info["creationTime"]:
        return False
    image = winplatform.process_image_path(pid)
    if image and Path(image).name.lower() != DAEMON_NAME.lower():
        return False
    return True


def ipc_command(command, timeout=5.0):
    """Motora kanal komutu gönderir; ``(ok, payload)`` döndürür.

    ``payload`` başarıda motorun JSON yanıtı (sözlük), hatada kısa metindir.
    Yanıt satırı JSON olduğundan en son boş olmayan satır ayrıştırılır.
    """
    engine = find_engine()
    if engine is None:
        return False, T("Motor ikilisi bulunamadı (macrostat-engine.exe).")
    try:
        result = _hidden_run([str(engine), "--ipc", command], timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return False, str(exc)
    stdout = (result.stdout or "").strip()
    if result.returncode != 0:
        detail = (result.stderr or "").strip() or stdout
        return False, detail or T("Kanal hatası (çıkış kodu {}).").format(
            result.returncode)
    line = ""
    for candidate in stdout.splitlines():
        if candidate.strip():
            line = candidate.strip()
    try:
        payload = json.loads(line)
    except (ValueError, TypeError):
        return False, T("Kanal yanıtı ayrıştırılamadı.")
    if not isinstance(payload, dict):
        return False, T("Kanal yanıtı ayrıştırılamadı.")
    if not payload.get("ok"):
        return False, str(payload.get("error") or T("Bilinmeyen kanal hatası."))
    return True, payload


def stop_engine(timeout=3.0):
    """Motoru zarifçe durdurur; yanıt yoksa kimliği doğrulanmış süreci sonlandırır.

    Windows'ta `TerminateProcess` tuş bırakma temizliği yapamaz; bu yüzden
    önce kanal `stop` denenir, yalnızca yanıtsız kalırsa zorla sonlandırılır.
    """
    import time

    info = read_runtime_info()
    if not info:
        return True
    ipc_command("stop", timeout=min(timeout, 5.0))
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not engine_running():
            return True
        time.sleep(0.1)
    if winplatform.process_alive(info["pid"]):
        # Zorla sonlandırmadan önce kimliği yeniden doğrula: süreç bu arada
        # çıkıp PID yeniden kullanılmışsa başka bir süreci öldürmeyelim.
        created = winplatform.process_creation_time(info["pid"])
        if created is not None and created != info["creationTime"]:
            return True
        image = winplatform.process_image_path(info["pid"])
        if image and Path(image).name.lower() != DAEMON_NAME.lower():
            return True
        return winplatform.terminate_process(info["pid"])
    return True


def cleanup_daemon():
    """Uygulama kapanırken motoru durdurur (sessiz; hata yutulur)."""
    global DAEMON_PROC
    try:
        if engine_running():
            stop_engine(timeout=2.0)
    except Exception:
        pass
    DAEMON_PROC = None


def _seed_profile_bytes():
    """Şablonu aktif dile göre üretir (yalnızca görünen ad/açıklama çevrilir)."""
    source = default_profile_path()
    try:
        raw = source.read_text(encoding="utf-8")
    except OSError as exc:
        raise OSError(str(exc)) from exc
    if get_language() != "en":
        return raw.encode("utf-8")
    try:
        data = json.loads(raw)
        for profile in data.get("profiles", []):
            for field in ("name", "description"):
                if isinstance(profile.get(field), str):
                    profile[field] = T(profile[field])
            for macro in profile.get("macros", []):
                for field in ("name", "description"):
                    if isinstance(macro.get(field), str):
                        macro[field] = T(macro[field])
        return json.dumps(data, indent=2, ensure_ascii=False).encode("utf-8")
    except Exception:
        return raw.encode("utf-8")


def locate_paths():
    """Motor ikilisini bulur ve kullanıcı profilini ilk açılışta tohumlar."""
    engine_bin = find_engine()
    try:
        ensure_dirs()
    except OSError as exc:
        print(f"[UYARI] Veri klasörü oluşturulamadı: {exc}")
    if not PROFILE_PATH.exists():
        try:
            write_private_bytes(PROFILE_PATH, _seed_profile_bytes())
        except OSError as exc:
            print(f"[UYARI] Profil tohumlanamadı: {exc}")
    return engine_bin, PROFILE_PATH


def safe_join_name(path):
    """Paylaşılan kilit/geçici dosya adı için yol bileşenini sadeleştirir."""
    return Path(path).name.replace(" ", "_")


# Tohumlama dile duyarlı olsun: arayüz `main()` çalışmadan önce de ayarı uygula.
try:
    set_language(load_settings().get("language", get_language()))
except Exception:
    pass

DAEMON_BIN, PROFILE_PATH = locate_paths()

try:
    from PyQt6.QtWidgets import (
        QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
        QTableWidget, QTableWidgetItem, QPushButton, QLabel, QLineEdit,
        QComboBox, QSpinBox, QDoubleSpinBox, QHeaderView, QMessageBox,
        QTabWidget, QGroupBox, QFormLayout, QTextEdit, QDialog,
        QAbstractItemView, QFrame, QInputDialog, QFileDialog, QToolButton,
        QMenu, QCheckBox, QListWidget, QListWidgetItem, QScrollArea
    )
    from PyQt6.QtCore import (
        Qt, QTimer, QProcess, pyqtSignal, QRectF, QPropertyAnimation,
        pyqtProperty, QEvent,
    )
    from PyQt6.QtGui import QFont, QColor, QPainter, QBrush, QPen, QIcon
except ImportError:
    print(T("[HATA] PyQt6 kütüphanesi eksik!"))
    print(T("Kurulum için: pip install -r gui/requirements-win.txt"))
    sys.exit(1)
