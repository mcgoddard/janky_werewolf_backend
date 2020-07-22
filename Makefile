SHELL = /bin/sh

build:
	$(MAKE) -C ${t} build

clean:
	$(MAKE) -C ${t} clean

clippy:
	$(MAKE) -C ${t} clippy

build_all:
	for package in common broadcast_lambda connect_lambda start_lambda sleep_lambda lynch_lambda seer_lambda werewolf_lambda bodyguard_lambda; do\
		$(MAKE) -C $$package build;\
	done

deploy: export AWS_PROFILE = jankywerewolf_admin
deploy: build_all
	$(MAKE) -C terraform/main apply

install:
	for package in common broadcast_lambda connect_lambda start_lambda sleep_lambda lynch_lambda seer_lambda werewolf_lambda bodyguard_lambda; do\
		$(MAKE) -C $$package install;\
	done
	$(MAKE) -C terraform install
	$(MAKE) -C tests install
