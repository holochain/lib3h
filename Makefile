# Top-Level Environment Switcher
# This will delegate to sub Makefile.* files

.PHONY: all test fmt clean

export
all test fmt clean:
ifeq ($(MAKE_ENV),local)
	$(MAKE) -f build/make/Makefile.local.mk $@
else
	$(MAKE) -f build/make/Makefile.nix.mk $@
endif
