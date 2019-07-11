# Top-Level Environment Switcher
# This will delegate to sub Makefile.* files

.PHONY: all

export
all:
ifeq ($(MAKE_ENV),local)
	$(MAKE) -f Makefile.local
else
	$(MAKE) -f Makefile.nix
endif
