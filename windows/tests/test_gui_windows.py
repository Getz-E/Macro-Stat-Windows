# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Windows GUI (Faz 4) birim testleri.

Testler Linux'ta da çalışır: Windows'a özgü Win32 çağrıları sahte
(monkeypatch) katmanlarla sınanır, gerçek süreç/kanal açılmaz. Qt gerekli
olan kurulum testleri ``QT_QPA_PLATFORM=offscreen`` ile çalıştırılmalıdır.

Çalıştırma (depo kökünden):

    QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

#: `macrostat.common` içe aktarılırken gerçek kullanıcı dizinlerine yazmasın.
_TMP_ROOT = tempfile.mkdtemp(prefix="macrostat-gui-tests-")
os.environ.setdefault("APPDATA", _TMP_ROOT)
os.environ.setdefault("LOCALAPPDATA", _TMP_ROOT)
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

GUI_DIR = Path(__file__).resolve().parents[1] / "gui"
WINDOWS_DIR = GUI_DIR.parent
if str(GUI_DIR) not in sys.path:
    sys.path.insert(0, str(GUI_DIR))

from macrostat import common, recorder, settings, winplatform  # noqa: E402
from macrostat import i18n  # noqa: E402


class RuntimeInfoTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.runtime = Path(self.dir.name) / "runtime.json"
        self.addCleanup(self.dir.cleanup)
        patcher = mock.patch.object(common, "RUNTIME_FILE", self.runtime)
        patcher.start()
        self.addCleanup(patcher.stop)

    def _write(self, payload):
        self.runtime.write_text(json.dumps(payload), encoding="utf-8")

    def _valid_payload(self):
        return {
            "pid": 4242,
            "creationTime": 133000000000000000,
            "exePath": r"C:\Program Files\MacroStat\macrostat-engine.exe",
            "sid": "S-1-5-21-1",
            "pipeName": r"\\.\pipe\macrostat-S-1-5-21-1-abcd",
            "logPath": r"C:\Users\u\AppData\Local\MacroStat\engine.log",
        }

    def test_valid_runtime_is_parsed(self):
        self._write(self._valid_payload())
        info = common.read_runtime_info()
        self.assertIsNotNone(info)
        self.assertEqual(info["pid"], 4242)

    def test_missing_file_returns_none(self):
        self.assertIsNone(common.read_runtime_info())
        self.assertIsNone(common.engine_pid())

    def test_bool_pid_is_rejected(self):
        payload = self._valid_payload()
        payload["pid"] = True
        self._write(payload)
        self.assertIsNone(common.read_runtime_info())

    def test_missing_pipe_name_is_rejected(self):
        payload = self._valid_payload()
        del payload["pipeName"]
        self._write(payload)
        self.assertIsNone(common.read_runtime_info())

    def test_broken_json_is_rejected(self):
        self.runtime.write_text("{not json", encoding="utf-8")
        self.assertIsNone(common.read_runtime_info())


class IpcTests(unittest.TestCase):
    def _run_with(self, **kwargs):
        result = subprocess.CompletedProcess(
            args=[], returncode=kwargs.get("returncode", 0),
            stdout=kwargs.get("stdout", ""), stderr=kwargs.get("stderr", ""))
        with mock.patch.object(common, "find_engine",
                               return_value=Path("/fake/engine")), \
                mock.patch.object(common, "_hidden_run", return_value=result):
            return common.ipc_command("ping")

    def test_ok_reply_is_parsed_from_last_line(self):
        ok, payload = self._run_with(
            stdout='noise line\n{"ok":true,"cmd":"ping"}\n\n')
        self.assertTrue(ok)
        self.assertEqual(payload["cmd"], "ping")

    def test_error_payload_is_returned(self):
        ok, detail = self._run_with(stdout='{"ok":false,"error":"denied"}')
        self.assertFalse(ok)
        self.assertEqual(detail, "denied")

    def test_nonzero_exit_carries_stderr(self):
        ok, detail = self._run_with(returncode=6, stderr="no channel")
        self.assertFalse(ok)
        self.assertIn("no channel", detail)

    def test_unparseable_reply_fails(self):
        ok, _detail = self._run_with(stdout="not json")
        self.assertFalse(ok)

    def test_missing_engine_fails_cleanly(self):
        with mock.patch.object(common, "find_engine", return_value=None):
            ok, detail = common.ipc_command("ping")
        self.assertFalse(ok)
        self.assertIn("macrostat-engine.exe", detail)


class EngineRunningTests(unittest.TestCase):
    def setUp(self):
        self.info = {"pid": 123, "creationTime": 999, "pipeName": "p",
                     "exePath": "e", "sid": "s", "logPath": "l"}
        patcher = mock.patch.object(common, "read_runtime_info",
                                    return_value=self.info)
        patcher.start()
        self.addCleanup(patcher.stop)
        for name in ("process_alive", "process_creation_time",
                     "process_image_path"):
            patcher = mock.patch.object(winplatform, name)
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_dead_process_is_not_running(self):
        winplatform.process_alive.return_value = False
        self.assertFalse(common.engine_running())

    def test_reused_pid_is_not_running(self):
        winplatform.process_alive.return_value = True
        winplatform.process_creation_time.return_value = 1000
        self.assertFalse(common.engine_running())

    def test_foreign_image_is_not_running(self):
        winplatform.process_alive.return_value = True
        winplatform.process_creation_time.return_value = 999
        winplatform.process_image_path.return_value = str(
            Path("/Windows") / "notepad.exe")
        self.assertFalse(common.engine_running())

    def test_matching_identity_is_running(self):
        winplatform.process_alive.return_value = True
        winplatform.process_creation_time.return_value = 999
        winplatform.process_image_path.return_value = str(
            Path("/Apps") / common.DAEMON_NAME)
        self.assertTrue(common.engine_running())


class PrivateFileTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.path = Path(self.dir.name) / "profile.json"

    def test_roundtrip(self):
        common.write_private_bytes(self.path, b'{"a":1}')
        self.assertEqual(common.read_private_bytes(self.path), b'{"a":1}')
        self.assertEqual(common.read_private_text_file(self.path), '{"a":1}')

    def test_oversize_is_rejected(self):
        common.write_private_bytes(self.path, b"x" * 64)
        with self.assertRaises(OSError):
            common.read_private_bytes(self.path, max_bytes=16)

    def test_bad_utf8_returns_none(self):
        common.write_private_bytes(self.path, b"\xff\xfe\x00")
        self.assertIsNone(common.read_private_text_file(self.path))

    def test_symlink_is_refused(self):
        target = Path(self.dir.name) / "target.json"
        common.write_private_bytes(target, b"secret")
        try:
            os.symlink(target, self.path)
        except (OSError, NotImplementedError):
            self.skipTest("symlink desteklenmiyor")
        with self.assertRaises(OSError):
            common.read_private_bytes(self.path)

    def test_backup_copies_source_to_destination(self):
        common.write_private_bytes(self.path, b"payload")
        backup = Path(self.dir.name) / "backup.json"
        common.backup_private_file(self.path, backup)
        self.assertEqual(backup.read_bytes(), b"payload")

    def test_rotated_log_keeps_previous_copy(self):
        log = Path(self.dir.name) / "engine.log"
        common.write_private_bytes(log, b"x" * 32)
        common.open_rotated_log(log, max_bytes=8)
        self.assertTrue((Path(self.dir.name) / "engine.log.1").is_file())
        self.assertFalse(log.exists())


class SanitizerTests(unittest.TestCase):
    def test_unknown_button_is_dropped(self):
        data = {"buttons": [[10, "BTN_LEFT", 1], [20, "BTN_NOPE", 1]]}
        self.assertEqual(recorder.sanitize_mouse_buttons(data),
                         [[10, "BTN_LEFT", 1]])

    def test_key_events_require_known_names(self):
        data = {"keys": [[5, "KEY_A", 1], [6, "NOT_A_KEY", 1],
                         [7, "KEY_A", 2]]}
        self.assertEqual(recorder.sanitize_key_events(data),
                         [[5, "KEY_A", 1]])

    def test_malformed_channels_return_empty(self):
        self.assertEqual(recorder.sanitize_key_events(None), [])
        self.assertEqual(recorder.sanitize_key_events({"keys": "x"}), [])
        self.assertEqual(recorder.sanitize_mouse_buttons({}), [])

    def test_negative_time_is_clamped(self):
        data = {"keys": [[-50, "KEY_A", 0]]}
        self.assertEqual(recorder.sanitize_key_events(data),
                         [[0, "KEY_A", 0]])


class MergeInputTests(unittest.TestCase):
    def test_points_only(self):
        events = recorder.merge_input_events([[5, 1, 2], [5, 3, 4]], [], [])
        self.assertEqual(events, [{
            "type": "MOUSE_PATH",
            "points": [[5, 1, 2], [5, 3, 4]]}])

    def test_empty_recording(self):
        self.assertEqual(recorder.merge_input_events([], [], []), [])

    def test_key_splits_path_and_adds_single_delay(self):
        events = recorder.merge_input_events(
            [[10, 0, 0], [10, 0, 0]], [], [[100, "KEY_A", 1]])
        self.assertEqual([e["type"] for e in events],
                         ["MOUSE_PATH", "DELAY", "KEY_DOWN"])
        self.assertEqual(events[1]["delayMs"], 80)
        self.assertEqual(events[2]["key"], "KEY_A")

    def test_mouse_event_orders_before_key_at_same_time(self):
        # Kaydın başındaki bekleme de korunur: 5 ms'lik baş DELAY.
        events = recorder.merge_input_events(
            [], [[5, "BTN_LEFT", 1]], [[5, "KEY_A", 1]])
        self.assertEqual([e["type"] for e in events],
                         ["DELAY", "MOUSE_DOWN", "KEY_DOWN"])
        self.assertEqual(events[0]["delayMs"], 5)

    def test_trailing_points_are_emitted_after_last_event(self):
        events = recorder.merge_input_events(
            [[10, 1, 1], [20, 2, 2]], [], [[5, "KEY_A", 1]])
        self.assertEqual([e["type"] for e in events],
                         ["DELAY", "KEY_DOWN", "MOUSE_PATH"])
        # Mutlak zamanlar 10 ve 30 ms; ilk nokta son olaydan (5 ms) sonra
        # geçen süreyle ölçülür, ikinci nokta kendi dt'sini korur.
        self.assertEqual(events[2]["points"], [[5, 1, 1], [20, 2, 2]])

    def test_backward_compatible_wrapper(self):
        self.assertEqual(
            recorder.merge_mouse_path_buttons([[1, 0, 0]], []),
            [{"type": "MOUSE_PATH", "points": [[1, 0, 0]]}])


class PathAndKeyTests(unittest.TestCase):
    def test_restart_key_validation(self):
        self.assertEqual(common.normalize_restart_key("key_f9"), "KEY_F9")
        self.assertEqual(common.normalize_restart_key("not a key"),
                         common.DEFAULT_RESTART_KEY)
        self.assertEqual(common.normalize_restart_key(None),
                         common.DEFAULT_RESTART_KEY)

    def test_restart_watch_command(self):
        command = common.restart_watch_command(Path("/e/engine"), "KEY_F12")
        self.assertEqual(command[1:], ["--watch-restart", "KEY_F12"])

    def test_default_profile_template_exists(self):
        self.assertTrue(common.DEFAULT_PROFILE_PATH.is_file())
        data = json.loads(common.DEFAULT_PROFILE_PATH.read_text("utf-8"))
        self.assertEqual(data["schemaVersion"], 1)
        self.assertTrue(data["profiles"])


class PackagingLayoutTests(unittest.TestCase):
    """Paket sözleşmesi: PyInstaller düzeni, sürüm ve kurulum ilkeleri."""

    def test_engine_candidates_cover_pyinstaller_layout(self):
        internal = common.SCRIPT_DIR / "_internal" / common.DAEMON_NAME
        self.assertIn(internal, common.engine_candidates())

    def test_profile_template_candidate_covers_pyinstaller_layout(self):
        candidate = common.SCRIPT_DIR / "_internal" / "assets" / "default_profile.json"
        self.assertIn(candidate, common._default_profile_candidates())

    def test_frozen_app_dir_uses_executable_directory(self):
        executable = "/opt/MacroStat/MacroStat.exe"
        with mock.patch.object(sys, "frozen", True, create=True), \
                mock.patch.object(sys, "executable", executable):
            self.assertEqual(common._app_dir(),
                             Path(executable).resolve().parent)

    def test_version_is_consistent(self):
        import re
        version = (WINDOWS_DIR / "VERSION").read_text("utf-8").strip()
        self.assertRegex(version, r"^\d+\.\d+\.\d+$")
        cargo = (WINDOWS_DIR / "engine" / "Cargo.toml").read_text("utf-8")
        self.assertRegex(cargo,
                         r'(?m)^version\s*=\s*"' + re.escape(version) + '"')
        info = (WINDOWS_DIR / "gui" / "version_info.txt").read_text("utf-8")
        self.assertIn(f"'{version}'", info)  # ProductVersion

    def test_packaging_files_exist(self):
        for relative in ("gui/MacroStat.spec", "gui/version_info.txt",
                         "installer/MacroStat.iss", "scripts/build.ps1",
                         "scripts/sign.ps1", "scripts/verify.ps1",
                         "LICENSE", "THIRD_PARTY.md"):
            self.assertTrue((WINDOWS_DIR / relative).is_file(), relative)

    def test_installer_is_user_scoped(self):
        script = (WINDOWS_DIR / "installer" / "MacroStat.iss").read_text("utf-8-sig")
        self.assertIn("PrivilegesRequired=lowest", script)
        self.assertIn("{localappdata}", script)
        self.assertIn("MB_DEFBUTTON2", script)


class TranslationCoverageTests(unittest.TestCase):
    """Paketteki her sabit ``T("...")`` metninin EN karşılığı olmalı."""

    def test_every_literal_has_english_translation(self):
        import ast

        missing = []
        for path in sorted(Path(GUI_DIR, "macrostat").glob("*.py")):
            tree = ast.parse(path.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if not isinstance(node, ast.Call):
                    continue
                if not (isinstance(node.func, ast.Name)
                        and node.func.id == "T"):
                    continue
                for arg in node.args[:1]:
                    if (isinstance(arg, ast.Constant)
                            and isinstance(arg.value, str)
                            and arg.value not in i18n._EN):
                        missing.append(f"{path.name}:{node.lineno} "
                                       f"{arg.value[:50]!r}")
        self.assertEqual(missing, [], "\n".join(missing))

    def test_placeholders_are_preserved(self):
        bad = [key for key, value in i18n._EN.items()
               if isinstance(key, str) and isinstance(value, str)
               and key.count("{}") != value.count("{}")]
        self.assertEqual(bad, [])


class SettingsTests(unittest.TestCase):
    def test_defaults_when_missing(self):
        path = Path(tempfile.mkdtemp(prefix="macrostat-settings-")) / "s.json"
        with mock.patch.object(settings, "SETTINGS_PATH", path):
            data = settings.load_settings()
            self.assertIn(data["language"], ("tr", "en"))
            self.assertIn(data["theme"], settings.THEME_MODES)

    def test_invalid_values_fall_back(self):
        path = Path(tempfile.mkdtemp(prefix="macrostat-settings-")) / "s.json"
        path.write_text(json.dumps({"language": "de", "theme": "neon"}),
                        encoding="utf-8")
        with mock.patch.object(settings, "SETTINGS_PATH", path):
            data = settings.load_settings()
        self.assertIn(data["language"], ("tr", "en"))
        self.assertEqual(data["theme"], "dark")

    def test_oversized_file_is_not_read(self):
        path = Path(tempfile.mkdtemp(prefix="macrostat-settings-")) / "s.json"
        path.write_bytes(b"{" + b" " * (settings.MAX_SETTINGS_BYTES + 1) + b"}")
        with mock.patch.object(settings, "SETTINGS_PATH", path):
            data = settings.load_settings()
        self.assertIn(data["language"], ("tr", "en"))
        self.assertEqual(data["theme"], "dark")

    def test_reparse_point_is_refused(self):
        root = Path(tempfile.mkdtemp(prefix="macrostat-settings-"))
        target = root / "target.json"
        target.write_text(json.dumps({"language": "en", "theme": "light"}),
                          encoding="utf-8")
        link = root / "s.json"
        try:
            link.symlink_to(target)
        except (OSError, NotImplementedError):
            self.skipTest("symlinks unavailable")
        with mock.patch.object(settings, "SETTINGS_PATH", link):
            data = settings.load_settings()
            self.assertIn(data["language"], ("tr", "en"))
            self.assertFalse(settings.save_settings({"language": "en"}))
        self.assertEqual(
            json.loads(target.read_text(encoding="utf-8"))["language"], "en",
            "a symlinked settings file must not be written through")


class OnboardingCheckTests(unittest.TestCase):
    def test_checks_have_expected_keys(self):
        from macrostat import onboarding
        keys = [key for key, *_ in onboarding.collect_checks()]
        self.assertEqual(keys, ["engine", "datadir", "profile", "job",
                                "privilege"])

    def test_onboarding_never_requires_admin(self):
        from macrostat import onboarding
        for _key, ok, title, _detail in onboarding.collect_checks():
            if "admin" in title.lower() or "yönetici" in title.lower():
                self.assertTrue(ok, title)


if __name__ == "__main__":
    unittest.main(verbosity=2)
