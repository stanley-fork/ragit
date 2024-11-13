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

You need [cargo](https://github.com/rust-lang/cargo) anyway to compile the source. `cargo` is the only dependency.

```
cargo build
```

If you have Python, test your build with test scripts.

```
python scripts/tests.py all
```
