# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Makro tablosu ve kayıt etkileşimi regresyon testleri.

İki bildirilen hatayı kapsar:

* Delete tuşu, odak makro tablosunda değilken (ör. motor STOP ile
  kapatıldıktan sonra odak STOP düğmesinde kalmışken) seçili makroyu
  silmiyordu. Silme motorun AÇIK/KAPALI olmasından bağımsız olmalıdır.
* Kayıt başladığında odak "Kaydı Durdur" düğmesinde kalıyordu; Qt Space
  tuşunu düğme etkinleştirmesi saydığı için Space kaydı durduruyordu.

Qt gerekir; ``QT_QPA_PLATFORM=offscreen`` ile Linux'ta da çalışır. Gerçek
motor süreci, kanal veya kullanıcı profili kullanılmaz.

Çalıştırma (depo kökünden):

    QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests \
        -p "test_gui*.py" -v
"""

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_TMP_ROOT = tempfile.mkdtemp(prefix="macrostat-gui-interaction-tests-")
os.environ.setdefault("APPDATA", _TMP_ROOT)
os.environ.setdefault("LOCALAPPDATA", _TMP_ROOT)
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

GUI_DIR = Path(__file__).resolve().parents[1] / "gui"
if str(GUI_DIR) not in sys.path:
    sys.path.insert(0, str(GUI_DIR))

from macrostat import common, recorder  # noqa: E402


def _qt():
    from PyQt6.QtCore import Qt
    from PyQt6.QtTest import QTest
    from PyQt6.QtWidgets import QApplication
    return Qt, QTest, QApplication


class _AppTestCase(unittest.TestCase):
    """Profil/settings yolları geçici dizine yönlendirilmiş gerçek pencere."""

    @classmethod
    def setUpClass(cls):
        _Qt, _QTest, QApplication = _qt()
        cls.qapp = QApplication.instance() or QApplication([])

    def _new_app(self, macro_count=3):
        from macrostat import core
        from macrostat.app import MacroStatApp

        tmp = tempfile.TemporaryDirectory(prefix="macrostat-app-")
        self.addCleanup(tmp.cleanup)
        base = Path(tmp.name)

        patchers = [
            mock.patch.object(common, "PROFILE_PATH", base / "profile.json"),
            mock.patch.object(common, "RUNTIME_FILE", base / "runtime.json"),
            mock.patch.object(core, "load_settings", return_value={}),
            mock.patch.object(MacroStatApp, "maybe_run_onboarding",
                              lambda self: None),
        ]
        for patcher in patchers:
            patcher.start()
            self.addCleanup(patcher.stop)

        app = MacroStatApp()
        self.addCleanup(self._dispose, app)
        # Testler diske yazmasın ve motorla konuşmasın.
        app.save_profile_to_disk = lambda **_kwargs: None
        app.show_autosave_banner = lambda *_args, **_kwargs: None
        macros = app.profile_data.setdefault("macros", [])
        while len(macros) < macro_count:
            index = len(macros)
            macros.append({
                "id": f"test-macro-{index}",
                "name": f"Test {index}",
                "enabled": True,
                "triggerKey": "KEY_X",
                "triggerMode": "ONCE",
                "actions": [{"type": "DELAY", "delayMs": 10}],
            })
        app.refresh_macro_table()
        app.show()
        return app

    @staticmethod
    def _dispose(app):
        try:
            app.status_timer.stop()
            app.log_panel.timer.stop()
            app._input_record_timer.stop()
            app.close()
            app.deleteLater()
            _Qt, _QTest, QApplication = _qt()
            QApplication.processEvents()
        except Exception:
            pass

    def _start_recording_ui(self, app):
        app.tabs.setCurrentWidget(app.tab_recorder)
        app.is_daemon_running = lambda: False
        app._start_input_recording = lambda: True
        app._start_recording()

    def _assert_macro_selected(self, app, row=0):
        app.tabs.setCurrentWidget(app.tab_macros)
        app.macro_table.selectRow(row)
        self.assertEqual(app.macro_table.currentRow(), row)


class DeleteMacroTests(_AppTestCase):
    """Delete, motor durumundan ve odaktan bağımsız GUI işlemi olmalı."""

    def test_delete_works_when_focus_is_not_the_table(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._assert_macro_selected(app)
        # STOP'a basıldıktan sonra odak bu düğmede kalır (motor KAPALI).
        app.btn_panic.setFocus()
        before = len(app.profile_data["macros"])
        QTest.keyClick(app.btn_panic, Qt.Key.Key_Delete)
        self.assertEqual(len(app.profile_data["macros"]), before - 1)

    def test_delete_works_with_engine_on_and_off(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        for running in (True, False):
            with self.subTest(engine_running=running):
                app.is_manually_stopped = not running
                with mock.patch.object(app, "is_daemon_running",
                                       return_value=running):
                    self._assert_macro_selected(app)
                    app.btn_panic.setFocus()
                    before = len(app.profile_data["macros"])
                    QTest.keyClick(app.btn_panic, Qt.Key.Key_Delete)
                    self.assertEqual(
                        len(app.profile_data["macros"]), before - 1)

    def test_delete_works_when_a_row_widget_has_focus(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._assert_macro_selected(app)
        cell = app.macro_table.cellWidget(0, 1)  # aç/kapat hücresi
        self.assertIsNotNone(cell)
        before = len(app.profile_data["macros"])
        QTest.keyClick(cell, Qt.Key.Key_Delete)
        self.assertEqual(len(app.profile_data["macros"]), before - 1)

    def test_delete_from_table_removes_exactly_one(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._assert_macro_selected(app)
        before = len(app.profile_data["macros"])
        QTest.keyClick(app.macro_table, Qt.Key.Key_Delete)
        self.assertEqual(len(app.profile_data["macros"]), before - 1)

    def test_delete_after_clear_selection_deletes_nothing(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._assert_macro_selected(app)
        # Qt, clearSelection() sonrası geçerli satırı koruyabilir; bu yüzden
        # setCurrentCell(-1, -1) çağrılmaz ve seçim modeli denetlenir.
        app.macro_table.clearSelection()
        self.assertFalse(
            app.macro_table.selectionModel().hasSelection())
        self.assertGreaterEqual(app.macro_table.currentRow(), 0)
        before = list(app.profile_data["macros"])
        app.btn_panic.setFocus()
        QTest.keyClick(app.btn_panic, Qt.Key.Key_Delete)
        self.assertEqual(app.profile_data["macros"], before)

    def test_delete_without_selection_deletes_nothing(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        app.tabs.setCurrentWidget(app.tab_macros)
        app.macro_table.clearSelection()
        app.macro_table.setCurrentCell(-1, -1)
        self.assertEqual(app.macro_table.currentRow(), -1)
        before = list(app.profile_data["macros"])
        QTest.keyClick(app.btn_panic, Qt.Key.Key_Delete)
        self.assertEqual(app.profile_data["macros"], before)

    def test_delete_on_other_tabs_is_ignored(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._assert_macro_selected(app)
        app.tabs.setCurrentWidget(app.tab_editor)
        before = list(app.profile_data["macros"])
        QTest.keyClick(app.btn_panic, Qt.Key.Key_Delete)
        self.assertEqual(app.profile_data["macros"], before)

    def test_text_entry_focus_is_detected(self):
        _Qt, _QTest, QApplication = _qt()
        from PyQt6.QtWidgets import QLineEdit

        app = self._new_app()
        edit = QLineEdit(app)
        with mock.patch.object(QApplication, "focusWidget",
                               return_value=edit):
            self.assertTrue(app._focus_is_text_entry())
        with mock.patch.object(QApplication, "focusWidget",
                               return_value=app.btn_panic):
            self.assertFalse(app._focus_is_text_entry())
        with mock.patch.object(QApplication, "focusWidget",
                               return_value=None):
            self.assertFalse(app._focus_is_text_entry())


class RecordingSpaceTests(_AppTestCase):
    """Space kayıt sırasında sıradan giriş olmalı, kaydı durdurmamalı."""

    def test_space_does_not_stop_recording(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._start_recording_ui(app)
        self.assertTrue(app.recording_active)
        # Odak artık "Kaydı Durdur" düğmesinde değil ve düğme klavye
        # odağı alamaz; Qt Space'i düğme tıklamasına çeviremez.
        self.assertFalse(app.btn_start_rec.hasFocus())
        self.assertEqual(app.btn_start_rec.focusPolicy(),
                         Qt.FocusPolicy.NoFocus)
        with mock.patch.object(app, "_stop_recording") as stop:
            QTest.keyClick(app.rec_table, Qt.Key.Key_Space)
            QTest.keyClick(app, Qt.Key.Key_Space)
        stop.assert_not_called()
        self.assertTrue(app.recording_active)

    def test_space_down_and_up_are_recorded(self):
        events = recorder.sanitize_key_events(
            {"keys": [[0, "KEY_SPACE", 1], [75, "KEY_SPACE", 0]]})
        self.assertEqual(
            events, [[0, "KEY_SPACE", 1], [75, "KEY_SPACE", 0]])
        merged = recorder.merge_input_events([], [], events)
        key_events = [event for event in merged if event["type"].startswith(
            "KEY_")]
        self.assertEqual(
            [event["type"] for event in key_events], ["KEY_DOWN", "KEY_UP"])
        self.assertEqual({event["key"] for event in key_events}, {"KEY_SPACE"})

    def test_stop_button_click_still_stops_and_restores_focus_policy(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._start_recording_ui(app)
        self.assertTrue(app.recording_active)
        QTest.mouseClick(app.btn_start_rec, Qt.MouseButton.LeftButton)
        self.assertFalse(app.recording_active)
        self.assertEqual(app.btn_start_rec.focusPolicy(),
                         Qt.FocusPolicy.StrongFocus)
        self.assertEqual(app.btn_start_rec.text(), common.T("Kaydı Başlat"))
        self.assertTrue(app.btn_start_rec.isEnabled())

    def test_finished_exception_restores_recording_ui(self):
        Qt, _QTest, _QApplication = _qt()
        app = self._new_app()
        self._start_recording_ui(app)
        tmp = tempfile.TemporaryDirectory(prefix="macrostat-record-")
        self.addCleanup(tmp.cleanup)
        record = Path(tmp.name) / "record.json"
        record.write_text(
            json.dumps({"keys": [[0, "KEY_SPACE", 1]]}), encoding="utf-8")
        app._input_record_file = str(record)
        proc = mock.Mock()
        proc.readAllStandardError.return_value = b""
        app._input_record_proc = proc
        app.btn_start_rec.setEnabled(False)  # _stop_recording'in bıraktığı durum
        with mock.patch.object(recorder, "merge_input_events",
                               side_effect=RuntimeError("boom")):
            with self.assertRaises(RuntimeError):
                app._input_record_finished(0, None)
        self.assertFalse(app.recording_active)
        self.assertTrue(app.btn_start_rec.isEnabled())
        self.assertEqual(app.btn_start_rec.focusPolicy(),
                         Qt.FocusPolicy.StrongFocus)
        self.assertEqual(app.btn_start_rec.text(), common.T("Kaydı Başlat"))
        self.assertTrue(app.chk_mouse_path.isEnabled())
        proc.deleteLater.assert_called_once()

    def test_process_failure_restores_recording_ui(self):
        Qt, _QTest, _QApplication = _qt()
        app = self._new_app()
        self._start_recording_ui(app)
        proc = mock.Mock()
        proc.errorString.return_value = "boom"
        app._input_record_proc = proc
        app.btn_start_rec.setEnabled(False)
        with mock.patch.object(recorder.QMessageBox, "warning") as warn:
            app._input_record_failed(None)
        warn.assert_called_once()
        self.assertFalse(app.recording_active)
        self.assertTrue(app.btn_start_rec.isEnabled())
        self.assertEqual(app.btn_start_rec.focusPolicy(),
                         Qt.FocusPolicy.StrongFocus)
        self.assertEqual(app.btn_start_rec.text(), common.T("Kaydı Başlat"))
        self.assertTrue(app.chk_mouse_path.isEnabled())
        proc.deleteLater.assert_called_once()

    def test_stop_uses_pause_sentinel(self):
        app = self._new_app()
        self._start_recording_ui(app)
        app._input_record_proc = mock.Mock()
        with mock.patch.object(recorder.winplatform, "send_pause_key",
                               return_value=True) as send:
            app._stop_recording()
        send.assert_called_once()
        self.assertFalse(app.recording_active)
        app._input_record_proc = None

    def test_pause_sentinel_failure_falls_back_to_terminate(self):
        app = self._new_app()
        self._start_recording_ui(app)
        app._input_record_proc = mock.Mock()
        with mock.patch.object(recorder.winplatform, "send_pause_key",
                               return_value=False), \
                mock.patch.object(app, "_terminate_record_process") as term:
            app._stop_recording()
        term.assert_called_once()
        app._input_record_proc = None

    def test_panic_key_still_triggers_when_not_recording(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        app.recording_active = False
        with mock.patch.object(app, "trigger_panic") as panic:
            QTest.keyClick(app, Qt.Key.Key_Pause)
        panic.assert_called_once()

    def test_panic_key_does_not_fire_while_recording(self):
        Qt, QTest, _QApplication = _qt()
        app = self._new_app()
        self._start_recording_ui(app)
        with mock.patch.object(app, "trigger_panic") as panic:
            QTest.keyClick(app, Qt.Key.Key_Pause)
        panic.assert_not_called()
        self.assertTrue(app.recording_active)


if __name__ == "__main__":
    unittest.main()
