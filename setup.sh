#!/bin/bash

set -eo pipefail
if [[ -z $FC_TEMP_BUILD_DIR ]]; then
  false
else
  mkdir /tmp/fc_setup
  cd /tmp/fc_setup
fi


echo "Adding new user to system..."
adduser --system fc


echo "Cloning server repo..."
git clone https://github.com/FerrisChat/Server


cd Server
echo "Building server binary with optimizations..."
RUSTFLAGS="-Ctarget-cpu=native --emit=asm" cargo build --release
echo "Copying server binary to /usr/bin..."
mv target/release/ferrischat_server /usr/bin


echo "Setting up config files..."
mkdir /etc/ferrischat

# why not echo aaaaaaaaaaaaaaaaa
# see https://unix.stackexchange.com/questions/219268/ddg#219274
printf "[database]\n" >> /etc/ferrischat/config.toml
if [[ -z "${FC_DATABASE_HOST}" ]]; then
  printf "host=\"%s\"\n" "${FC_DATABASE_HOST}" >> /etc/ferrischat/config.toml
fi
if [[ -z "${FC_DATABASE_PORT}" ]]; then
  printf "port=%s\n" "${FC_DATABASE_PORT}" >> /etc/ferrischat/config.toml
fi
if [[ -z "${FC_DATABASE_USERNAME}" ]]; then
  printf "user=\"%s\"\n" "${FC_DATABASE_USERNAME}" >> /etc/ferrischat/config.toml
fi
if [[ -z "${FC_DATABASE_PASSWORD}" ]]; then
  printf "password=\"%s\"\n" "${FC_DATABASE_PASSWORD}" >> /etc/ferrischat/config.toml
fi

printf "\n[redis]\n" >> /etc/ferrischat/config.toml
if [[ -z "${FC_REDIS_HOST}" ]]; then
  printf "host=\"%s\"\n" "${FC_REDIS_HOST}" >> /etc/ferrischat/config.toml
fi
if [[ -z "${FC_REDIS_PORT}" ]]; then
  printf "port=%s\n" "${FC_REDIS_PORT}" >> /etc/ferrischat/config.toml
fi
if [[ -z "${FC_REDIS_USERNAME}" ]]; then
  printf "user=\"%s\"\n" "${FC_REDIS_USERNAME}" >> /etc/ferrischat/config.toml
fi
if [[ -z "${FC_REDIS_PASSWORD}" ]]; then
  printf "password=\"%s\\n" "${FC_REDIS_PASSWORD}" >> /etc/ferrischat/config.toml
fi


echo "Adding systemctl service..."
if [[ -z "${FC_NO_SYSTEMCTL_SETUP}" ]]; then
  echo "Skipping because FC_NO_SYSTEMCTL_SETUP is set..."
else
  mv node_setup/ferrischat_server.service /etc/systemd/system/ferrischat_server.service
  echo "Enabling systemctl service..."
  systemctl enable ferrischat_server.service
  echo "Starting systemctl service..."
  if [[ -z ${FC_NO_STARTUP} ]]; then
    echo "Skipping because FC_NO_STARTUP is set..."
  else
    systemctl start ferrischat_server.service
  fi
fi


echo "Cleaning up after setup..."
if [[ -z $FC_NO_SYSTEMCTL_SETUP ]]; then
  rm -rf node_setup/ || false
fi
if [[ -z $FC_TEMP_BUILD_DIR ]]; then
  false
else
  cd /tmp/
  rm -rf fc_setup/ || false
fi


echo "Node setup complete. You may want to edit the config file, which is probably at /etc/ferrischat/config.toml depending on setup status."
echo "It is probably a good idea to reboot the system to clean up any leftovers from setup. If the systemctl service was set up, the server will automatically start on boot."
