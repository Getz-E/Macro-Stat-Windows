# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
"""MacroStat Windows arayüzü giriş noktası (geliştirme ve PyInstaller).

Kullanım: ``python gui/main.py`` (depo kökünden) ya da paketlenmiş ``MacroStat.exe``.
"""

import sys
from pathlib import Path

# Paket ``gui/macrostat`` altında; betik doğrudan çalıştırıldığında bu dizin
# zaten `sys.path[0]`'dır, ancak kısayol/ilişkilendirme yoluyla farklı bir
# çalışma dizininden açılırsa garantiye alınır.
_PACKAGE_ROOT = Path(__file__).resolve().parent
if str(_PACKAGE_ROOT) not in sys.path:
    sys.path.insert(0, str(_PACKAGE_ROOT))

from macrostat.app import main  # noqa: E402

if __name__ == "__main__":
    main()
