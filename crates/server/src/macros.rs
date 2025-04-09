/// It's a very thin wrapper around `sqlx::query!`. If `log_sql` feature is set,
/// this macro calls `write_log((query, args))` before calling `sqlx::query!`.
/// `log_sql` is very expensive. Some tables store blobs where each blob is 100KiB ~ 2MiB.
/// Rust will try to dump the blob in `Vec<u8>` format, which is a few MiB per blob.
#[macro_export]
#[cfg(feature = "log_sql")]
macro_rules! query {
    ($query: expr, $($args:tt)*) => {{
        let r = sqlx::query!($query, $($args)*);
        ragit_fs::write_log(
            "sql",
            &format!("query: {:?}, args: {:?}", $query, ($($args)*)),
        );
        r
    }};
}

#[macro_export]
#[cfg(not(feature = "log_sql"))]
macro_rules! query {
    ($($args:tt)*) => { sqlx::query!($($args)*) };
}
