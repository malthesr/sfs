# sfs

**`sfs`** is a tool for creating and working with the site frequency spectra.

The README is currently under construction. Usage and examples will be added soon.

## Contents

1. [Installation](#installation)
    1. [From source](#from-source)
	    1. [Latest release](#latest-release)
	    2. [Current git](#current-git)
    1. [Pre-built](#pre-built)

## Installation

### From source

A recent Rust toolchain is required to build `sfs` from source. Currently, the Rust toolchain can be installed by running:

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

See [instructions][rust-installation] for more details.

`sfs` can now be build from source, using either the latest released version, or the current git head, as described below.

#### Latest release

The following will install the latest released version of `sfs`:

```shell
cargo install sfs-cli
```

This will install the `sfs` binary to `$HOME/.cargo/bin` by default, which should be in the `$PATH` after installing `cargo`. Alternatively:

```shell
cargo install sfs-cli --root $HOME
```

Will install to `$HOME/bin`.

#### Current git

The latest git version may include more (potentially experimental) features, and can be installed using:

```shell
cargo install --git https://github.com/malthesr/sfs
```

### Pre-built

Pre-built binaries are available from the [releases][releases] page ([linux][linux-binary], [mac][mac-binary], [windows][windows-binary]).

For a one-liner, something like the following should work in a UNIX environment:

```shell
curl -s -L $url | tar xvz -O > $dest
```

Where `$url` is chosen from above, and `$dest` is the resulting binary, e.g. `$HOME/bin/sfs`.

[releases]: https://github.com/malthesr/sfs/releases/latest/
[linux-binary]: https://github.com/malthesr/sfs/releases/latest/download/sfs-x86_64-unknown-linux-gnu.tar.gz
[mac-binary]: https://github.com/malthesr/sfs/releases/latest/download/sfs-x86_64-apple-darwin.tar.gz
[windows-binary]: https://github.com/malthesr/sfs/releases/latest/download/sfs-x86_64-pc-windows-msvc.zip
[rust-installation]: https://www.rust-lang.org/tools/install
