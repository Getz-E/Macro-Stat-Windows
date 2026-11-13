# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Windows platform katmanı: süreç kimliği, Job Object, yetki ve imleç.

Linux sürümündeki `/proc`, `fcntl` ve `signal` kullanımının karşılığıdır.
Bu modül Linux'ta da içe aktarılabilir (testler için); Windows'a özgü
çağrılar `IS_WINDOWS` denetimiyle korunur ve desteklenmeyen ortamda
`None`/`False` döner. Hiçbir fonksiyon kullanıcı girdisiyle komut satırı
çalıştırmaz; yalnızca sabit Win32 çağrıları kullanılır.
"""

import ctypes
import ctypes.wintypes
import os
import sys
from pathlib import Path

IS_WINDOWS = sys.platform == "win32"

#: Aynı dosya sistemi girdisini işaret eden yeniden ayrıştırma noktası
#: (symlink, junction, mount point) özniteliği.
FILE_ATTRIBUTE_REPARSE_POINT = 0x400

_PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
_PROCESS_TERMINATE = 0x0001
_STILL_ACTIVE = 259
_TOKEN_QUERY = 0x0008
_TOKEN_ELEVATION_CLASS = 20
_MAX_PATH_CHARS = 32768


if IS_WINDOWS:  # pragma: no cover - Windows'a özgü
    _kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    _user32 = ctypes.WinDLL("user32", use_last_error=True)

    # HANDLE 64 bit'tir; varsayılan c_int dönüş tipi taşmaya yol açar.
    _kernel32.OpenProcess.restype = ctypes.c_void_p
    _kernel32.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int,
                                      ctypes.c_uint32]
    _kernel32.GetCurrentProcess.restype = ctypes.c_void_p
    _kernel32.CreateJobObjectW.restype = ctypes.c_void_p
    _kernel32.CreateJobObjectW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p]
    _kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
    _kernel32.TerminateProcess.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
    _kernel32.AssignProcessToJobObject.argtypes = [ctypes.c_void_p,
                                                   ctypes.c_void_p]
    _advapi32.OpenProcessToken.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
                                           ctypes.POINTER(ctypes.c_void_p)]


class _FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", ctypes.c_uint32),
                ("dwHighDateTime", ctypes.c_uint32)]


class _KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", ctypes.c_uint16),
                ("wScan", ctypes.c_uint16),
                ("dwFlags", ctypes.c_uint32),
                ("time", ctypes.c_uint32),
                ("dwExtraInfo", ctypes.c_void_p)]


class _MOUSEINPUT(ctypes.Structure):
    _fields_ = [("dx", ctypes.c_int32),
                ("dy", ctypes.c_int32),
                ("mouseData", ctypes.c_uint32),
                ("dwFlags", ctypes.c_uint32),
                ("time", ctypes.c_uint32),
                ("dwExtraInfo", ctypes.c_void_p)]


class _HARDWAREINPUT(ctypes.Structure):
    _fields_ = [("uMsg", ctypes.c_uint32),
                ("wParamL", ctypes.c_uint16),
                ("wParamH", ctypes.c_uint16)]


class _INPUTUNION(ctypes.Union):
    _fields_ = [("ki", _KEYBDINPUT),
                ("mi", _MOUSEINPUT),
                ("hi", _HARDWAREINPUT)]


class _INPUT(ctypes.Structure):
    _fields_ = [("type", ctypes.c_uint32),
                ("u", _INPUTUNION)]


_INPUT_KEYBOARD = 1
_VK_PAUSE = 0x13
_SCAN_PAUSE = 0x45

if IS_WINDOWS:  # pragma: no cover - Windows'a özgü
    _user32.SendInput.restype = ctypes.c_uint
    _user32.SendInput.argtypes = [ctypes.c_uint, ctypes.POINTER(_INPUT),
                                  ctypes.c_int]


def is_windows():
    """Çalışılan ortam Windows mu?"""
    return IS_WINDOWS


def file_has_reparse_point(path):
    """Dosya/dizin bir yeniden ayrıştırma noktası mı (symlink/junction)?

    Windows'ta `os.open(..., O_NOFOLLOW)` yoktur; profil okuma/yazma yolu
    bu denetimle sembolik bağlantı üzerinden başka bir hedefe
    yönlendirilmeyi reddeder. Denetim OpenProcess/`lstat` benzeri sabit bir
    çağrıdır, kullanıcı girdisi yürütmez.
    """
    try:
        info = os.lstat(path)
    except OSError:
        return False
    if getattr(os.path, "islink", None) and os.path.islink(path):
        return True
    attributes = getattr(info, "st_file_attributes", 0)
    return bool(attributes & FILE_ATTRIBUTE_REPARSE_POINT)


def _open_process(pid, access):
    if not IS_WINDOWS or not isinstance(pid, int) or pid <= 0:
        return None
    handle = _kernel32.OpenProcess(access, False, pid)
    return handle or None


def _close_handle(handle):
    if handle:
        try:
            _kernel32.CloseHandle(handle)
        except Exception:
            pass


def process_image_path(pid):
    """Sürecin tam çalıştırılabilir yolu; erişilemezse ``None``."""
    if not IS_WINDOWS:
        return _posix_image_path(pid)
    handle = _open_process(pid, _PROCESS_QUERY_LIMITED_INFORMATION)
    if not handle:
        return None
    try:
        buffer = ctypes.create_unicode_buffer(_MAX_PATH_CHARS)
        size = ctypes.c_uint32(len(buffer))
        ok = _kernel32.QueryFullProcessImageNameW(
            handle, 0, buffer, ctypes.byref(size))
        return buffer.value if ok else None
    except Exception:
        return None
    finally:
        _close_handle(handle)


def process_creation_time(pid):
    """Sürecin oluşturulma zamanı (100 ns birim, FILETIME).

    PID yeniden kullanımını ayırt etmek için kullanılır; Linux'taki
    `/proc/<pid>/stat` ``starttime`` alanının karşılığıdır.
    """
    if not IS_WINDOWS:
        return None
    handle = _open_process(pid, _PROCESS_QUERY_LIMITED_INFORMATION)
    if not handle:
        return None
    try:
        created = _FILETIME()
        exited = _FILETIME()
        kernel = _FILETIME()
        user = _FILETIME()
        ok = _kernel32.GetProcessTimes(
            handle, ctypes.byref(created), ctypes.byref(exited),
            ctypes.byref(kernel), ctypes.byref(user))
        if not ok:
            return None
        return (created.dwHighDateTime << 32) | created.dwLowDateTime
    except Exception:
        return None
    finally:
        _close_handle(handle)


def process_alive(pid):
    """Süreç yaşıyor mu? (zombi kavramı Windows'ta yoktur)"""
    if not IS_WINDOWS:
        return _posix_alive(pid)
    handle = _open_process(pid, _PROCESS_QUERY_LIMITED_INFORMATION)
    if not handle:
        return False
    try:
        code = ctypes.c_uint32(0)
        ok = _kernel32.GetExitCodeProcess(handle, ctypes.byref(code))
        return bool(ok) and code.value == _STILL_ACTIVE
    except Exception:
        return False
    finally:
        _close_handle(handle)


def terminate_process(pid):
    """Kimliği doğrulanmış süreci zorla sonlandırır (son çare).

    Yalnızca çağıran `process_creation_time` ile PID yeniden kullanımını
    ayıkladıktan sonra çağırmalıdır. Windows'ta `TerminateProcess`; tuş
    bırakma temizliği yapılamaz, bu yüzden IPC `stop` tercih edilir.
    """
    if not IS_WINDOWS:
        import signal
        try:
            os.kill(pid, signal.SIGKILL)
            return True
        except OSError:
            return False
    handle = _open_process(pid, _PROCESS_TERMINATE)
    if not handle:
        return False
    try:
        return bool(_kernel32.TerminateProcess(handle, 1))
    except Exception:
        return False
    finally:
        _close_handle(handle)


def is_elevated():
    """Süreç yönetici (yükseltilmiş) bütünlük düzeyinde mi?"""
    if not IS_WINDOWS:
        try:
            return os.geteuid() == 0
        except AttributeError:
            return False
    try:
        token = ctypes.c_void_p()
        if not _advapi32.OpenProcessToken(
                _kernel32.GetCurrentProcess(), _TOKEN_QUERY,
                ctypes.byref(token)):
            return False
        try:
            elevation = ctypes.c_uint32(0)
            returned = ctypes.c_uint32(0)
            ok = _advapi32.GetTokenInformation(
                token, _TOKEN_ELEVATION_CLASS, ctypes.byref(elevation),
                ctypes.sizeof(elevation), ctypes.byref(returned))
            return bool(ok) and bool(elevation.value)
        finally:
            _close_handle(token)
    except Exception:
        return False


def current_user_sid_string():
    """Geçerli kullanıcının SID metni (``S-1-5-…``); alınamazsa ``None``."""
    if not IS_WINDOWS:
        return None
    try:
        import winreg
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Volatile Environment"):
            user = winreg.QueryValueEx(winreg, "USERNAME")[0]
            domain = winreg.QueryValueEx(winreg, "USERDOMAIN")[0]
        return f"{domain}\\{user}"
    except Exception:
        return None


def cursor_position():
    """Fiziksel ekran imleç konumu ``(x, y)``; alınamazsa ``None``.

    Kayıt başlangıç noktası olarak kullanılır; çoklu monitörde negatif
    koordinatlar geçerlidir.
    """
    if not IS_WINDOWS:
        return None
    try:
        point = ctypes.wintypes.POINT()
        if _user32.GetCursorPos(ctypes.byref(point)):  # noqa: E501 - sabit API
            return (int(point.x), int(point.y))
    except Exception:
        return None
    return None


def send_pause_key():
    """Sentinel PAUSE tuşunu sisteme enjekte eder (kaydı bitirmek için).

    Kayıtçı süreci yalnızca panik tuşuyla temiz biçimde kapanıp dosyayı
    yazar; `terminate()` yarıda kesilmiş veri bırakır. Enjeksiyon kendi
    imzamızı (`dwExtraInfo`) taşımaz; kayıtçı kancası bunu gerçek tuş gibi
    görür. Başarıyı döndürür.
    """
    if not IS_WINDOWS:
        return False
    try:
        inputs = (_INPUT * 2)()
        for index, flags in ((0, 0), (1, 0x0002)):  # KEYEVENTF_KEYUP = 0x0002
            inputs[index].type = _INPUT_KEYBOARD
            inputs[index].u.ki = _KEYBDINPUT(
                _VK_PAUSE, _SCAN_PAUSE, flags, 0, None)
        sent = _user32.SendInput(2, inputs, ctypes.sizeof(_INPUT))
        return sent == 2
    except Exception:
        return False


def set_dark_title_bar(window_handle, dark=True):
    """Pencere başlık çubuğunu koyu/açık moda çeker (Windows 10 1809+).

    ``DWMWA_USE_IMMERSIVE_DARK_MODE`` (20) desteklenmezse sessizce başarısız
    olur; başlık çubuğu yalnızca görsel tercih olduğundan ölümcül değildir.
    """
    if not IS_WINDOWS or not window_handle:
        return False
    try:
        dwmapi = ctypes.WinDLL("dwmapi", use_last_error=True)
        value = ctypes.c_int(1 if dark else 0)
        result = dwmapi.DwmSetWindowAttribute(
            ctypes.c_void_p(int(window_handle)), 20,
            ctypes.byref(value), ctypes.sizeof(value))
        return result == 0
    except Exception:
        return False


def set_dpi_awareness():
    """Qt açılmadan önce Per-Monitor V2 DPI farkındalığını sabitler.

    Qt bunu genellikle kendisi yapar; açık çağrı, farklı Qt sürümlerinde
    %150/%200 ölçekte tutarlı davranış sağlar. Başarısızlık ölümcül değildir.
    """
    if not IS_WINDOWS:
        return False
    try:
        context = ctypes.c_void_p(-4)  # DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
        return bool(_user32.SetProcessDpiAwarenessContext(context))
    except Exception:
        return False


class _BASIC_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_int64),
        ("PerJobUserTimeLimit", ctypes.c_int64),
        ("LimitFlags", ctypes.c_uint32),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", ctypes.c_uint32),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", ctypes.c_uint32),
        ("SchedulingClass", ctypes.c_uint32),
    ]


class _IO_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_uint64),
        ("WriteOperationCount", ctypes.c_uint64),
        ("OtherOperationCount", ctypes.c_uint64),
        ("ReadTransferCount", ctypes.c_uint64),
        ("WriteTransferCount", ctypes.c_uint64),
        ("OtherTransferCount", ctypes.c_uint64),
    ]


class _EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _BASIC_LIMIT_INFORMATION),
        ("IoInfo", _IO_COUNTERS),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class JobObject:
    """Motoru GUI ömrüne bağlayan Job Object (``KILL_ON_JOB_CLOSE``).

    GUI çökerse çekirdek motoru zorla sonlandırır. Bu, motorun
    ``--parent-pid`` yolunun sert yedeğidir: süreç ölmeden önce tuş
    bırakma temizliği garanti edilemez; bkz. `docs/TEST-MATRIX.md` L-02.
    """

    _JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x2000
    _JOB_OBJECT_LIMIT_BREAKAWAY_OK = 0x0800
    _JOBOBJECT_EXTENDED_LIMIT_INFORMATION_CLASS = 9
    _PROCESS_TERMINATE = 0x0001
    _PROCESS_SET_QUOTA = 0x0100
    _PROCESS_QUERY_LIMITED_INFORMATION = 0x1000

    def __init__(self):
        self._handle = None
        if not IS_WINDOWS:
            return
        try:
            handle = _kernel32.CreateJobObjectW(None, None)
            if not handle:
                return
            info = _EXTENDED_LIMIT_INFORMATION()
            info.BasicLimitInformation.LimitFlags = (
                self._JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | self._JOB_OBJECT_LIMIT_BREAKAWAY_OK)
            ok = _kernel32.SetInformationJobObject(
                handle, self._JOBOBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                ctypes.byref(info), ctypes.sizeof(info))
            if not ok:
                _close_handle(handle)
                return
            self._handle = handle
        except Exception:
            self._handle = None

    @property
    def available(self):
        return self._handle is not None

    def assign(self, pid):
        """Süreci iş nesnesine atar; başarılıysa ``True``."""
        if not self._handle or not IS_WINDOWS:
            return False
        process = _open_process(pid, self._PROCESS_SET_QUOTA
                                | self._PROCESS_TERMINATE
                                | self._PROCESS_QUERY_LIMITED_INFORMATION)
        if not process:
            return False
        try:
            return bool(_kernel32.AssignProcessToJobObject(self._handle, process))
        except Exception:
            return False
        finally:
            _close_handle(process)

    def close(self):
        if self._handle:
            _close_handle(self._handle)
            self._handle = None


# -- POSIX yedekleri (yalnızca Linux'ta test/geliştirme için) --------------

def _posix_image_path(pid):
    try:
        return os.readlink(f"/proc/{pid}/exe")
    except (OSError, AttributeError):
        return None


def _posix_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except (OSError, AttributeError):
        return False


def safe_join_name(path):
    """Paylaşılan kilit/geçici dosya adı için yol bileşenini sadeleştirir."""
    return Path(path).name.replace(" ", "_")
