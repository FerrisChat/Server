#!/bin/bash

set -eo pipefail
if [[ -z $FC_TEMP_BUILD_DIR ]]; then
  :
else
  mkdir /tmp/fc_setup
  cd /tmp/fc_setup
fi

echo "Checking if Rust is installed..."
if [[ ! $(command -v cargo &> /dev/null) ]]; then
  echo "cargo not found, installing it now..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -q -y --default-toolchain nightly
  source $HOME/.cargo/env
fi


echo "Cloning server repo..."
git clone --branch develop https://github.com/FerrisChat/Server || (cd Server; git checkout develop; git pull; cd ..)


cd Server
echo "Building server binary with optimizations..."
SQLX_OFFLINE="true" RUSTFLAGS="-Ctarget-cpu=native --emit=asm" cargo build --release
echo "Copying server binary to /usr/bin..."
rm /usr/bin/ferrischat_server
mv target/release/ferrischat_server /usr/bin

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
