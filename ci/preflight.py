#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Windows CI ön kontrolü — çevrimdışı.

`ci/windows.yml` (kaynak), `.github/workflows/windows.yml` (etkin kopya) ve
`windows/` ağacı arasındaki sözleşmeyi denetler:

* iş akışının beklediği betikler, spec, kurulum tanımı ve test dizini var mı;
* sürüm tek kaynaktan mı geliyor (`VERSION` = `Cargo.toml` = `version_info.txt`);
* etkin iş akışı ile `ci/windows.yml` **birebir** aynı mı;
* YAML yapısal olarak ayrıştırılıyor mu; gerekli işler, `needs` ilişkileri ve
  `sign` koşulu beklenen biçimde mi;
* `audit` yalnızca pull request'te bilgilendirici, diğer koşularda engelleyici mi;
* `push` / `pull_request` yol filtreleri, `v*` etiketi ve haftalık `schedule`
  tanımlı mı;
* iş akışındaki her `uses:` satırı 40 karakterli küçük harf commit SHA'sına
  sabit mi;
* yasaklı ağ/telemetri adımları (kuşkulu `curl`/`Invoke-WebRequest`) yok mu.

Kullanım (depo kökünden):

    python -m pip install -r ci/requirements.txt
    python ci/preflight.py

Çıkış kodu 0 = hazır, 1 = eksik/uyumsuzluk. Hiçbir dosyayı değiştirmez ve
ağa çıkmaz.
"""

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - CI'da ci/requirements.txt kurulur
    yaml = None

ROOT = Path(__file__).resolve().parents[1]
WINDOWS = ROOT / "windows"
CI = ROOT / "ci"
WORKFLOW = CI / "windows.yml"
ACTIVE_WORKFLOW = ROOT / ".github" / "workflows" / "windows.yml"

REQUIRED_FILES = [
    WINDOWS / "VERSION",
    WINDOWS / "LICENSE",
    WINDOWS / "THIRD_PARTY.md",
    WINDOWS / "engine" / "Cargo.toml",
    WINDOWS / "engine" / "Cargo.lock",
    WINDOWS / "gui" / "MacroStat.spec",
    WINDOWS / "gui" / "requirements-win.txt",
    WINDOWS / "gui" / "requirements-build.txt",
    WINDOWS / "gui" / "version_info.txt",
    WINDOWS / "gui" / "main.py",
    WINDOWS / "installer" / "MacroStat.iss",
    WINDOWS / "scripts" / "build.ps1",
    WINDOWS / "scripts" / "sign.ps1",
    WINDOWS / "scripts" / "verify.ps1",
    WINDOWS / "tests" / "test_gui_windows.py",
    WINDOWS / "tests" / "test_e2e_windows.py",
    WINDOWS / "docs" / "profile.schema.json",
    WINDOWS / "docs" / "KEY-DICTIONARY.tsv",
    CI / "requirements.txt",
]

REQUIRED_JOBS = ["core", "gui-tests", "build", "e2e", "audit", "sign"]

REQUIRED_NEEDS = {
    "build": {"core", "gui-tests"},
    "e2e": {"build"},
    "sign": {"build"},
}

REQUIRED_PATHS = {"windows/**", "ci/**", ".github/workflows/**"}


def fail(problems, message):
    problems.append(message)


def check_files(problems):
    for path in REQUIRED_FILES:
        if not path.is_file():
            fail(problems, f"eksik dosya: {path.relative_to(ROOT)}")


def check_versions(problems):
    version_path = WINDOWS / "VERSION"
    cargo_path = WINDOWS / "engine" / "Cargo.toml"
    info_path = WINDOWS / "gui" / "version_info.txt"
    iss_path = WINDOWS / "installer" / "MacroStat.iss"
    if not all(p.is_file() for p in (version_path, cargo_path, info_path, iss_path)):
        return  # eksik dosya check_files'ta zaten raporlandı

    version = version_path.read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        fail(problems, f"VERSION geçersiz: {version!r}")
        return

    cargo = cargo_path.read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', cargo)
    if not match or match.group(1) != version:
        fail(problems, f"Cargo.toml sürümü VERSION ile uyuşmuyor: {match and match.group(1)}")

    info = info_path.read_text(encoding="utf-8")
    if f"'{version}'" not in info and f"({version})" not in info:
        fail(problems, f"version_info.txt {version} sürümünü içermiyor")

    iss = iss_path.read_text(encoding="utf-8")
    if f'MyAppVersion "{version}"' not in iss:
        fail(problems, f"MacroStat.iss varsayılan sürümü {version} değil")


def check_workflow_copies(problems):
    """Etkin GitHub Actions dosyası, kaynak kopyayla birebir aynı olmalı."""
    if not ACTIVE_WORKFLOW.is_file():
        fail(
            problems,
            "aktif iş akışı yok: .github/workflows/windows.yml "
            "(ci/windows.yml'in birebir kopyası olmalı)",
        )
        return
    if not WORKFLOW.is_file():
        return  # eksik kaynak check_files/load sırasında raporlanır
    if WORKFLOW.read_bytes() != ACTIVE_WORKFLOW.read_bytes():
        fail(
            problems,
            "ci/windows.yml ile .github/workflows/windows.yml birebir aynı değil; "
            "kopyayı yenileyin (cp ci/windows.yml .github/workflows/windows.yml)",
        )


def load_workflow(problems):
    """(ayrıştırılmış veri | None, ham metin | None) döndürür."""
    if not WORKFLOW.is_file():
        return None, None
    text = WORKFLOW.read_text(encoding="utf-8")
    if yaml is None:
        fail(
            problems,
            "PyYAML bulunamadı: python -m pip install -r ci/requirements.txt",
        )
        return None, text
    try:
        data = yaml.safe_load(text)
    except yaml.YAMLError as exc:
        fail(problems, f"ci/windows.yml ayrıştırılamadı: {exc}")
        return None, text
    if not isinstance(data, dict):
        fail(problems, "ci/windows.yml kökü bir eşleme (mapping) değil")
        return None, text
    return data, text


def check_action_pins(problems, text):
    for match in re.finditer(r"uses:\s*([^\s#]+)", text):
        spec = match.group(1)
        if "@" not in spec:
            fail(problems, f"sürümsüz action: {spec}")
            continue
        ref = spec.rsplit("@", 1)[1]
        if not re.fullmatch(r"[0-9a-f]{40}", ref):
            fail(problems, f"action 40 karakterli SHA'ya sabitlenmemiş: {spec}")


def check_forbidden_commands(problems, text):
    for forbidden in ("Invoke-WebRequest", "curl ", "wget ", "Start-BitsTransfer"):
        if forbidden in text:
            fail(problems, f"iş akışında beklenmeyen ağ komutu: {forbidden.strip()}")


def check_jobs(problems, jobs):
    for job in REQUIRED_JOBS:
        if job not in jobs:
            fail(problems, f"iş akışında iş yok: {job}")

    for job_id, expected in REQUIRED_NEEDS.items():
        if job_id not in jobs:
            continue
        raw = jobs[job_id].get("needs", [])
        actual = {raw} if isinstance(raw, str) else set(raw or [])
        if actual != expected:
            fail(
                problems,
                f"{job_id} işinin needs ilişkisi yanlış: "
                f"beklenen {sorted(expected)}, bulunan {sorted(actual)}",
            )

    sign = jobs.get("sign") or {}
    condition = str(sign.get("if") or "")
    if "SIGNING_ENABLED" not in condition:
        fail(problems, "sign işi SIGNING_ENABLED ile koşullu değil")
    if "github.event_name" not in condition:
        fail(problems, "sign işi olay türünü koşulda sınırlamıyor")
    if "pull_request" in condition:
        fail(problems, "sign koşulu pull_request olayını dışlamıyor")
    if "'push'" not in condition and '"push"' not in condition:
        fail(problems, "sign işi yalnızca push/workflow_dispatch ile sınırlı değil")
    if sign.get("environment") != "signing":
        fail(problems, "sign işi korumalı 'signing' ortamına bağlı değil")

    audit = jobs.get("audit") or {}
    suppression = str(audit.get("continue-on-error", ""))
    if "pull_request" not in suppression:
        fail(
            problems,
            "audit işi yalnızca pull_request'te bilgilendirici değil; "
            "diğer koşularda başarısızlık engelleyici olmalı",
        )


def check_triggers(problems, data):
    triggers = data.get("on")
    if triggers is None:
        # PyYAML, YAML 1.1 gereği düz `on` anahtarını True olarak ayrıştırır.
        triggers = data.get(True)
    if not isinstance(triggers, dict):
        fail(problems, "iş akışında geçerli bir `on:` tetikleyici bloğu yok")
        return

    if "workflow_dispatch" not in triggers:
        fail(problems, "workflow_dispatch tetikleyicisi yok")

    for event in ("push", "pull_request"):
        block = triggers.get(event) or {}
        if not isinstance(block, dict):
            fail(problems, f"{event} tetikleyicisi eşleme değil")
            continue
        paths = set(block.get("paths") or [])
        missing = REQUIRED_PATHS - paths
        if missing:
            fail(
                problems,
                f"{event} yol filtresinde eksik: {', '.join(sorted(missing))}",
            )
        # Var olan dal filtreleri korunmalı; en az bir dal deseni beklenir.
        if not block.get("branches"):
            fail(problems, f"{event} dal filtresi tanımlı değil")

    push = triggers.get("push") or {}
    if "v*" not in (push.get("tags") or []):
        fail(problems, "push tetikleyicisinde `v*` sürüm etiketi filtresi yok")

    schedule = triggers.get("schedule")
    if not isinstance(schedule, list) or not schedule:
        fail(problems, "haftalık schedule tetikleyicisi yok")
    elif not any(isinstance(item, dict) and item.get("cron") for item in schedule):
        fail(problems, "schedule tetikleyicisinde cron ifadesi yok")


def check_workflow(problems, data, text):
    check_action_pins(problems, text)
    check_forbidden_commands(problems, text)
    if data is None:
        return

    if "--deny warnings" not in text:
        fail(problems, "cargo audit `--deny warnings` ile çalışmıyor")
    if "--strict" not in text:
        fail(problems, "pip-audit `--strict` ile çalışmıyor")

    jobs = data.get("jobs")
    if not isinstance(jobs, dict):
        fail(problems, "iş akışında `jobs:` eşlemesi yok")
        return
    check_jobs(problems, jobs)
    check_triggers(problems, data)


def _use_utf8_stdio():
    """Use UTF-8 for redirected output on Windows.

    CI captures stdout with a legacy code page (cp1252) where Turkish
    characters raise `UnicodeEncodeError`; Actions logs themselves are UTF-8.
    """
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass


def main():
    _use_utf8_stdio()
    problems = []
    check_files(problems)
    check_versions(problems)
    check_workflow_copies(problems)
    data, text = load_workflow(problems)
    if text is not None:
        check_workflow(problems, data, text)

    if problems:
        print("CI ön kontrolü BAŞARISIZ:")
        for problem in problems:
            print(f" - {problem}")
        return 1
    print("CI on kontrolu gecti: dosyalar, surumler, is akisi kopyasi, YAML grafigi ve SHA sabitlemeleri tutarli.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
