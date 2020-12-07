SHELL := /bin/bash

install:
	curl https://sh.rustup.rs -sSf | sh -s -- -y
	source ~/.cargo/env
	~/.cargo/bin/rustup target add x86_64-unknown-linux-musl
	sudo apt-get install -y musl-tools

build:
	cargo build --release --target x86_64-unknown-linux-musl
	mkdir -p release
	cp target/x86_64-unknown-linux-musl/release/bootstrap release/bootstrap

clean:
	rm -rf release || true
	rm release.zip || true
	cargo clean

clippy:
	cargo clippy -- -D warnings -A clippy::needless-arbitrary-self-type
