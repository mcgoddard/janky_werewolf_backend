SHELL = /bin/sh

clean_broadcast:
	$(MAKE) -C broadcast_lambda clean

build_broadcast:
	$(MAKE) -C broadcast_lambda build

clean_connect:
	$(MAKE) -C connect_lambda clean

build_connect:
	$(MAKE) -C connect_lambda build

clean_start:
	$(MAKE) -C start_lambda clean

build_start:
	$(MAKE) -C start_lambda build

deploy: export AWS_PROFILE = jankywerewolf_admin
deploy: build_broadcast build_connect build_start
	$(MAKE) -C terraform/main apply
