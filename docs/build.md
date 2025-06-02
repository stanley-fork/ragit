# Build

## cargo

If you have [cargo](https://github.com/rust-lang/cargo) installed, just run `cargo install ragit`.

## install from source

### Dependencies

- ubuntu: `sudo apt install clang libssl-dev libfontconfig1-dev make unzip`
- redhat: `sudo dnf install clang openssl-devel fontconfig-devel unzip`

These are dependencies needed when all the features are enabled. By default, most features are disabled and you don't need all of them.

### Compile

```
git clone https://github.com/baehyunsol/ragit
```

```
cd ragit
```

You need [cargo](https://github.com/rust-lang/cargo) anyway to compile the source. `cargo` is the only dependency. Make sure that your rust-toolchain is up to date. Ragit doesn't have an MSRV policy. It's always tested on the newest version of rust and is likely to fail on 1.84 or older versions.

```
# only default features
cargo build --release

# enable all features
cargo build --release --features=full
```

### Building mupdf-rs

If you enable `pdf` feature, it depends on [mupdf-rs](https://github.com/messense/mupdf-rs).

On apple silicon, you have to add below lines to `Cargo.toml`. It's already added to ragit. If you want to use ragit as library, you have to add it manually.

```toml
[patch.crates-io]
pathfinder_simd = { git = "https://github.com/servo/pathfinder.git" }
```

## Test scripts

If you have Python, test your build with test scripts.

```
python scripts/tests.py all
```

## Download pre-built binary

Check out [github release page](https://github.com/baehyunsol/ragit/releases)
