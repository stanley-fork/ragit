# Build

## cargo

If you have [cargo](https://github.com/rust-lang/cargo) installed, just run `cargo install ragit`.

## install from source

```
git clone https://github.com/baehyunsol/ragit
```

```
cd ragit
```

You need [cargo](https://github.com/rust-lang/cargo) anyway to compile the source. `cargo` is the only dependency. Make sure that your rust-toolchain is up to date. Ragit doesn't have an MSRV policy (I don't have time to test it on different rust versions), but it's likely to fail on 1.80 or older versions of rust.

```
cargo build
```

If you have Python, test your build with test scripts.

```
python scripts/tests.py all
```

## Download pre-built binary

Check out [github release page](https://github.com/baehyunsol/ragit/releases)
