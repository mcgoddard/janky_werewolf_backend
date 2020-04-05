SHELL = /bin/sh

clean_connect:
	$(MAKE) -C connect_lambda clean

build_connect:
	$(MAKE) -C connect_lambda lambda

deploy: export AWS_PROFILE = jankywerewolf_admin
deploy: build_connect
	$(MAKE) -C terraform/main apply
