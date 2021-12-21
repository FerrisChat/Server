# FerrisChat server installation guide
## Pre-installation
FerrisChat requires Rust's `-Ctarget-cpu=native` to be set, which means we do not provide pre-built binaries.
```bash
sudo apt update
sudo apt install git curl nginx gcc libssl-dev pkg-config postgresql-13 redis-server
sudo useradd --system ferris
sudo mkdir /etc/ferrischat && sudo chown -R ferris:ferris /etc/ferrischat
```
You can also install Redis from source or get postgres from their offical repositories.
Now we install Rust:
```bash
curl https://sh.rustup.rs -sSf | sh
```
Make sure to select "Customize installation" and then set the default toolchain to Nightly.
If rust is already installed, you can run 
```bash
rustup install nightly
```
You also need a tool called sqlx-cli:
```bash
cargo install sqlx-cli
```
## Server setup
Then get the source:
```bash
git clone https://github.com/FerrisChat/Server.git
cd Server
```
Now set up the database. You need a user called `ferris_chat` with the password `ferris_chat` in your Postgres database.
```bash
cargo sqlx migrate run
```
Then you can actually build the server!
```bash
RUSTFLAGS="-Ctarget-cpu=native --emit=asm" cargo build --release --bin both
sudo mv ./target/release/both /etc/ferrischat/server
```
Now you can set up the server to run as a service using [this service file](https://github.com/FerrisChat/scripts/blob/main/host/ferrischat_selfhost.service) and [this config](https://github.com/FerrisChat/Server/blob/develop/config.example.toml), which should go in /etc/ferrischat/config.toml and be owned by the `ferris` user.
