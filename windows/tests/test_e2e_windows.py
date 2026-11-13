# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
r"""Faz 7 — uçtan uca (E2E) motor kabul testleri.

Bu testler **yalnızca Windows'ta** anlamlıdır; diğer platformlarda tüm sınıf
atlanır. Gerçek `macrostat-engine.exe` ikilisini çalıştırır, adlandırılmış
kanalı, `runtime.json`'ı, tek örnek kilidini, panik tuşunu ve kayıt akışını
canlı sınar.

İkili konumu:

1. ``MACROSTAT_E2E_ENGINE`` ortam değişkeni,
2. ``windows/dist/MacroStat/_internal/macrostat-engine.exe`` (CI artefaktı),
3. ``windows/engine/target/{release,debug}/macrostat-engine.exe``.

CI:
    python -m unittest discover -s windows/tests -p "test_e2e*.py" -v

Yerel (Windows):
    $env:MACROSTAT_E2E_ENGINE = "C:\...\macrostat-engine.exe"
    python -m unittest discover -s windows/tests -p "test_e2e*.py" -v
"""

import ctypes
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE_NAME = "macrostat-engine.exe"
VK_A = 0x41
VK_F24 = 0x87
KEYEVENTF_KEYUP = 0x0002
MOUSEEVENTF_MOVE = 0x0001


def find_engine():
    override = os.environ.get("MACROSTAT_E2E_ENGINE")
    if override:
        path = Path(override)
        return path if path.is_file() else None
    candidates = [
        REPO_ROOT / "windows" / "dist" / "MacroStat" / "_internal" / ENGINE_NAME,
        REPO_ROOT / "windows" / "engine" / "target" / "release" / ENGINE_NAME,
        REPO_ROOT / "windows" / "engine" / "target" / "debug" / ENGINE_NAME,
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


def profile_document(name="e2e", panic_key="KEY_F24"):
    return {
        "schemaVersion": 1,
        "id": "e2e-profile",
        "name": name,
        "panicKey": panic_key,
        "macros": [
            {
                "id": "e2e-delay",
                "name": "E2E Delay",
                "triggerKey": "KEY_F13",
                "triggerMode": "TOGGLE_LOOP",
                "actions": [{"type": "DELAY", "delayMs": 30}],
            }
        ],
    }


@unittest.skipUnless(sys.platform == "win32", "E2E motor testleri Windows gerektirir")
class EngineE2ETests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.engine = find_engine()
        if cls.engine is None:
            raise unittest.SkipTest(
                "macrostat-engine.exe bulunamadı (MACROSTAT_E2E_ENGINE ayarlayın)"
            )
        cls.user32 = ctypes.WinDLL("user32", use_last_error=True)
        cls.user32.keybd_event.argtypes = [
            ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_uint, ctypes.c_size_t
        ]
        cls.user32.mouse_event.argtypes = [
            ctypes.c_uint, ctypes.c_long, ctypes.c_long, ctypes.c_uint,
            ctypes.c_size_t
        ]

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="macrostat-e2e-"))
        self.local = self.root / "local"
        self.local.mkdir()
        self.profile = self.root / "profile.json"
        self.profile.write_text(json.dumps(profile_document()), encoding="utf-8")
        self.log = self.root / "engine.log"
        self.processes = []

    def tearDown(self):
        for process in self.processes:
            if process.poll() is None:
                process.kill()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass
        shutil.rmtree(self.root, ignore_errors=True)

    # -- yardımcılar -----------------------------------------------------

    def engine_env(self):
        env = os.environ.copy()
        env["LOCALAPPDATA"] = str(self.local)
        return env

    def spawn(self, *args):
        process = subprocess.Popen(
            [str(self.engine), *map(str, args)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=self.engine_env(),
        )
        self.processes.append(process)
        return process

    def start_engine(self):
        return self.spawn(self.profile, "--log", self.log)

    def wait_for(self, predicate, timeout=15.0, interval=0.1):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if predicate():
                return True
            time.sleep(interval)
        return False

    def runtime_path(self):
        return self.local / "MacroStat" / "runtime.json"

    def read_log(self):
        try:
            return self.log.read_text(encoding="utf-8", errors="replace")
        except OSError:
            return ""

    def wait_for_runtime(self):
        self.assertTrue(
            self.wait_for(lambda: self.runtime_path().is_file()),
            "runtime.json oluşmadı",
        )
        info = json.loads(self.runtime_path().read_text(encoding="utf-8"))
        self.assertTrue(info["pipeName"].startswith("\\\\.\\pipe\\macrostat-"))
        return info

    def ipc(self, command):
        return subprocess.run(
            [str(self.engine), "--ipc", command],
            capture_output=True,
            text=True,
            timeout=20,
            env=self.engine_env(),
        )

    def inject_key(self, vk):
        self.user32.keybd_event(vk, 0, 0, 0)
        time.sleep(0.02)
        self.user32.keybd_event(vk, 0, KEYEVENTF_KEYUP, 0)

    def stop_engine(self):
        process = self.start_engine()
        self.wait_for_runtime()
        return process

    # -- testler ---------------------------------------------------------

    def test_version_and_invalid_profile(self):
        version = subprocess.run(
            [str(self.engine), "--version"], capture_output=True, text=True,
            timeout=20, env=self.engine_env(),
        )
        self.assertEqual(version.returncode, 0)
        self.assertIn("macrostat-engine", version.stdout)

        broken = self.root / "broken.json"
        broken.write_text("{not json", encoding="utf-8")
        bad = subprocess.run(
            [str(self.engine), str(broken)], capture_output=True, text=True,
            timeout=20, env=self.engine_env(),
        )
        self.assertEqual(bad.returncode, 2)

    def test_ipc_status_ping_reload_stop(self):
        process = self.stop_engine()
        info = self.wait_for_runtime()
        self.assertEqual(info["pid"], process.pid)

        status = self.ipc("status")
        self.assertEqual(status.returncode, 0, status.stderr)
        reply = json.loads(status.stdout)
        self.assertTrue(reply["ok"])
        self.assertEqual(reply["pid"], process.pid)

        ping = self.ipc("ping")
        self.assertEqual(ping.returncode, 0, ping.stderr)
        self.assertTrue(json.loads(ping.stdout)["ok"])

        self.profile.write_text(
            json.dumps(profile_document(name="e2e-reloaded")), encoding="utf-8"
        )
        reload = self.ipc("reload")
        self.assertEqual(reload.returncode, 0, reload.stderr)
        self.assertTrue(
            self.wait_for(lambda: "Profil yeniden yüklendi" in self.read_log()),
            "reload günlüğe yansımadı",
        )

        stop = self.ipc("stop")
        self.assertEqual(stop.returncode, 0, stop.stderr)
        self.assertEqual(process.wait(timeout=15), 0)
        self.assertFalse(self.runtime_path().exists(), "runtime.json silinmedi")
        self.assertIn("Motor durdu", self.read_log())

    def test_second_instance_is_rejected(self):
        first = self.stop_engine()
        second = self.spawn(self.profile, "--log", self.root / "second.log")
        self.assertEqual(second.wait(timeout=15), 7)
        stderr = second.stderr.read()
        self.assertIn("zaten çalışıyor", stderr)
        self.assertIsNone(first.poll(), "ilk motor hâlâ çalışıyor olmalı")
        self.assertEqual(self.ipc("stop").returncode, 0)
        self.assertEqual(first.wait(timeout=15), 0)

    @unittest.skipIf(
        os.environ.get("MACROSTAT_E2E_SKIP_INPUT") == "1",
        "girdi enjeksiyonu bu oturumda desteklenmiyor (MACROSTAT_E2E_SKIP_INPUT=1)",
    )
    def test_panic_key_stops_engine(self):
        process = self.stop_engine()
        self.wait_for_runtime()
        self.inject_key(VK_F24)
        self.assertEqual(process.wait(timeout=15), 0)
        self.assertIn("Acil durdurma tetiklendi", self.read_log())
        self.assertFalse(self.runtime_path().exists())

    @unittest.skipIf(
        os.environ.get("MACROSTAT_E2E_SKIP_INPUT") == "1",
        "girdi enjeksiyonu bu oturumda desteklenmiyor (MACROSTAT_E2E_SKIP_INPUT=1)",
    )
    def test_self_test_mode(self):
        result = subprocess.run(
            [str(self.engine), "--self-test"],
            capture_output=True, text=True, timeout=60, env=self.engine_env(),
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("kanca kurulumu", result.stdout)
        self.assertIn("panik tuşu mandalı", result.stdout)
        self.assertNotIn("HATA (", result.stdout)

    def test_test_macro_runs_delay_only_macro(self):
        result = subprocess.run(
            [str(self.engine), str(self.profile), "--test-macro", "e2e-delay"],
            capture_output=True, text=True, timeout=30, env=self.engine_env(),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("[TEST]", result.stdout)

    @unittest.skipIf(
        os.environ.get("MACROSTAT_E2E_SKIP_INPUT") == "1",
        "girdi enjeksiyonu bu oturumda desteklenmiyor (MACROSTAT_E2E_SKIP_INPUT=1)",
    )
    def test_record_input_roundtrip(self):
        output = self.root / "recording.json"
        recorder = self.spawn("--record-input", output, "--panic-key", "KEY_F24")
        time.sleep(1.5)
        self.user32.mouse_event(MOUSEEVENTF_MOVE, 20, 20, 0, 0)
        self.user32.mouse_event(MOUSEEVENTF_MOVE, -20, -20, 0, 0)
        self.inject_key(VK_A)
        time.sleep(0.2)
        self.inject_key(VK_F24)
        self.assertEqual(recorder.wait(timeout=20), 0, recorder.stderr.read())

        recording = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(recording["version"], 1)
        self.assertEqual(recording["sample_ms"], 4)
        names = [entry[1] for entry in recording.get("keys", [])]
        self.assertIn("KEY_A", names)
        self.assertTrue(
            any(entry[2] == 1 for entry in recording.get("keys", [])),
            "KEY_A basışı kaydedilmedi",
        )
        self.assertNotIn("BTN_", names)


if __name__ == "__main__":
    unittest.main(verbosity=2)
