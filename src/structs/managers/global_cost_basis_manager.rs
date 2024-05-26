use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::structs::{GlobalCostBasis, TransactionId};

use super::Persistable;


/* This manager is used for associating the global cost basis at a time "t" => associated with a transaction
This allow to save the history of global cost basis with drop implementation
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCostBasisManager {
    pub global_cost_basis_history : HashMap<TransactionId,GlobalCostBasis>,
    path: String,
}


impl Persistable for GlobalCostBasisManager {
    const PATH:  &'static str = ".data/global_cost_basis_history";

    fn default_new(path: String) -> Self {
        Self {
            global_cost_basis_history: HashMap::new(),
            path,
        }
    }

    fn get_path(&self) -> &str{
        return &self.path;
    }
}

impl Drop for GlobalCostBasisManager {
    fn drop(&mut self) {
        let _save = self.save();
    }
}
