# -*- mode: python ; coding: utf-8 -*-
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""PyInstaller (onedir) tanımı — MacroStat arayüzü + motor ikilisi.

Üretilen düzen::

    dist/MacroStat/MacroStat.exe
    dist/MacroStat/_internal/          (Python, Qt, assets/)
    dist/MacroStat/_internal/macrostat-engine.exe

`common.find_engine()` adayları arasında `_internal/macrostat-engine.exe`
vardır; sembolik bağlantı/junction reddi korunur.

Derleme: ``pyinstaller --clean --noconfirm windows/gui/MacroStat.spec``
(depo kökünden). Motor ikilisi önceden ``cargo build --release --locked``
ile üretilmiş olmalıdır; eksikse spec **bilinçli olarak** başarısız olur.
"""

from pathlib import Path

GUI_DIR = Path(SPECPATH).resolve()
WINDOWS_DIR = GUI_DIR.parent
ENGINE_NAME = "macrostat-engine.exe"

_engine_candidates = [
    WINDOWS_DIR / "engine" / "target" / "release" / ENGINE_NAME,
    WINDOWS_DIR / "engine" / "target" / "x86_64-pc-windows-msvc"
    / "release" / ENGINE_NAME,
]
_engine = next((path for path in _engine_candidates if path.is_file()), None)
if _engine is None:
    raise SystemExit(
        "macrostat-engine.exe bulunamadı; önce windows/engine içinde "
        "'cargo build --release --locked' çalıştırın. Denenen yollar: "
        + ", ".join(str(path) for path in _engine_candidates))

_icon = GUI_DIR / "assets" / "macrostat.ico"
_version_file = GUI_DIR / "version_info.txt"

# Hedefler `_internal/` altına göre verilir (PyInstaller 6 onedir).
_datas = [
    (str(GUI_DIR / "assets" / "default_profile.json"), "assets"),
    (str(GUI_DIR / "assets" / "macrostat.svg"), "assets"),
    (str(GUI_DIR / "assets" / "macrostat.ico"), "assets"),
]

a = Analysis(
    [str(GUI_DIR / "main.py")],
    pathex=[str(GUI_DIR)],
    binaries=[(str(_engine), ".")],
    datas=_datas,
    hiddenimports=[],
    hookspath=[],
    runtime_hooks=[],
    excludes=[
        "tkinter",
        "unittest",
        "pydoc_data",
        "PyQt6.QtQml",
        "PyQt6.QtQuick",
        "PyQt6.QtQuickWidgets",
        "PyQt6.QtWebEngineCore",
        "PyQt6.QtWebEngineWidgets",
        "PyQt6.Qt3DCore",
        "PyQt6.QtMultimedia",
        "PyQt6.QtBluetooth",
        "PyQt6.QtNfc",
        "PyQt6.QtPositioning",
        "PyQt6.QtSerialPort",
    ],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="MacroStat",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    disable_windowed_traceback=False,
    icon=str(_icon) if _icon.is_file() else None,
    version=str(_version_file) if _version_file.is_file() else None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="MacroStat",
)
