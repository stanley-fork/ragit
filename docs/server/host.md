It requires a postgres DB to run. If you're not sure what to do, ask chatGPT `"I want to run a simple web server and the server wants me a URL of a postgres DB. How do I host a postgres DB?"`.

With the DB, please set env var `DATABASE_URL` with the DB URL (TODO: use dotenvy crate).

> `export DATABASE_URL=postgres://postgres:password@url/to/your/db:5432/`

This has to be done before compilation or migration. If it's first time running ragit-server, you have to init your DB. [sqlx](https://github.com/launchbadge/sqlx) provides you a cli tool for that. Install `sqlx-cli` and run `sqlx migrate run`

> `cargo install sqlx-cli; sqlx migrate run`

Now you're good to go.

> `cargo run --release -- run`
