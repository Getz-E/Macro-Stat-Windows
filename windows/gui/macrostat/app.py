# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""MacroStat Windows uygulama sınıfı ve giriş noktası.

Linux sürümündeki `fcntl.flock` tek örnek kilidinin karşılığı `QLockFile`'dır:
kilit `%LOCALAPPDATA%\\MacroStat` altında durur, her kullanıcı kendi
kopyasını çalıştırabilir ve ölü süreç kilitleri Qt tarafından temizlenir.
"""

import os
import sys
from pathlib import Path

from PyQt6.QtWidgets import (
    QAbstractSpinBox, QApplication, QLineEdit, QPlainTextEdit, QTextEdit)

from . import common, theme, winplatform
from .common import *  # noqa: F401,F403
from .core import CoreMixin
from .macros import MacrosMixin
from .editor import EditorMixin
from .recorder import RecorderMixin
from .system import SystemMixin


class MacroStatApp(CoreMixin, MacrosMixin, EditorMixin, RecorderMixin,
                   SystemMixin, QMainWindow):
    """Sekmeli MacroStat arayüzü; davranış mixin modüllerinde ayrıştırılmıştır."""

    @staticmethod
    def _focus_is_text_entry():
        """Odak bir metin alanındaysa Delete normal metin düzenlemesidir."""
        focused = QApplication.focusWidget()
        return isinstance(
            focused, (QLineEdit, QTextEdit, QPlainTextEdit, QAbstractSpinBox))

    def keyPressEvent(self, event):
        # Delete yalnızca tablo odaklıyken değil, makro sekmesi açıkken ve
        # gerçek bir satır seçiliyken çalışır. Motorun AÇIK/KAPALI olması
        # (ör. STOP sonrası odak başka bir düğmede kalsa bile) bu GUI
        # işlemini etkilemez; metin alanlarında Delete düzenleme olarak kalır.
        # `currentRow()` tek başına yetmez: `clearSelection()` sonrası Qt
        # geçerli satırı koruyabilir, bu yüzden seçim modeli de denetlenir.
        selection = self.macro_table.selectionModel()
        if (event.key() == Qt.Key.Key_Delete
                and not self.recording_active
                and self.tabs.currentWidget() is self.tab_macros
                and selection is not None
                and selection.hasSelection()
                and self.macro_table.currentRow() >= 0
                and not self._focus_is_text_entry()):
            # Odak satır içi bir düğmedeyse silme sırasında o düğme yok
            # edilmeden önce odağı tabloya taşı.
            self.macro_table.setFocus(Qt.FocusReason.OtherFocusReason)
            self.delete_selected_macro()
            return
        super().keyPressEvent(event)


#: Kilit dosyası adı; veri dizini kullanıcıya özel olduğundan ek SID gerekmez.
LOCK_NAME = "macrostat-gui.lock"

_GUI_LOCK = None


def _lock_path():
    return Path(common.RUNTIME_DIR) / LOCK_NAME


def acquire_gui_single_instance():
    """İkinci bir arayüz örneğinin aynı profili/motoru yönetmesini engeller.

    Kilit açılamazsa (salt-okunur/kısıtlı dizin) uygulama engellenmez; bu,
    Linux sürümündeki "kilitsiz de çalış" davranışıyla aynıdır.
    """
    global _GUI_LOCK
    from PyQt6.QtCore import QLockFile

    try:
        _lock_path().parent.mkdir(parents=True, exist_ok=True)
        if winplatform.file_has_reparse_point(_lock_path().parent):
            return True
        lock = QLockFile(str(_lock_path()))
        # Ölü süreç kilidi Qt tarafından algılanır; eski kilit 30 sn sonra
        # temizlenebilir olsa da canlı bir örnek kilidi bırakmaz.
        lock.setStaleLockTime(30_000)
        if not lock.tryLock(100):
            return False
        _GUI_LOCK = lock
        return True
    except Exception:
        return True


def release_gui_single_instance():
    """Kilit dosyasını bırakır; yeniden başlatmada yeni örneğin açılmasını sağlar."""
    global _GUI_LOCK
    if _GUI_LOCK is not None:
        try:
            _GUI_LOCK.unlock()
        except Exception:
            pass
        _GUI_LOCK = None


def restart_arguments():
    """Arayüzü yeniden başlatmak için ``(program, args)``.

    Paketlenmiş uygulamada (PyInstaller) EXE doğrudan çalıştırılır; geliştirme
    düzeninde ``gui/main.py`` Python yorumlayıcısıyla açılır.
    """
    if getattr(sys, "frozen", False):
        return sys.executable, []
    entry = Path(common.SCRIPT_DIR) / "main.py"
    return sys.executable, [str(entry)]


def main():
    # DPI farkındalığı QApplication'dan ÖNCE sabitlenmeli.
    winplatform.set_dpi_awareness()

    user_settings = common.load_settings()
    common.set_language(user_settings.get("language", "tr"))
    app = QApplication(sys.argv)
    app.setApplicationName("MacroStat")
    app.setApplicationDisplayName("MacroStat")
    app.setOrganizationName("MacroStat")
    theme.apply_theme(app, user_settings.get("theme", "dark"))
    if not acquire_gui_single_instance():
        QMessageBox.warning(
            None, "MacroStat",
            T("MacroStat zaten çalışıyor. Mevcut pencereyi kullanın."))
        sys.exit(0)
    app.aboutToQuit.connect(common.cleanup_daemon)
    window = MacroStatApp()
    window.show()
    if theme.resolve_mode(app, theme.current_mode()) == "dark":
        winplatform.set_dark_title_bar(int(window.winId()), True)
    exit_code = app.exec()
    release_gui_single_instance()
    common.cleanup_daemon()
    sys.exit(exit_code)
