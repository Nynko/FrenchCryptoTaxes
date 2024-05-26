use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::structs::{Taxable, TransactionId};

use super::Persistable;


/* This manager is used for associating the global cost basis at a time "t" => associated with a transaction
This allow to save the history of global cost basis with drop implementation
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxableManager {
    pub taxables : HashMap<TransactionId,Taxable>,
    path: String,
}


impl Persistable for TaxableManager {
    const PATH:  &'static str = ".data/taxables";

    fn default_new(path: String) -> Self {
        Self {
            taxables: HashMap::new(),
            path,
        }
    }

    fn get_path(&self) -> &str{
        return &self.path;
    }
}

impl Drop for TaxableManager {
    fn drop(&mut self) {
        let _save = self.save();
    }
}
