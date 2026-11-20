"""Makes the user's site-packages (numpy, Pillow) importable under `python -I`, which drops it.

Appended last, so nothing in it can shadow the standard library or a script of this folder.
"""

import site
import sys

user = site.getusersitepackages()
if user not in sys.path:
    sys.path.append(user)
