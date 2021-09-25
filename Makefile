.PHONY: all target/release/clean-recently-used check install clean

CARGO = cargo

INSTALL = install
INSTALL_PROGRAM = $(INSTALL)
INSTALL_DATA = $(INSTALL) -m 644

all: target/release/clean-recently-used

target/release/clean-recently-used:
	$(CARGO) $(CARGOFLAGS) build --release

check:
	$(CARGO) $(CARGOFLAGS) check
	$(CARGO) $(CARGOFLAGS) test
	$(CARGO) $(CARGOFLAGS) clippy

install: target/release/clean-recently-used clean-recently-used@.service clean-recently-used@.timer
	$(INSTALL_PROGRAM) $< ~/.local/bin/
	$(INSTALL_DATA) clean-recently-used@.service clean-recently-used@.timer ~/.config/systemd/user/

clean:
	$(CARGO) $(CARGOFLAGS) clean
