# Hosting ragit-server

You can host a simple server where you can clone/push knowledge-bases to.

It requires a postgres DB to run. Create an empty postgres instance, and it'll do the rest.

With the DB, please set env var `DATABASE_URL` with the DB URL (TODO: use dotenvy crate).

> `export DATABASE_URL=postgres://postgres:password@url/to/your/db:5432/`

This has to be done before compilation or migration. If it's first time running ragit-server, you have to init your DB. [sqlx](https://github.com/launchbadge/sqlx) provides you a cli tool for that. Install `sqlx-cli` and run `sqlx migrate run`

> `cargo install sqlx-cli; sqlx migrate run`

Now you're good to go.

> `cargo run --release -- run`

By default, it runs on port 41127. You can change the port number with `--port` option.
