.PHONY: all target/release/clean-recently-used install clean

all: target/release/clean-recently-used

target/release/clean-recently-used:
	cargo build --release

install: target/release/clean-recently-used clean-recently-used@.service clean-recently-used@.timer
	cp $< ~/.local/bin/
	cp clean-recently-used@.service clean-recently-used@.timer ~/.config/systemd/user/

clean:
	cargo clean
