use scrypto::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ScryptoSbor, ManifestSbor)]
pub enum Instruction {
    Escrowed,
    BuyerPaid,
    Released,
    SellerRequestCancel,
    SellerCancelled,
    BuyerCancelled,
    Resolved
}


#[derive(ScryptoSbor, Clone, Debug, PartialEq, Eq)]
pub struct EscrowData {
    pub instruction: Instruction,
    pub last_epoch: u64,
    pub paid_epochs: u64,
    pub request_cancel_epoch: u64,
    pub gas_spent_by_relayer: Decimal
}