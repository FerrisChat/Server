# Contributing

First and foremost, we truly do appreciate your interest in contributing to FerrisChat! Before you start, make sure your
contribution meets the following criteria:

- Code is formatted with `rustfmt` - since this is a cargo project this can be done running `cargo fmt`.
- Code passes `clippy` with our preferred lints - this can be done by running
  `cargo clippy --workspace -- -D clippy::all -D clippy::pedantic -D clippy::nursery -D clippy::cargo`.

Our workflows will check these for you, but please note that we will only merge contributions that pass these checks.