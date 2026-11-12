PREFIX ?= /usr/local
DESTDIR ?=

build:
	cargo build --release

# For packages and system-wide installs. A plain `cargo install --path .` needs none of
# this: undertow adds its own launcher entry the first time it is opened.
install: build
	install -Dm755 target/release/undertow $(DESTDIR)$(PREFIX)/bin/undertow
	install -Dm644 assets/undertow.desktop $(DESTDIR)$(PREFIX)/share/applications/undertow.desktop
	install -Dm644 assets/undertow.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/undertow.svg

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/undertow $(DESTDIR)$(PREFIX)/share/applications/undertow.desktop \
		$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/undertow.svg

.PHONY: build install uninstall
