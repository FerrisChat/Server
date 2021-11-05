#!/bin/bash

set -eo pipefail
if [[ -z $FC_TEMP_BUILD_DIR ]]; then
  :
else
  mkdir /tmp/fc_setup
  cd /tmp/fc_setup
fi

echo "Downloading new binary..."
wget -O "ferrischat_server" https://download.ferris.chat/FerrisChat_Server

echo "Copying server binary to /usr/bin..."
rm /usr/bin/ferrischat_server
mv ferrischat_server /usr/bin
sudo chmod +x /usr/bin/ferrischat_server

echo "Restarting systemd service..."
systemctl restart ferrischat_server

echo "Cleaning up after setup..."
if [[ -z $FC_TEMP_BUILD_DIR ]]; then
  :
else
  cd /tmp/
  rm -rf fc_setup/ || :
fi

echo "Binary now updated!"
