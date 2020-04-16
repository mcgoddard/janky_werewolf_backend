SHELL := /bin/bash

install:
	curl https://sh.rustup.rs -sSf | sh
	source ~/.cargo/env
	~/.cargo/bin/rustup target add x86_64-unknown-linux-musl
	sudo apt-get install musl-tools

build:
	cargo build --release --target x86_64-unknown-linux-musl

clean:
	rm -rf release || true
	rm release.zip || true
	cargo clean