use super::Index;
use crate::error::Error;
use crate::index::{ExternalIndex, LoadMode};

pub type Path = String;

impl Index {
    /// It extends the knowledge-base with an external knowledge-base.
    pub fn ext(&mut self, real_path: &Path) -> Result<(), Error> {
        let rel_path = Index::get_rel_path(
            &self.root_dir,
            real_path,
        );
        let new_index = Index::load(real_path.to_string(), LoadMode::OnlyJson)?;
        self.external_index_info.push(ExternalIndex::new(rel_path));
        self.external_indexes.push(new_index);
        Ok(())
    }
}
