# Top-Level Environment Switcher
# This will delegate to sub Makefile.* files

.PHONY: all test fmt clean benchmarks

export
all test fmt clean benchmarks:
ifeq ($(MAKE_ENV),local)
	$(MAKE) -f scripts/make/Makefile.local.mk $@
else
	$(MAKE) -f scripts/make/Makefile.nix.mk $@
endif
