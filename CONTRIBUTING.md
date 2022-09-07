# Contributing

First and foremost, we truly do appreciate your interest in contributing to FerrisChat! Before you start, make sure your
contribution meets the following criteria:

- Code is formatted with `rustfmt` - since this is a cargo project this can be done running `cargo fmt`.
- Code passes `clippy` with our preferred lints - this can be done by running
  `cargo clippy --workspace -- -D clippy::all -D clippy::pedantic -D clippy::nursery -D clippy::cargo`.

Our workflows will check these for you, but please note that we will only merge contributions that pass these checks.

## Setting up the project

FerrisChat requires the [Common](https://github.com/FerrisChat/Common/tree/rewrite) crate in order for it to work. 
Because you plan to contribute, it is recommended you also have a copy of the Common repository, too.

Instructions to set this up can be found [here](https://github.com/FerrisChat/Server/tree/rewrite#self-hosting).

```shell