# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Canlı kaydedici (Windows): sistem geneli klavye + fare.

Linux'taki Qt içi kayıt yolunun yerini motorun `--record-input` kipi alır:
düşük seviyeli kanca tüm masaüstünü dinler, tuş/fare düğmesi/topçuğu
zaman damgalarıyla tek bir JSON belgesine yazar. Kayıtçı yalnızca panik
tuşunda (Pause) temiz kapanıp dosyayı atomik yazar; bu yüzden "Durdur"
düğmesi `SendInput` ile sentinel Pause tuşu enjekte eder.
"""

import json
import os
import time
from pathlib import Path

from . import common, theme, winplatform
from .common import *  # noqa: F401,F403
from .widgets import SlidingSwitch, KeyCaptureDialog  # noqa: F401

#: Fare kaydında kabul edilen düğme adları (motorla aynı küme).
_MOUSE_BUTTON_NAMES = frozenset(
    {"BTN_LEFT", "BTN_RIGHT", "BTN_MIDDLE", "BTN_SIDE", "BTN_EXTRA"})
_MAX_MOUSE_POINTS = 200_000
_MAX_MOUSE_DT_MS = 600_000
_MAX_KEY_EVENTS = 200_000


def _sanitize_channel(data, field, allowed, limit=_MAX_KEY_EVENTS):
    """`data[field]` içindeki ``[t, AD, 0|1]`` üçlülerini doğrular."""
    if not isinstance(data, dict):
        return []
    raw = data.get(field)
    if not isinstance(raw, list):
        return []
    events = []
    for entry in raw:
        if not isinstance(entry, (list, tuple)) or len(entry) != 3:
            continue
        when, name, down = entry
        if not isinstance(name, str) or name not in allowed:
            continue
        try:
            when = int(when)
            down = int(down)
        except (TypeError, ValueError):
            continue
        if down not in (0, 1):
            continue
        events.append([max(0, min(when, _MAX_MOUSE_DT_MS)), name, down])
        if len(events) >= limit:
            break
    return events


def sanitize_mouse_buttons(data):
    """Motorun kaydettiği fare düğmesi olaylarını doğrular ve sınırlara çeker."""
    return _sanitize_channel(data, "buttons", _MOUSE_BUTTON_NAMES)


def sanitize_key_events(data):
    """Motorun kaydettiği klavye olaylarını doğrular.

    Yalnızca ``KEY_*`` adları kabul edilir; `KEY_CODE_*` gibi Qt'ye özgü
    geçici adlar motor tarafından üretilmediğinden reddedilir.
    """
    if not isinstance(data, dict):
        return []
    raw = data.get("keys")
    if not isinstance(raw, list):
        return []
    events = []
    for entry in raw:
        if not isinstance(entry, (list, tuple)) or len(entry) != 3:
            continue
        when, name, down = entry
        if not isinstance(name, str) or not is_valid_key_name(
                name, allow_button=False):
            continue
        try:
            when = int(when)
            down = int(down)
        except (TypeError, ValueError):
            continue
        if down not in (0, 1):
            continue
        events.append([max(0, min(when, _MAX_MOUSE_DT_MS)), name, down])
        if len(events) >= _MAX_KEY_EVENTS:
            break
    return events


def widget_is_within(widget, ancestor):
    """`widget` kendisi veya bir alt/üst öğesiyle `ancestor` içinde mi?"""
    seen = 0
    while widget is not None and seen < 64:
        if widget is ancestor:
            return True
        parent_widget = getattr(widget, "parentWidget", None)
        if parent_widget is None:
            return False
        widget = parent_widget()
        seen += 1
    return False


def merge_input_events(points, buttons, keys):
    """Nokta, fare düğmesi ve tuş olaylarını tek zaman çizelgesinde birleştirir.

    Noktaların ``dt`` değerleri artımlı, düğme/tuş zamanları mutlaktır.
    Noktalar önce mutlak zamana çevrilir; her bölümün ilk ``dt`` değeri bir
    önceki yayımlanan olaydan geçen süreye göre yeniden hesaplanır, böylece
    bekleme iki kez sayılmaz.
    """
    cleaned = [list(p) for p in points
               if isinstance(p, (list, tuple)) and len(p) == 3]
    combined = []
    for when, name, down in buttons:
        combined.append((int(when), 0, name, int(down)))
    for when, name, down in keys:
        combined.append((int(when), 1, name, int(down)))
    combined.sort(key=lambda entry: (entry[0], entry[1]))

    if not combined:
        return ([{"type": "MOUSE_PATH", "points": cleaned}] if cleaned else [])

    absolute = []
    elapsed = 0
    for dt, dx, dy in cleaned:
        elapsed += max(0, int(dt))
        absolute.append((elapsed, dx, dy))

    def take_points(start, stop, previous):
        segment = []
        for when_point, dx, dy in absolute[start:stop]:
            segment.append([max(0, when_point - previous), dx, dy])
            previous = when_point
        return segment, previous

    events = []
    index = 0
    base = 0
    for when, kind, name, down in combined:
        start = index
        while index < len(absolute) and absolute[index][0] <= when:
            index += 1
        if index > start:
            segment, base = take_points(start, index, base)
            events.append({"type": "MOUSE_PATH", "points": segment})
        gap = when - base
        if gap > 0:
            events.append({"type": "DELAY", "delayMs": gap, "jitterMs": 2})
        base = max(base, when)
        if kind == 0:
            events.append({
                "type": "MOUSE_DOWN" if down else "MOUSE_UP",
                "mouseButton": name,
            })
        else:
            events.append({
                "type": "KEY_DOWN" if down else "KEY_UP",
                "key": name,
            })
    if index < len(absolute):
        tail, _ = take_points(index, len(absolute), base)
        events.append({"type": "MOUSE_PATH", "points": tail})
    return events


def merge_mouse_path_buttons(points, buttons):
    """Geriye dönük uyumluluk: yalnızca nokta + düğme birleştirmesi."""
    return merge_input_events(points, buttons, [])


class RecorderMixin:
    """Sistem geneli kayıt (motor `--record-input`)."""

    def setup_recorder_tab(self):
        layout = QVBoxLayout(self.tab_recorder)
        layout.addWidget(QLabel(T(
            "Milisaniye hassasiyetinde klavye, fare tuşları ve tekerlek "
            "olaylarını sistem genelinde kaydeder.")))

        banner = QFrame()
        banner.setObjectName("dangerBanner")
        banner_layout = QHBoxLayout(banner)
        banner_layout.setContentsMargins(8, 4, 8, 4)
        label = QLabel(T(
            "Kayıt motor üzerinden yapılır. Kaydı bitirmenin güvenli yolu "
            "Pause tuşuna basmaktır; dosya ancak o zaman diske yazılır."))
        label.setObjectName("bannerText")
        label.setWordWrap(True)
        banner_layout.addWidget(label)
        layout.addWidget(banner)

        self.rec_table = QTableWidget()
        self.rec_table.setColumnCount(4)
        self.rec_table.setHorizontalHeaderLabels(
            [T("Sıra"), T("Olay"), T("Tuş / Buton"), T("Gecikme (ms)")])
        self.rec_table.horizontalHeader().setSectionResizeMode(
            2, QHeaderView.ResizeMode.Stretch)
        layout.addWidget(self.rec_table)

        self.chk_mouse_path = QCheckBox(T("Fare hareketini de kaydet"))
        self.chk_mouse_path.setChecked(True)
        self.chk_mouse_path.setToolTip(T(
            "Kapalıysa fare hareketi kayda eklenmez; tuş ve düğme olayları "
            "yine kaydedilir. Pause tuşu kaydı bitirir."))
        layout.addWidget(self.chk_mouse_path)

        # Sistem geneli kayıt alt süreci (motor --record-input).
        self._input_record_proc = None
        self._input_record_file = None
        self._input_record_started = 0.0
        self._input_record_timer = QTimer(self)
        self._input_record_timer.timeout.connect(self._update_record_label)

        b_box = QHBoxLayout()
        self.btn_start_rec = QPushButton(T("Kaydı Başlat"))
        theme.set_variant(self.btn_start_rec, "primary")
        self.btn_start_rec.clicked.connect(self.toggle_recording)
        b_box.addWidget(self.btn_start_rec)

        self.btn_clear_rec = QPushButton(T("Temizle"))
        self.btn_clear_rec.clicked.connect(self.clear_recorded)
        b_box.addWidget(self.btn_clear_rec)

        self.btn_conv_rec = QPushButton(T("Kaydı Yeni Makro Olarak Ekle"))
        self.btn_conv_rec.clicked.connect(self.convert_recording_to_macro)
        b_box.addWidget(self.btn_conv_rec)

        layout.addLayout(b_box)

    def toggle_recording(self):
        if self.recording_active:
            self._stop_recording()
        else:
            self._start_recording()

    def _start_recording(self):
        # Kayıt esnasında makroların devreye girmemesi için motoru durdur.
        if self.is_daemon_running():
            self.was_recording_daemon_running = True
            self.stop_daemon()
        else:
            self.was_recording_daemon_running = False

        if not self._start_input_recording():
            if getattr(self, "was_recording_daemon_running", False):
                self.was_recording_daemon_running = False
                self.restart_daemon(show_message=False)
            return

        self.recording_active = True
        self.btn_start_rec.setText(T("Kaydı Durdur"))
        theme.set_variant(self.btn_start_rec, "danger")
        self.chk_mouse_path.setEnabled(False)
        self.show_autosave_banner(T(
            "Sistem geneli kayıt başladı: bitirmek için Pause tuşuna basın "
            "veya düğmeyi kullanın."))

    def _record_file_path(self):
        return str(common.RUNTIME_DIR / f"macrostat-input-{os.getpid()}.json")

    def _start_input_recording(self):
        engine = common.find_engine()
        if engine is None:
            QMessageBox.warning(
                self, T("Hata"),
                T("Motor ikilisi bulunamadı. Önce windows/engine içinde "
                  "'cargo build --release --locked' çalıştırın."))
            return False
        self._input_record_file = self._record_file_path()
        try:
            Path(self._input_record_file).unlink(missing_ok=True)
        except OSError:
            pass

        start = winplatform.cursor_position()
        arguments = [
            "--record-input", self._input_record_file,
            "--panic-key", common.PANIC_KEY,
        ]
        if start:
            arguments += ["--start-x", str(start[0]), "--start-y", str(start[1])]

        proc = QProcess(self)
        try:
            from .system import hide_console
            hide_console(proc)
        except Exception:
            pass
        proc.setProgram(str(engine))
        proc.setArguments(arguments)
        proc.finished.connect(self._input_record_finished)
        proc.errorOccurred.connect(self._input_record_failed)
        self._input_record_proc = proc
        self._input_record_started = time.time()
        proc.start()
        if not proc.waitForStarted(1500):
            if self._input_record_proc is None:
                return False
            self._input_record_proc = None
            QMessageBox.warning(
                self, T("Hata"),
                T("Kayıt başlatılamadı: {}").format(proc.errorString()))
            return False
        self._input_record_timer.start(500)
        self._update_record_label()
        return True

    def _update_record_label(self):
        if self._input_record_proc is None:
            return
        seconds = max(0, int(time.time() - self._input_record_started))
        self.btn_start_rec.setText(T("Durdur ({} sn)").format(seconds))

    def _stop_recording(self):
        self.recording_active = False
        self.releaseKeyboard()
        if self._input_record_proc is not None:
            self._input_record_timer.stop()
            self.btn_start_rec.setEnabled(False)
            self.btn_start_rec.setText(T("Kayıt bitiriliyor..."))
            # Kayıtçı yalnızca panik tuşunda temiz kapanıp dosyayı yazar.
            if not winplatform.send_pause_key():
                self._terminate_record_process()
                return
            QTimer.singleShot(2000, self._terminate_record_process)
            return
        self._finish_recording_ui()

    def _terminate_record_process(self):
        proc = self._input_record_proc
        if proc is None:
            return
        try:
            if proc.state() != QProcess.ProcessState.NotRunning:
                proc.terminate()
        except Exception:
            pass

    def _finish_recording_ui(self):
        self.btn_start_rec.setEnabled(True)
        self.btn_start_rec.setText(T("Kaydı Başlat"))
        theme.set_variant(self.btn_start_rec, "primary")
        self.chk_mouse_path.setEnabled(True)
        if getattr(self, "was_recording_daemon_running", False):
            self.was_recording_daemon_running = False
            self.restart_daemon(show_message=False)
            self.show_autosave_banner(T("Makro motoru tekrar aktif edildi."))

    def _input_record_failed(self, _error):
        if self._input_record_proc is None:
            return
        proc = self._input_record_proc
        self._input_record_proc = None
        self._input_record_timer.stop()
        self.recording_active = False
        message = proc.errorString() or T("bilinmeyen hata")
        self._cleanup_record_file()
        self._finish_recording_ui()
        proc.deleteLater()
        QMessageBox.warning(self, T("Hata"),
                            T("Kayıt hatası: {}").format(message))

    def _cleanup_record_file(self):
        path = self._input_record_file
        self._input_record_file = None
        if path:
            try:
                Path(path).unlink(missing_ok=True)
            except OSError:
                pass

    @staticmethod
    def _sanitize_mouse_path(data):
        """Kayıt dosyasındaki noktaları doğrular ve güvenli sınırlara çeker."""
        if not isinstance(data, dict):
            return []
        raw_points = data.get("points")
        if not isinstance(raw_points, list):
            return []
        points = []
        for point in raw_points:
            if not isinstance(point, (list, tuple)) or len(point) != 3:
                continue
            try:
                dt, dx, dy = (int(v) for v in point)
            except (TypeError, ValueError):
                continue
            points.append([
                max(0, min(dt, _MAX_MOUSE_DT_MS)),
                max(-2_147_483_648, min(dx, 2_147_483_647)),
                max(-2_147_483_648, min(dy, 2_147_483_647)),
            ])
            if len(points) >= _MAX_MOUSE_POINTS:
                break
        return points

    @staticmethod
    def _sanitize_mouse_start(data):
        """Kayıt dosyasındaki isteğe bağlı başlangıç imleç konumunu doğrular."""
        if not isinstance(data, dict):
            return None
        raw = data.get("start")
        if not isinstance(raw, (list, tuple)) or len(raw) != 2:
            return None
        try:
            x, y = (int(v) for v in raw)
        except (TypeError, ValueError):
            return None
        limit = 1_000_000
        return [max(-limit, min(x, limit)), max(-limit, min(y, limit))]

    def _input_record_finished(self, exit_code, _exit_status):
        if self._input_record_proc is None:
            return
        proc = self._input_record_proc
        self._input_record_proc = None
        self._input_record_timer.stop()
        self.recording_active = False
        stderr = bytes(proc.readAllStandardError()).decode(
            "utf-8", "replace").strip()
        if exit_code != 0 and stderr:
            QMessageBox.warning(
                self, T("Kayıt hatası: {}").format(stderr.splitlines()[-1]))

        points = []
        buttons = []
        keys = []
        start = None
        path = self._input_record_file
        if path and Path(path).exists():
            try:
                with open(path, "r", encoding="utf-8") as handle:
                    data = json.load(handle)
                points = self._sanitize_mouse_path(data)
                buttons = sanitize_mouse_buttons(data)
                keys = sanitize_key_events(data)
                start = self._sanitize_mouse_start(data)
            except (OSError, ValueError, json.JSONDecodeError) as exc:
                QMessageBox.warning(
                    self, T("Kayıt okunamadı: {}").format(exc))
        self._cleanup_record_file()

        if not self.chk_mouse_path.isChecked():
            points = []
        if points or buttons or keys:
            events = merge_input_events(points, buttons, keys)
            if start is not None and points:
                for event in events:
                    if event["type"] == "MOUSE_PATH":
                        event["start"] = start
                        break
            self.recorded_events.extend(events)
            self.refresh_rec_table()
            total_ms = sum(p[0] for p in points)
            self.show_autosave_banner(T(
                "{} tuş, {} fare olayı, {} nokta kaydedildi ({} ms).").format(
                    len(keys), len(buttons), len(points), total_ms))
        elif exit_code == 0:
            self.show_autosave_banner(T("Kayıt boş; olay algılanmadı."))

        proc.deleteLater()
        self._finish_recording_ui()

    def map_qt_key_to_evdev(self, qt_key, text):
        return KeyCaptureDialog.map_qt_key_to_evdev(self, qt_key, text)

    def keyPressEvent(self, event):
        # Kayıt sırasında tuşları motor yakalar; Pause kaydı bitirir ve
        # panik tetiklemez.
        if self.recording_active:
            if event.key() in (Qt.Key.Key_Pause, Qt.Key.Key_Escape):
                return
            super().keyPressEvent(event)
            return
        # STOP kontrolü (motorun kabul ettiği KEY_* adlarıyla karşılaştırılır)
        pressed_key = self.map_qt_key_to_evdev(event.key(), event.text())
        if pressed_key == common.PANIC_KEY or event.key() == Qt.Key.Key_Pause:
            self.trigger_panic()
            return
        if event.key() == Qt.Key.Key_Escape:
            self.tabs.setCurrentIndex(0)
            return
        super().keyPressEvent(event)

    def keyReleaseEvent(self, event):
        super().keyReleaseEvent(event)

    def refresh_rec_table(self):
        self.rec_table.setRowCount(len(self.recorded_events))
        for index, event in enumerate(self.recorded_events):
            event_type = event.get("type", "")
            if event_type == "MOUSE_PATH":
                points = event.get("points") or []
                detail = T("{} nokta").format(len(points))
                timing = "{} ms".format(
                    sum(int(p[0]) for p in points if len(p) == 3))
            else:
                detail = event.get("key") or event.get("mouseButton") or "-"
                timing = str(event.get("delayMs", "-"))
            self.rec_table.setItem(index, 0, QTableWidgetItem(str(index + 1)))
            self.rec_table.setItem(index, 1, QTableWidgetItem(event_type))
            self.rec_table.setItem(index, 2, QTableWidgetItem(detail))
            self.rec_table.setItem(index, 3, QTableWidgetItem(timing))

    def clear_recorded(self):
        self.recorded_events.clear()
        self._recording_key_names = {}
        self.rec_table.setRowCount(0)

    def convert_recording_to_macro(self):
        if not self.recorded_events:
            return
        new_macro = {
            "id": f"macro-{int(time.time() * 1000)}",
            "name": T("Kayıtlı Makro ({} adım)").format(len(self.recorded_events)),
            "description": "Canlı kaydedici ile oluşturuldu",
            "enabled": True,
            "triggerKey": "KEY_LEFTSHIFT+KEY_LEFTCTRL",
            "triggerMode": "ONCE",
            "repeatCount": 1,
            "actions": list(self.recorded_events),
        }
        self.profile_data.setdefault("macros", []).append(new_macro)
        self.save_profile_to_disk(restart_daemon=True)
        self.refresh_macro_table()
        self.tabs.setCurrentWidget(self.tab_macros)
