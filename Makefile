APP := tinydock
VERSION := 1.0
ARCH := $(shell dpkg --print-architecture 2>/dev/null || echo amd64)
DEB := $(APP)_$(VERSION)_$(ARCH).deb

.PHONY: all build release deb install uninstall clean

all: release

build release:
	cargo build --release

$(DEB): release
	./build-deb.sh $(VERSION)

deb: $(DEB)

install: release
	./install.sh

uninstall:
	rm -f /usr/local/bin/$(APP)
	rm -f /usr/local/bin/tclock
	rm -f /usr/share/applications/$(APP).desktop
	rm -f /usr/share/applications/tclock.desktop
	rm -f $(HOME)/.config/autostart/$(APP).desktop
	rm -f $(HOME)/.config/autostart/tclock.desktop

clean:
	cargo clean
	rm -f $(DEB)
