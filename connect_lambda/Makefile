install:
	curl https://sh.rustup.rs -sSf | sh
	source $HOME/.cargo/env
	rustup target add x86_64-unknown-linux-musl
	sudo apt install musl-gcc
	sudo apt-get install musl-tools

build:
	cargo build --release --target x86_64-unknown-linux-musl
	mkdir -p release
	cp target/x86_64-unknown-linux-musl/release/bootstrap release/bootstrap

lambda: build
	zip -j rust.zip ./target/x86_64-unknown-linux-musl/release/bootstrap

clean:
	rm -rf release || true
	rm release.zip || true
	cargo clean