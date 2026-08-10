# Coywin Windows Executables

This folder contains pre-compiled, standalone Windows applications (`miner.exe` and `node.exe`) designed for users to interact with the Coywin network without needing to install Python or any dependencies.

## ⚠️ Antivirus Warning (False Positive)
Please note that these `.exe` files are generated using PyInstaller. Because they are not signed with a paid digital certificate, some antivirus software (including Windows Defender) may incorrectly flag them as a virus or malicious file. **This is a known false positive.**

The executables are simply bundles of the open-source Python scripts (`miner.py` and `viewer_server.py`) found in this repository, along with standard Python libraries like `Pillow` and `pqcrypto`.

If your antivirus blocks the execution or downloads, you can safely add an exception (whitelist) for these files. Alternatively, if you prefer, you can inspect the Python source code and run the `.py` scripts directly using your own Python environment.

---
*Mined by algorithms, secured by post-quantum cryptography.*
