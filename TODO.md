- implement permissions system using landlock(7) (Linux) or unveil(2) (OpenBSD)
- detect when shell is called from a Makefile using `$MAKELEVEL` and apply
  security features
