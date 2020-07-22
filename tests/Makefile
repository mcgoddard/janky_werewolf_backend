SHELL = /bin/sh

install:
	sudo apt install -y python3 python3-pip
	sudo pip3 install pipenv
	pipenv update
	pipenv install --dev

run:
	pipenv run behave
