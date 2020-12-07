#!/bin/bash
set -e 

main(){
	echo ":::"

	# Must be root to install
	if [[ $EUID -eq 0 ]];then
		echo "::: You are root."
	else
		echo "::: sudo will be used for the install."
		# Check if it is actually installed
		# If it isn't, exit because the install cannot complete
		if [[ $(dpkg-query -s sudo) ]];then
			export SUDO="sudo"
			export SUDOE="sudo -E"
		else
			echo "::: Please install sudo or run this as root."
			exit 1
		fi
	fi
	echo "::: Installing AWS SSO, but better"
	LATEST_RELEASE=$(curl --silent "https://api.github.com/repos/thehamsterjam/better_aws_sso/releases/latest" | grep -Po '"tag_name": "\K.*?(?=")')
	# echo "::: Will install version $LATEST_RELEASE"

	if [[ -f /usr/local/bin/ssologin ]]; then
		echo "::: ssologin exists on your filesystem, considering upgrading..."

		INSTALLED_VERSION=$(ssologin -V)
		temp=$(echo $INSTALLED_VERSION)
		INSTALLED_VERSION=(${temp//[\(\),]/})
		INSTALLED_VERSION=$(echo ${INSTALLED_VERSION[${#INSTALLED_VERSION[@]}-1]})

		if [ $INSTALLED_VERSION = $LATEST_RELEASE ]; then
			echo "::: You already have the latest version."
			exit 0
		else
			echo "::: Newer version avaliable, will upgrade now..."
		fi
	fi

	echo "::: Will install version $LATEST_RELEASE"
	wget -q --show-progress https://github.com/thehamsterjam/better_aws_sso/releases/download/$LATEST_RELEASE/ssologin_ubuntu
	
	echo "::: I require your password for chmod and install"
	$SUDO chmod +x ssologin_ubuntu
	$SUDO mv ssologin_ubuntu /usr/local/bin/ssologin
	ssologin --help
	echo "::: Done"
}

trap ctrl_c INT
function ctrl_c() {
	echo ""
	echo "::: Cleaning up and exiting..."
	rm -f ssologin*
	echo "::: Done"
}

main "$@"