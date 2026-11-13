# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""Sistem sekmesi, motor süreç denetimi ve yedekleme (Windows).

Linux sürümünden farkları:

* udev/uinput yok. Yerine motor ikilisinin varlığı, `runtime.json` ve
  kanal (`--ipc`) durumu denetlenir; "İzinleri Düzelt" düğmesi motoru
  yeniden bulup başlatır.
* Motor, GUI'nin alt süreci olarak `--parent-pid` ile başlatılır ve bir
  Job Object'e atanır (`KILL_ON_JOB_CLOSE`). GUI çökerse çekirdek motoru
  sonlandırır; zarif yol `--parent-pid` izleyicisidir.
* Yenileme `SIGUSR1` yerine kanal `reload` komutuyla yapılır; STOP sonrası
  tuş dinleyici `--watch-restart` çıkış kodu 42'yi kullanır.
"""

import os
import subprocess
import sys
import time
from pathlib import Path

from . import common, theme, transfer, winplatform
from .common import *  # noqa: F401,F403
from .widgets import SlidingSwitch, KeyCaptureDialog  # noqa: F401

#: Panik tuşuyla durdurulan motorun günlüğe bıraktığı imza.
PANIC_LOG_MARKER = "Acil durdurma tetiklendi"

#: `subprocess`/QProcess için konsol penceresini gizleyen bayrak.
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)


def _resolved_profile_path(path):
    """Profil yolunu çözer; çözümleme hatasında özgün yolu korur."""
    try:
        return path.resolve()
    except (OSError, RuntimeError):
        return path


def hide_console(proc):
    """QProcess'in konsol penceresi açmasını engeller (Windows'a özgü)."""
    try:
        if not winplatform.is_windows() or not hasattr(
                proc, "setCreateProcessArgumentsModifier"):
            return
        def modifier(arguments):
            arguments.flags |= CREATE_NO_WINDOW
        proc.setCreateProcessArgumentsModifier(modifier)
    except Exception:
        pass


class SystemMixin:
    """Sistem sekmesi, motor süreç denetimi ve yedekleme."""

    def setup_system_tab(self):
        layout = QVBoxLayout(self.tab_system)
        self.sys_info = QTextEdit()
        self.sys_info.setReadOnly(True)
        layout.addWidget(self.sys_info)

        p_box = QHBoxLayout()
        btn_fix = QPushButton(T("Motoru Bul ve Başlat"))
        theme.set_variant(btn_fix, "primary")
        btn_fix.clicked.connect(self.fix_permissions)
        p_box.addWidget(btn_fix)

        btn_ref = QPushButton(T("Durumu Yenile"))
        btn_ref.clicked.connect(self.refresh_system_status)
        p_box.addWidget(btn_ref)
        p_box.addStretch()
        layout.addLayout(p_box)

        b_box = QHBoxLayout()
        btn_backup = QPushButton(T("Yedekle…"))
        btn_backup.clicked.connect(self.backup_all_profiles)
        b_box.addWidget(btn_backup)

        btn_restore = QPushButton(T("Geri Yükle…"))
        btn_restore.clicked.connect(self.restore_profiles_from_file)
        b_box.addWidget(btn_restore)

        btn_onboarding = QPushButton(T("Kurulumu yeniden çalıştır"))
        btn_onboarding.clicked.connect(self.run_onboarding_wizard)
        b_box.addWidget(btn_onboarding)
        b_box.addStretch()
        layout.addLayout(b_box)
        self.refresh_system_status()

    # -- Yedekleme ---------------------------------------------------------

    def backup_all_profiles(self):
        if not self.all_profiles:
            QMessageBox.warning(self, T("Yedekleme"),
                                T("Yedeklenecek profil bulunamadı."))
            return
        default_name = "macrostat-yedek-{}.json".format(time.strftime("%Y%m%d-%H%M"))
        path, _ = QFileDialog.getSaveFileName(
            self, T("Profilleri Yedekle"),
            str(Path.home() / default_name), "JSON (*.json)")
        if not path:
            return
        try:
            transfer.export_profiles(self.all_profiles, path, self.active_profile_id)
            self.show_autosave_banner(
                T("{} profil yedeklendi: {}").format(len(self.all_profiles), path))
        except transfer.TransferError as exc:
            QMessageBox.critical(self, T("Yedekleme Hatası"), str(exc))

    def restore_profiles_from_file(self):
        path, _ = QFileDialog.getOpenFileName(
            self, T("Yedeği Geri Yükle"), str(Path.home()), "JSON (*.json)")
        if not path:
            return
        try:
            data = transfer.load_payload(path)
            existing_macros = {
                m.get("id") for p in self.all_profiles
                for m in p.get("macros", []) if isinstance(m, dict) and m.get("id")
            }
            existing_profiles = {p.get("id") for p in self.all_profiles if p.get("id")}
            result = transfer.import_payload(data, existing_macros, existing_profiles)
        except transfer.TransferError as exc:
            QMessageBox.critical(self, T("Yedek Geri Yükleme"), str(exc))
            return

        if not result["profiles"]:
            QMessageBox.warning(
                self, T("Yedek Geri Yükleme"),
                T("Bu dosyada geri yüklenecek profil yok. Makroları "
                  "Makrolar sekmesindeki 'İçe Aktar…' ile ekleyin."))
            return

        answer = QMessageBox.question(
            self, T("Yedek Geri Yükleme"),
            T("{} profil geri yüklenecek. Aynı adlı profiller '(2)', '(3)' "
              "ekleriyle eklenir; mevcut profiller silinmez. Devam edilsin mi?").format(
                len(result["profiles"])),
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No)
        if answer != QMessageBox.StandardButton.Yes:
            return

        try:
            used_names = {p.get("name") for p in self.all_profiles if p.get("name")}
            for profile in result["profiles"]:
                profile["name"] = transfer.unique_name(profile.get("name"), used_names)
                used_names.add(profile["name"])
                self.all_profiles.append(profile)
            self.populate_profile_combo()
            self.save_profile_to_disk(restart_daemon=True)
            self.refresh_macro_table()
            self.show_autosave_banner(
                T("{} profil geri yüklendi.").format(len(result["profiles"])))
        except Exception as exc:
            QMessageBox.critical(self, T("Yedek Geri Yükleme"), str(exc))

    # -- Tanılama ----------------------------------------------------------

    def _engine_status_text(self):
        info = common.read_runtime_info()
        if not info:
            return T("Kapalı (runtime.json yok)")
        pid = info["pid"]
        if not common.engine_running():
            return T("Kapalı (PID {} artık yaşamıyor)").format(pid)
        return T("Çalışıyor — PID {}, kanal {}").format(pid, info["pipeName"])

    def _log_size_text(self):
        try:
            size = LOG_FILE.stat().st_size
        except OSError:
            return T("Yok")
        return "{} ({} bayt)".format(LOG_FILE, size)

    def refresh_system_status(self):
        engine = common.find_engine()
        elevated = winplatform.is_elevated()
        guide_text = T(
            "=== MacroStat Windows Tanılama ===\n"
            "Motor İkili Dosyası: {}\n"
            "Motor Durumu: {}\n"
            "Veri Klasörü: {}\n"
            "Profil Dosyası: {}\n"
            "Acil Durdurma Tuşu (STOP): {}\n"
            "Günlük Dosyası: {}\n"
            "Yönetici Olarak: {}\n"
            "Job Object: {}\n"
            "DPI Farkındalığı: {}\n"
        ).format(
            str(engine) if engine else T(
                "Bulunamadı (windows/engine içinde 'cargo build --release --locked')"),
            self._engine_status_text(),
            str(common.DATA_DIR),
            str(_resolved_profile_path(common.PROFILE_PATH)),
            common.PANIC_KEY,
            self._log_size_text(),
            T("Evet") if elevated else T("Hayır"),
            T("Kullanılabilir") if getattr(self, "job_object", None)
            and self.job_object.available else T("Kullanılamıyor"),
            T("Etkin") if winplatform.is_windows() else T("Windows dışı (test)"),
        )
        if not elevated:
            guide_text += "\n" + T(
                "Not: Yükseltilmiş (yönetici) pencereler UIPI nedeniyle girdi "
                "almayabilir; güvenli masaüstünde (UAC/kilit ekranı) kanca "
                "sessiz kalır. MacroStat'ı yönetici olarak çalıştırmak isteğe "
                "bağlıdır ve güvenlik ödünleşimi taşır.")
        self.sys_info.setPlainText(guide_text)

    def fix_permissions(self):
        """Motoru yeniden bulur ve (gerekirse) başlatır.

        Linux'taki udev onarımının Windows karşılığıdır: burada ayrıcalık
        istenmez; yalnızca ikili bulunur ve normal kullanıcı süreci başlatılır.
        """
        common.DAEMON_BIN, common.PROFILE_PATH = common.locate_paths()
        if common.DAEMON_BIN is None:
            self.refresh_system_status()
            QMessageBox.warning(
                self, T("Motor Bulunamadı"),
                T("macrostat-engine.exe bulunamadı.\n\n"
                  "Geliştirme derlemesi için windows/engine dizininde şunu "
                  "çalıştırın:\n  cargo build --release --locked\n\n"
                  "Kurulum paketinde motor uygulama klasöründe bulunur."))
            return
        try:
            self.restart_daemon(show_message=False, force=True)
            self.refresh_system_status()
            if self.is_daemon_running():
                QMessageBox.information(
                    self, T("Motor Hazır"),
                    T("Makro motoru başlatıldı.\n{}").format(common.DAEMON_BIN))
            else:
                QMessageBox.warning(
                    self, T("Motor Başlatılamadı"),
                    T("Motor bulundu ancak başlamadı. Ayrıntı için günlüğü "
                      "inceleyin:\n{}").format(LOG_FILE))
        except Exception as exc:
            QMessageBox.critical(self, T("Hata"), str(exc))

    # -- Motor durumu ------------------------------------------------------

    def _daemon_stopped_by_panic(self):
        """Bu motor çalışması sırasında günlüğe yazılmış STOP kaydı var mı?"""
        try:
            offset = getattr(self, "_daemon_log_offset", 0)
            if not LOG_FILE.exists() or LOG_FILE.stat().st_size <= offset:
                return False
            with open(LOG_FILE, "r", encoding="utf-8", errors="ignore") as handle:
                handle.seek(offset)
                return PANIC_LOG_MARKER in handle.read()
        except Exception:
            return False

    def is_daemon_running(self):
        if self.daemon_proc is not None and self.daemon_proc.poll() is None:
            return True
        return common.engine_running()

    def check_and_auto_start_daemon(self):
        if getattr(self, "is_manually_stopped", False) or getattr(
                self, "was_editor_daemon_running", False):
            return
        if not self.is_daemon_running():
            self.restart_daemon(show_message=False)

    def update_daemon_status(self):
        if getattr(self, "is_manually_stopped", False):
            self.lbl_stopped.show()
            self.status_badge.setText(T("Motor Durduruldu (STOP)"))
            theme.set_state(self.status_badge, "error")
            self.btn_restart_after_stop.setEnabled(True)
            self.start_restart_watch()
            return
        self.btn_restart_after_stop.setEnabled(False)
        if getattr(self, "was_editor_daemon_running", False):
            self.status_badge.setText(T("Düzenleme Modu (Beklemede)"))
            theme.set_state(self.status_badge, "info")
            return
        if self.is_daemon_running():
            self.failed_status_checks = 0
            self.status_badge.setText(T("Aktif"))
            theme.set_state(self.status_badge, "ok")
            if hasattr(self, "act_view_log"):
                self.act_view_log.setText(T("Günlüğü İncele"))
                self.act_view_log.setIcon(theme.icon("document-preview"))
            if hasattr(self, "btn_quick_fix"):
                self.btn_quick_fix.hide()
        else:
            # Fiziksel STOP tuşu motoru kapatır; bu durum oturum boyunca korunur.
            if self._daemon_stopped_by_panic():
                self.is_manually_stopped = True
                self.lbl_stopped.show()
                self.status_badge.setText(T("Motor Durduruldu (STOP)"))
                theme.set_state(self.status_badge, "error")
                self.btn_restart_after_stop.setEnabled(True)
                self.start_restart_watch()
                return
            self.failed_status_checks += 1
            if self.failed_status_checks >= 2:
                exit_code = (self.daemon_proc.poll()
                             if self.daemon_proc is not None else None)
                if exit_code not in (None, 0):
                    self.status_badge.setText(
                        T("Motor Çöktü (kod {})").format(exit_code))
                else:
                    self.status_badge.setText(T("Motor Durduruldu"))
                theme.set_state(self.status_badge, "error")
                if hasattr(self, "act_view_log"):
                    self.act_view_log.setText(T("Hatayı İncele"))
                    self.act_view_log.setIcon(theme.icon("dialog-error"))
                if hasattr(self, "btn_quick_fix"):
                    self.btn_quick_fix.show()

    def stop_daemon(self):
        """Motoru zarifçe durdurur; yanıt yoksa kimliği doğrulanmış süreci sonlandırır."""
        try:
            if self.daemon_proc is not None and self.daemon_proc.poll() is None:
                common.stop_engine(timeout=2.5)
                try:
                    self.daemon_proc.wait(timeout=1.0)
                except Exception:
                    pass
            else:
                common.stop_engine(timeout=2.5)
            self.daemon_proc = None
            common.DAEMON_PROC = None
            self.update_daemon_status()
        except Exception:
            pass

    def request_daemon_reload(self):
        """Profili motoru durdurmadan tazeler (kanal `reload`)."""
        if not self.is_daemon_running():
            return False
        ok, _payload = common.ipc_command("reload")
        return ok

    def apply_profile_to_daemon(self):
        if getattr(self, "is_manually_stopped", False):
            return
        # Düzenleme/kayıt modunda motor bilinçli olarak kapalıdır; çıkışta açılır.
        if getattr(self, "was_editor_daemon_running", False) or \
                getattr(self, "was_recording_daemon_running", False):
            return
        if self.is_daemon_running() and self.request_daemon_reload():
            self.show_autosave_banner(
                T("Makro motoru yeniden başlatılmadan güncellendi."))
            return
        self.restart_daemon(show_message=False)

    # -- Motor başlatma ----------------------------------------------------

    def _ensure_job_object(self):
        """Motoru GUI ömrüne bağlayan Job Object'i bir kez oluşturur."""
        job = getattr(self, "job_object", None)
        if job is None:
            job = winplatform.JobObject()
            self.job_object = job
        return job

    def _engine_command(self):
        engine = common.find_engine()
        if engine is None:
            return None
        command = [str(engine)]
        profile = _resolved_profile_path(common.PROFILE_PATH)
        if profile.exists():
            command.append(str(profile))
        command += ["--parent-pid", str(os.getpid())]
        return command

    def _cargo_candidates(self):
        """Cargo için sabit adaylar (PATH araması yapılmaz).

        Paketlenmiş uygulamada derleme denemesi hiç yapılmaz; geliştirme
        ortamında yalnızca bilinen kurulum konumlarına bakılır. PATH ele
        geçirilirse rastgele bir çalıştırılabilir motor sanılıp
        derlenebilirdi.
        """
        if getattr(sys, "frozen", False):
            return []
        candidates = []
        cargo_home = os.environ.get("CARGO_HOME")
        if cargo_home and os.path.isabs(cargo_home):
            candidates.append(Path(cargo_home) / "bin" / "cargo.exe")
        candidates.append(Path.home() / ".cargo" / "bin" / "cargo.exe")
        candidates.append(Path.home() / ".cargo" / "bin" / "cargo")
        return candidates

    def _find_cargo(self):
        for candidate in self._cargo_candidates():
            try:
                if candidate.is_file() and not winplatform.file_has_reparse_point(candidate):
                    return candidate
            except OSError:
                continue
        return None

    def _try_dev_build(self):
        """Geliştirme ağacında motor yoksa `cargo build --release` dener."""
        engine_dir = common.SCRIPT_DIR.parent / "engine"
        if getattr(sys, "frozen", False):
            return False
        if not (engine_dir / "Cargo.toml").exists():
            return False
        cargo = self._find_cargo()
        if cargo is None:
            return False
        self.status_badge.setText(T("Motor derleniyor..."))
        theme.set_state(self.status_badge, "info")
        QApplication.processEvents()
        try:
            subprocess.run(
                [cargo, "build", "--release", "--locked"],
                cwd=str(engine_dir), check=False,
                creationflags=CREATE_NO_WINDOW)
            common.DAEMON_BIN, common.PROFILE_PATH = common.locate_paths()
            return common.find_engine() is not None
        except Exception:
            return False

    def restart_daemon(self, show_message=False, force=False):
        # STOP'tan sonra yalnızca açık "Yeniden Başlat" isteği motoru açar.
        if self.is_manually_stopped and not force:
            self.update_daemon_status()
            return False
        self.is_manually_stopped = False
        self.lbl_stopped.hide()
        self.stop_restart_watch()
        if hasattr(self, "btn_restart_after_stop"):
            self.btn_restart_after_stop.setEnabled(False)

        common.DAEMON_BIN, common.PROFILE_PATH = common.locate_paths()
        try:
            # Önce olası eski motoru (doğrulanmış kimlikle) kapat. Kapanmazsa
            # yeni motor başlatılmaz: iki motor aynı anda tuş enjekte edip
            # panik tuşunu yarıştırabilirdi.
            if not common.stop_engine(timeout=2.0):
                if show_message:
                    QMessageBox.warning(
                        self, T("Motor Durdurulamadı"),
                        T("Eski motor durdurulamadı; yeniden başlatma "
                          "iptal edildi."))
                self.update_daemon_status()
                return False
            self.daemon_proc = None

            if common.find_engine() is None:
                self._try_dev_build()

            command = self._engine_command()
            if command is None:
                if show_message:
                    QMessageBox.warning(
                        self, T("Motor Bulunamadı"),
                        T("macrostat-engine.exe bulunamadı. Geliştirme için "
                          "windows/engine dizininde 'cargo build --release "
                          "--locked' çalıştırın."))
                self.update_daemon_status()
                return False

            try:
                self._daemon_log_offset = LOG_FILE.stat().st_size
            except OSError:
                self._daemon_log_offset = 0

            self.daemon_proc = subprocess.Popen(
                command,
                cwd=str(common.SCRIPT_DIR),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=CREATE_NO_WINDOW,
            )
            common.DAEMON_PROC = self.daemon_proc
            job = self._ensure_job_object()
            if job.available:
                job.assign(self.daemon_proc.pid)

            if show_message:
                time.sleep(0.3)
                self.failed_status_checks = 0
                self.update_daemon_status()
                QMessageBox.information(self, T("Başarılı"),
                                        T("Makro motoru başlatıldı."))
            else:
                QTimer.singleShot(350, self.update_daemon_status)
            return True
        except Exception as exc:
            if show_message:
                QMessageBox.critical(self, T("Hata"), str(exc))
            return False

    # -- STOP sonrası tuş izleyici ----------------------------------------

    def start_restart_watch(self):
        """STOP sonrası atanmış tuşu dinleyen hafif izleyiciyi başlatır."""
        if self.restart_watch_proc is not None:
            if getattr(self, "_restart_watch_key", None) == self.restart_key:
                return
            # Tuş değişti: eski izleyici yeni tuşu dinlemez, yenisiyle değiştirilir.
            self.stop_restart_watch()
        common.DAEMON_BIN, common.PROFILE_PATH = common.locate_paths()
        if common.find_engine() is None:
            return
        command = common.restart_watch_command(common.DAEMON_BIN, self.restart_key)
        proc = QProcess(self)
        hide_console(proc)
        proc.setProgram(command[0])
        proc.setArguments(command[1:])
        proc.finished.connect(self._restart_watch_finished)
        proc.errorOccurred.connect(lambda _err: self.stop_restart_watch())
        self.restart_watch_proc = proc
        self._restart_watch_key = self.restart_key
        proc.start()

    def stop_restart_watch(self):
        proc = self.restart_watch_proc
        self.restart_watch_proc = None
        self._restart_watch_key = None
        if proc is None:
            return
        try:
            # Bilinçli durdurulan izleyicinin `finished` sinyali hata banner'ı üretmesin.
            proc.finished.disconnect()
        except (TypeError, RuntimeError):
            pass
        try:
            if proc.state() != QProcess.ProcessState.NotRunning:
                proc.terminate()
        except Exception:
            pass
        proc.deleteLater()

    def _restart_watch_finished(self, exit_code, _exit_status):
        proc = self.restart_watch_proc
        if proc is None:
            # Zaten durdurulmuş (elle yeniden başlatma/kapanış) izleyicinin geç gelen çıkışı.
            return
        self.restart_watch_proc = None
        self._restart_watch_key = None
        proc.deleteLater()
        if exit_code == common.RESTART_WATCH_EXIT_CODE:
            self.show_autosave_banner(T(
                "Yeniden başlatma tuşuna basıldı; makro motoru açılıyor."))
            self.restart_daemon(show_message=False, force=True)
        elif exit_code not in (0, None):
            self.show_autosave_banner(T(
                "Yeniden başlatma tuşu dinlenemedi (kod {}).").format(exit_code))

    def restart_after_stop(self):
        """STOP sonrası motoru düğmeyle yeniden başlatır."""
        self.stop_restart_watch()
        self.restart_daemon(show_message=False, force=True)
        if self.is_daemon_running():
            self.show_autosave_banner(T("Makro motoru yeniden başlatıldı."))

    def trigger_panic(self):
        try:
            self.is_manually_stopped = True
            self.lbl_stopped.show()
            self.stop_daemon()
            self.status_badge.setText(T("Motor Durduruldu (STOP)"))
            theme.set_state(self.status_badge, "error")
            self.btn_restart_after_stop.setEnabled(True)
            self.start_restart_watch()
            self.show_autosave_banner(T(
                "STOP Tetiklendi! Motor 'Yeniden Başlat' düğmesine veya {} tuşuna "
                "basılana dek kapalı kalacaktır.").format(self.restart_key))
        except Exception:
            pass

    def closeEvent(self, event):
        self.stop_restart_watch()
        if getattr(self, "recording_active", False):
            self.recording_active = False
            self.releaseKeyboard()
        record_proc = getattr(self, "_input_record_proc", None)
        if record_proc is not None:
            try:
                record_proc.terminate()
                record_proc.waitForFinished(1500)
            except Exception:
                pass
        if not getattr(self, "_force_close", False) and not \
                self.confirm_discard_editor_changes():
            event.ignore()
            return
        try:
            self.status_timer.stop()
        except Exception:
            pass
        common.cleanup_daemon()
        try:
            if getattr(self, "daemon_proc", None) is not None and \
                    self.daemon_proc.poll() is None:
                self.daemon_proc.wait(timeout=2.0)
        except Exception:
            pass
        job = getattr(self, "job_object", None)
        if job is not None:
            job.close()
        event.accept()
