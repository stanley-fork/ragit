TODO: How do I run this on another machine?

```sh
cargo install sqlx-cli
export DATABASE_URL=postgres://postgres:password@url/to/your/db:5432/
sqlx migrate run
cargo run --release
```
