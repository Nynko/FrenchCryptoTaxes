use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}
