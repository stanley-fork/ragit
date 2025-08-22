use crate::CONFIG;
use crate::error::Error;
use ragit_fs::{
    WriteMode,
    create_dir_all,
    join3,
    parent,
    read_bytes,
    write_bytes,
};

// Ragit-server has to read/write a lot of binary objects: mostly for archives.
// My first attempt was a Postgres table, where the blobs are saved as `BYTEA` type.
// I found that it's inefficient to store large blobs in Postgres, so I chose file system.
//
// It uses file system, but you might want to use another service (e.g. S3, GCP).
// You can easily port this. All you have to do is write your own version of `save` and `get`.
//
// NOTE: `id` is a hexadecimal, random generated, and 64-characters long string.
// NOTE: `blob` is immutable. It either saves a new blob with a new id, or reads an existing one.

pub fn save(id: &str, blob: &[u8]) -> Result<(), Error> {
    let blob_at = get_blob_path(id)?;
    create_dir_all(&parent(&blob_at)?)?;

    Ok(write_bytes(
        &blob_at,
        blob,
        WriteMode::CreateOrTruncate,
    )?)
}

pub fn get(id: &str) -> Result<Vec<u8>, Error> {
    Ok(read_bytes(&get_blob_path(id)?)?)
}

fn get_blob_path(id: &str) -> Result<String, Error> {
    let config = CONFIG.get().ok_or(Error::ConfigNotInitialized)?;
    let prefix = id.get(0..2).unwrap();
    let suffix = id.get(2..).unwrap();

    Ok(join3(
        &config.blob_data_dir,
        prefix,
        suffix,
    )?)
}
