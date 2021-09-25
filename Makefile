.PHONY: all target/release/clean-recently-used check install clean

INSTALL = install
INSTALL_PROGRAM = $(INSTALL)
INSTALL_DATA = $(INSTALL) -m 644

all: target/release/clean-recently-used

target/release/clean-recently-used:
	cargo build --release

check:
	cargo check
	cargo test
	cargo clippy

install: target/release/clean-recently-used clean-recently-used@.service clean-recently-used@.timer
	$(INSTALL_PROGRAM) $< ~/.local/bin/
	$(INSTALL_DATA) clean-recently-used@.service clean-recently-used@.timer ~/.config/systemd/user/

clean:
	cargo clean
