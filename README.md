# FerrisChat Server

No one appreciates Discord's restrictive, unreliable, and downright annoying backend.
Enter the world of Ferris! Ferris is here to save everyone from the hell that is Electron.

One day Ferris aspires to be at the core of the fastest, leanest, and most feature-rich
chat app ever written. But until that day arrives, this is where Ferris will oversee
the entirety of development.

## Contributing

We welcome contributions from everyone! Please read our [Contributing Guide](CONTRIBUTING.md) to get started.

## Temporary Discord

Join us in our Discord server while we develop FerrisChat! https://discord.gg/ARwnUwWXNY

## Self-hosting

Please note that you are not allowed to self-host FerrisChat for commerical purposes. Otherwise, feel free to
self-host FerrisChat for personal use.

Create a new parent directory for this repository before cloning the repository:

```shell
$ mkdir FerrisChat
$ cd FerrisChat
```

Then, clone this repository into that directory:

```shell
# Assuming the current directory is FerrisChat (hence the `cd`)
$ git clone https://github.com/FerrisChat/Server
```

That's not it! The FerrisChat server also requires a copy of the FerrisChat [Common](https://github.com/FerrisChat/Common) 
crate, which contains most of the types and traits used in the server. Clone that repository into the same directory:

```shell
# Assuming the current directory is FerrisChat
$ git clone https://github.com/FerrisChat/Common
```

Now, you should have created a directory structure like this:

```
FerrisChat
├── Common/
└── Server/
```

#### I don't want to clone the Common repository!

You can use a patch so that cargo uses the GitHub version of `Common` instead of the local version. This means that
the odd directory setup above will not be needed.

This is only recommended if you plan to not contribute and you are solely hosting FerrisChat.

The patch is as follows:
```toml
[patch."../Common"]
common = { git = "https://github.com/FerrisChat/Common" }
```

Add this in the top-level `Cargo.toml` file. Make sure to only append to the file and not overwrite it.

Note that in order to update `Common`, you must run `cargo update` manually!
