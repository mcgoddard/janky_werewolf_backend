SHELL = /bin/sh

install: export TER_VER = 0.13.5
install:
	sudo apt install -y wget unzip
	wget https://releases.hashicorp.com/terraform/${TER_VER}/terraform_${TER_VER}_linux_amd64.zip
	unzip terraform_${TER_VER}_linux_amd64.zip
	sudo mv terraform /usr/local/bin/
	rm terraform_${TER_VER}_linux_amd64.zip
