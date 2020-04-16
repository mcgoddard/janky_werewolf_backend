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

clean_sleep:
	$(MAKE) -C sleep_lambda clean

build_sleep:
	$(MAKE) -C sleep_lambda build

clean_lynch:
	$(MAKE) -C lynch_lambda clean

build_lynch:
	$(MAKE) -C lynch_lambda build

clean_seer:
	$(MAKE) -C seer_lambda clean

build_seer:
	$(MAKE) -C seer_lambda build

clean_werewolf:
	$(MAKE) -C werewolf_lambda clean

build_werewolf:
	$(MAKE) -C werewolf_lambda build

clean_common:
	$(MAKE) -C common clean

build_common:
	$(MAKE) -C common build

deploy: export AWS_PROFILE = jankywerewolf_admin
deploy: build_common build_broadcast build_connect build_start build_sleep build_lynch build_seer build_werewolf
	$(MAKE) -C terraform/main apply
