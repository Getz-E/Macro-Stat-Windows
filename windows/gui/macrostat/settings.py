# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Kullanıcı ayarları (dil, tema) için kalıcı, atomik depolama.

Windows yerleşimi:

* ``%APPDATA%\\MacroStat\\settings.json`` — gezinme ayarları (dil, tema,
  onboarding, günlük paneli görünürlüğü).
* ``%LOCALAPPDATA%\\MacroStat`` — profil, motor günlüğü ve `runtime.json`
  (makinede yerel veri; dolaşımda kopyalanmaz).

Ortam değişkenleri yoksa (geliştirme/Linux testleri) ``~/.macrostat/...``
kullanılır. Dosya atomik yazılır: geçici dosya + `fsync` + `os.replace`.
"""

import json
import os
import tempfile
from pathlib import Path

from . import winplatform


def _base_dir(env_name, fallback_parts):
    value = os.environ.get(env_name)
    if value and os.path.isabs(value):
        return Path(value) / "MacroStat"
    return Path.home().joinpath(*fallback_parts)


CONFIG_DIR = _base_dir("APPDATA", (".macrostat", "config"))
DATA_DIR = _base_dir("LOCALAPPDATA", (".macrostat", "data"))
SETTINGS_PATH = CONFIG_DIR / "settings.json"

THEME_MODES = ("system", "dark", "light")

#: Ayar dosyası küçüktür; üst sınır bellek tüketimini sınırlar.
MAX_SETTINGS_BYTES = 64 * 1024


def _detect_language():
    """Windows görüntüleme dilinden TR/EN seçer.

    Qt henüz kurulmamış olabilir; bu yüzden Win32 çağrısı tercih edilir,
    başarısız olursa ortam değişkenlerine düşülür.
    """
    try:
        import ctypes
        lang_id = ctypes.windll.kernel32.GetUserDefaultUILanguage()
        # Birincil dil kimliği alt 10 bit; Türkçe 0x1F.
        if (lang_id & 0x3FF) == 0x1F:
            return "tr"
        return "en"
    except Exception:
        pass
    value = (os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES")
             or os.environ.get("LANG") or "").lower()
    return "tr" if value.startswith("tr") else "en"


def ensure_dirs():
    """Ayar ve veri dizinlerini oluşturur; hata ``OSError`` olarak yükselir."""
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    DATA_DIR.mkdir(parents=True, exist_ok=True)


def _read_settings_text():
    """Ayar dosyasını sınırlı boyutta UTF-8 okur.

    Yeniden ayrıştırma noktası (symlink/junction) reddedilir; dosya üst
    sınırı aşıyorsa ``None`` döner — ``json.load`` sınırsız okumaz.
    """
    if winplatform.file_has_reparse_point(SETTINGS_PATH):
        return None
    try:
        info = os.stat(SETTINGS_PATH)
    except OSError:
        return None
    if info.st_size > MAX_SETTINGS_BYTES:
        return None
    try:
        with open(SETTINGS_PATH, "rb") as handle:
            data = handle.read(MAX_SETTINGS_BYTES + 1)
    except OSError:
        return None
    if len(data) > MAX_SETTINGS_BYTES:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def load_settings():
    """Ayarları okur; eksik/bozuk dosyada güvenli varsayılan döndürür."""
    defaults = {
        "language": _detect_language(),
        "theme": "dark",
        "onboarding_completed": False,
        "log_panel_visible": False,
    }
    raw = _read_settings_text()
    if raw is not None:
        try:
            data = json.loads(raw)
        except (ValueError, TypeError):
            data = None
        if isinstance(data, dict):
            if data.get("language") not in ("tr", "en"):
                data["language"] = defaults["language"]
            if data.get("theme") not in THEME_MODES:
                data["theme"] = defaults["theme"]
            data.setdefault("onboarding_completed", False)
            data.setdefault("log_panel_visible", False)
            return data
    return defaults


def save_settings(settings):
    """Ayarları atomik olarak yazar; başarıyı ``bool`` olarak döndürür.

    Arayüz açılışta çalıştığından hata yükseltmez: yazma başarısızsa
    (kilitli/taşınmış dosya, yeniden ayrıştırma noktası) ``False`` döner ve
    varsayılanlar yalnızca bu oturumda geçerli olur.
    """
    if not isinstance(settings, dict):
        return False
    if winplatform.file_has_reparse_point(SETTINGS_PATH):
        return False
    tmp_name = None
    try:
        SETTINGS_PATH.parent.mkdir(parents=True, exist_ok=True)
        fd, tmp_name = tempfile.mkstemp(
            dir=str(SETTINGS_PATH.parent), prefix=".settings-")
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(settings, handle, indent=2, ensure_ascii=False)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp_name, SETTINGS_PATH)
        tmp_name = None
        return True
    except (OSError, TypeError, ValueError):
        return False
    finally:
        if tmp_name and os.path.exists(tmp_name):
            try:
                os.unlink(tmp_name)
            except OSError:
                pass
