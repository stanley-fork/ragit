use crate::error::Error;
use crate::index::{Index, LoadMode};
use serde::{Deserialize, Serialize};

pub type Path = String;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExternalIndex {
    path: Path,  // normalized rel_path
}

impl ExternalIndex {
    pub fn new(path: Path) -> Self {
        ExternalIndex { path }
    }
}

impl Index {
    // TODO: there must be a cycle check
    pub fn load_external_indexes(&mut self, load_mode: LoadMode) -> Result<(), Error> {
        for external_index_info in self.external_index_info.iter() {
            let mut external_index = Index::load(
                Index::get_data_path(&self.root_dir, &external_index_info.path),
                load_mode,
            )?;
            external_index.load_external_indexes(load_mode)?;
            self.external_indexes.push(external_index);
        }

        Ok(())
    }
}
