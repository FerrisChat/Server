# FerrisChat server installation guide
## Pre-installation
FerrisChat requires Rust's `-Ctarget-cpu=native` to be set, which means we do not provide pre-built binaries.
```bash
sudo apt update
sudo apt install git curl nginx gcc postgresql-13 
sudo useradd --system ferris
sudo mkdir /etc/ferrischat && sudo chown -R ferris:ferris /etc/ferrischat
```
You can also get postgres from their offical repository, rather then your OSes repo.
You also need Redis, which you can see how to install [here](https://redis.io/topics/quickstart).
**DO NOT INSTALL REDIS FROM YOUR OS REPO**.
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
Using [this service file](https://github.com/FerrisChat/scripts/blob/main/host/ferrischat_selfhost.service) (which can be placed in `/etc/systemd/system/`) and [this config](https://github.com/FerrisChat/Server/blob/develop/config.example.toml), which should go in `/etc/ferrischat/config.toml`.  You also need to add an Nginx config file, an example of which can be found [here](https://github.com/FerrisChat/Server/blob/develop/ferrischat-nginx.conf).
