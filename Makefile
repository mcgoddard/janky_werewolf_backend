SHELL = /bin/sh

build:
	$(MAKE) -C ${t} build

clean:
	$(MAKE) -C ${t} clean

cargo:
	$(MAKE) -C ${t} cargo

deploy: export AWS_PROFILE = jankywerewolf_admin
deploy:
	for package in common broadcast_lambda connect_lambda start_lambda sleep_lambda lynch_lambda seer_lambda werewolf_lambda; do\
		$(MAKE) -C $$package build;\
	done
	$(MAKE) -C terraform/main apply
