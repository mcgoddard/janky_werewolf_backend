SHELL = /bin/sh

build:
	$(MAKE) -C ${t} build

clean:
	$(MAKE) -C ${t} clean

clippy:
	$(MAKE) -C ${t} clippy

build_all:
	for package in common broadcast_lambda api_lambda; do\
		$(MAKE) -C $$package build;\
	done

clean_all:
	for package in common broadcast_lambda api_lambda; do\
		$(MAKE) -C $$package clean;\
	done

clippy_all:
	for package in common broadcast_lambda api_lambda; do\
		$(MAKE) -C $$package clippy;\
	done

deploy: export AWS_PROFILE = jankywerewolf_admin
deploy:
	$(MAKE) -C terraform/main apply

install: export AWS_PROFILE = jankywerewolf_admin
install:
	for package in common broadcast_lambda api_lambda; do\
		$(MAKE) -C $$package install;\
	done
	$(MAKE) -C terraform install
	$(MAKE) -C terraform/main init
	$(MAKE) -C tests install

ci_install:
	apt update && apt install -y libfindbin-libs-perl musl-tools
	rustup component add clippy --toolchain 1.49.0-x86_64-unknown-linux-gnu
	rustup target add x86_64-unknown-linux-musl
