# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Windows ilk kurulum sihirbazı: ortam kontrolü, Windows notları ve motor onayı.

Linux sürümünden farkı: udev/`uaccess` adımı yoktur ve sihirbaz hiçbir
noktada yönetici izni istemez. Motor normal (``asInvoker``) yetkiyle çalışır.
"""

import json
import tempfile
from pathlib import Path

from PyQt6.QtWidgets import (
    QCheckBox, QHBoxLayout, QLabel, QPushButton, QVBoxLayout,
    QWizard, QWizardPage,
)

from . import common, theme, winplatform
from .common import T


def daemon_available():
    """Motor ikilisi bulundu mu?"""
    binary = getattr(common, "DAEMON_BIN", None) or common.find_engine()
    return bool(binary and Path(binary).is_file())


def data_dir_writable():
    """Veri klasörü (profil, runtime.json, günlük) yazılabilir mi?"""
    try:
        Path(common.RUNTIME_DIR).mkdir(parents=True, exist_ok=True)
        handle = tempfile.NamedTemporaryFile(
            dir=str(common.RUNTIME_DIR), prefix=".onboarding-", delete=True)
        handle.write(b"ok")
        handle.close()
        return True
    except OSError:
        return False


def profile_readable():
    """Profil dosyası okunabiliyor ve sözleşmeye uygun JSON mu?"""
    raw = common.read_private_text_file(common.PROFILE_PATH)
    if raw is None:
        return False
    try:
        data = json.loads(raw)
    except (ValueError, TypeError):
        return False
    return isinstance(data, dict) and (
        isinstance(data.get("profiles"), list) or bool(data.get("macros")))


def job_object_available():
    """Job Object (``KILL_ON_JOB_CLOSE``) kullanılabiliyor mu?

    Kullanılamazsa motor yine ``--parent-pid`` ile GUI ömrüne bağlanır; arayüz
    çökerse motoru kapatmak için çekirdek güvencesi yerine yoklama kullanılır.
    """
    if not winplatform.is_windows():
        return True
    job = winplatform.JobObject()
    try:
        return job.available
    finally:
        job.close()


def engine_image_forbidden():
    """Motor ikilisi sembolik bağlantı/junction ile yönlendirilmiş mi?"""
    binary = getattr(common, "DAEMON_BIN", None) or common.find_engine()
    if not binary:
        return False
    return winplatform.file_has_reparse_point(Path(binary))


def collect_checks():
    """Sihirbaz ve özet için (anahtar, başarılı, başlık, açıklama) listesi."""
    elevated = winplatform.is_elevated()
    return [
        ("engine", daemon_available(),
         T("Motor ikilisi bulundu"),
         T("Yanında 'macrostat-engine.exe' olmalı ya da 'cargo build --release' "
           "ile derlenmiş olmalıdır.")),
        ("datadir", data_dir_writable(),
         T("Veri klasörü yazılabilir"),
         T("Profil, runtime.json ve günlük %LOCALAPPDATA%\\MacroStat altında "
           "tutulur; başka kullanıcılar okuyamaz.")),
        ("profile", profile_readable(),
         T("Profil dosyası geçerli"),
         T("profile.json sözleşmesi (schemaVersion 1) okunabilir durumda.")),
        ("job", job_object_available(),
         T("Süreç koruması (Job Object)"),
         T("Arayüz kapanırsa motorun da kapanmasını çekirdek düzeyinde "
           "garantiler.")),
        ("privilege", not engine_image_forbidden(),
         T("Normal kullanıcı yetkisi yeterli"),
         T("Yönetici izni gerekmez.{}").format(
             T(" Bu arayüz şu an yönetici olarak çalışıyor; normal açmanız "
               "önerilir.") if elevated else "")),
    ]


class StatusPage(QWizardPage):
    """Adım 1: ortam kontrolü; her satır ✓/✗ ve tek cümle açıklama."""

    def __init__(self):
        super().__init__()
        self.setTitle(T("Sistem Durumu"))
        self.setSubTitle(T("Kuruluma başlamadan önce ortamınızı kontrol edelim."))
        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        self._rows = {}
        for key, _ok, title, description in collect_checks():
            row = QHBoxLayout()
            row.setSpacing(10)
            mark = QLabel("…")
            mark.setObjectName("onboardingStatus")
            mark.setFixedWidth(18)
            row.addWidget(mark, 0)
            text = QVBoxLayout()
            text.setSpacing(0)
            title_label = QLabel(title)
            title_label.setObjectName("onboardingTitle")
            detail = QLabel(description)
            detail.setObjectName("onboardingDetail")
            detail.setWordWrap(True)
            text.addWidget(title_label)
            text.addWidget(detail)
            row.addLayout(text, 1)
            layout.addLayout(row)
            self._rows[key] = (mark, title_label)

        layout.addStretch()
        row = QHBoxLayout()
        refresh = QPushButton(T("Durumu Yenile"))
        refresh.clicked.connect(self.refresh)
        row.addWidget(refresh)
        row.addStretch()
        layout.addLayout(row)
        self.refresh()

    def refresh(self):
        for key, ok, _title, _description in collect_checks():
            mark, _title_label = self._rows[key]
            mark.setText("✓" if ok else "✗")
            theme.set_state(mark, "ok" if ok else "error")

    def initializePage(self):
        self.refresh()

    def isComplete(self):
        return True


class WindowsNotesPage(QWizardPage):
    """Adım 2: Windows'a özgü davranışlar ve sınırlar (bilgi amaçlı)."""

    def __init__(self):
        super().__init__()
        self.setTitle(T("Windows Notları"))
        self.setSubTitle(T(
            "Kurulum yönetici izni istemez; aşağıdaki sınırlar Windows'un "
            "güvenlik modelinden kaynaklanır."))
        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        notes = (
            T("UIPI: normal yetkili arayüz, yönetici olarak çalışan "
              "pencerelere tuş/fare enjekte edemez. Hedef uygulama yönetici "
              "ise MacroStat'ı da yönetici olarak açın (isteğe bağlı)."),
            T("Güvenli masaüstü: UAC istemi ve oturum kilitliyken kayıt "
              "durur; bu bir hata değildir."),
            T("SmartScreen/antivirüs: imzasız bir kurulum ilk açılışta "
              "uyarabilir; bu sürümde imzalama kapsam dışıdır."),
            T("Acil durdurma her zaman '{}' tuşudur; kayıt ve makro çalışması "
              "bu tuşla güvenle biter.").format(common.PANIC_KEY),
        )
        for note in notes:
            label = QLabel("• " + note)
            label.setObjectName("onboardingDetail")
            label.setWordWrap(True)
            layout.addWidget(label)

        layout.addStretch()

    def isComplete(self):
        return True


class ReadyPage(QWizardPage):
    """Adım 3: özet ve motoru başlatma onayı."""

    def __init__(self):
        super().__init__()
        self.setTitle(T("Hazır"))
        self.setSubTitle(T("Kurulum tamamlandı. Motoru şimdi başlatabilirsiniz."))
        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        self.summary = QLabel("")
        self.summary.setObjectName("onboardingResult")
        self.summary.setWordWrap(True)
        layout.addWidget(self.summary)

        self.chk_start = QCheckBox(T("Motoru şimdi başlat"))
        self.chk_start.setChecked(True)
        layout.addWidget(self.chk_start)
        layout.addStretch()

    def initializePage(self):
        checks = collect_checks()
        missing = [title for _key, ok, title, _d in checks if not ok]
        if missing:
            self.summary.setText(
                T("Eksik görünen adımlar: {}").format(", ".join(missing)))
            theme.set_state(self.summary, "info")
        else:
            self.summary.setText(T("Tüm kontroller başarılı."))
            theme.set_state(self.summary, "ok")

    def start_engine_requested(self):
        return self.chk_start.isChecked()


class OnboardingWizard(QWizard):
    """Üç adımlı ilk kurulum sihirbazı (yönetici izni istemez)."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle(T("MacroStat Kurulum"))
        self.resize(640, 480)
        self.setWizardStyle(QWizard.WizardStyle.ModernStyle)
        self.setOption(QWizard.WizardOption.NoBackButtonOnStartPage, True)

        self.status_page = StatusPage()
        self.notes_page = WindowsNotesPage()
        self.ready_page = ReadyPage()
        self.addPage(self.status_page)
        self.addPage(self.notes_page)
        self.addPage(self.ready_page)

        self.setButtonText(QWizard.WizardButton.BackButton, T("Geri"))
        self.setButtonText(QWizard.WizardButton.NextButton, T("İleri"))
        self.setButtonText(QWizard.WizardButton.FinishButton, T("Bitir"))
        self.setButtonText(QWizard.WizardButton.CancelButton, T("İptal"))

    def accept(self):
        data = common.load_settings()
        data["onboarding_completed"] = True
        common.save_settings(data)
        super().accept()

    def start_engine_requested(self):
        return self.ready_page.start_engine_requested()
