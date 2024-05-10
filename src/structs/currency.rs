
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8
}
