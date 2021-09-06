# FerrisChat Server

Fuck Discord's shitty, restrictive, slash-command-loving, and downright rude backend! Enter the world of Ferris! Ferris is here to save everyone from the hell that is Electron.

One day Ferris aspires to be at the core of the fastest, leanest, and most feature-rich
chat app ever written. But until that day arrives, this is where Ferris will oversee
the entirety of development.

# Contributing

**NOTE: THIS IS THE REPO FOR THE SERVER ONLY!!!**

Basically look at the issues, and see if there's something you can help with.
Look at the issue thread and make sure no one's claimed it yet. If no one has, go
ahead and write a comment saying you're claiming it.

You must run `rustfmt` **with default settings** on a PR for it to be merged.

# Temporary Discord

Join us in our Discord server while we develop FerrisChat! https://discord.gg/ARwnUwWXNY

# FAQ

### I get a `error[E0308]: mismatched types` when building `simd-json`
The error probably looks something like this: 
```
error[E0308]: mismatched types
   --> /home/dustin/.cargo/registry/src/github.com-1ecc6299db9ec823/simd-json-0.4.7/src/lib.rs:215:86
    |
215 | fn please_compile_with_a_simd_compatible_cpu_setting_read_the_simdjonsrs_readme() -> ! {}
    |    ----------------------------------------------------------------------------      ^ expected `!`, found `()`
    |    |
    |    implicitly returns `()` as its body has no tail or `return` expression
    |
    = note:   expected type `!`
            found unit type `()`
```
Read the function name. You're compiling for a CPU that doesn't support `sse4.2`, `avx2`, `pclmulqdq`, or (on ARM targets) `neon`.
This is to prevent slowness that can be hard to debug.
To fix it, add `allow-non-simd` to the `features` field for `simd-json` in `ferrischat_ws/Cargo.toml`.

On non-ARM targets, to test your support, you can run the following commands. If there is no output, your CPU does not support the feature.
```
cat /proc/cpuinfo | grep pclmulqdq # This one is required for simd-json to compile
cat /proc/cpuinfo | grep avx2      # Either this one or...
cat /proc/cpuinfo | grep sse4_2    # this one are required as well as pclmulqdq
```
