.PHONY: all target/release/clean-recently-used clean-recently-used@.service check install clean

CARGO = cargo

INSTALL = install
INSTALL_PROGRAM = $(INSTALL)
INSTALL_DATA = $(INSTALL) -m 644

all: target/release/clean-recently-used

target/release/clean-recently-used:
	$(CARGO) $(CARGOFLAGS) build --release

clean-recently-used@.service: clean-recently-used@.service.in
	USER_BINARIES=$$(systemd-path user-binaries) envsubst < $< > $@

check:
	$(CARGO) $(CARGOFLAGS) check
	$(CARGO) $(CARGOFLAGS) test
	$(CARGO) $(CARGOFLAGS) clippy

install: target/release/clean-recently-used clean-recently-used@.service clean-recently-used@.timer
	$(INSTALL_PROGRAM) $< "$$(systemd-path user-binaries)"
	$(INSTALL_DATA) clean-recently-used@.service clean-recently-used@.timer "$$(systemd-path user-configuration --suffix systemd/user)"

clean:
	$(CARGO) $(CARGOFLAGS) clean
	$(RM) clean-recently-used@.service
