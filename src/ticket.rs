use scrypto::prelude::*;

#[derive(ScryptoSbor, Clone, Debug, PartialEq, Eq, NonFungibleData)]
pub struct TicketData {
    #[mutable]
    pub pending_order_ids: Vec<u64>,
    #[mutable]
    pub cancel_as_buyer: u32,
    #[mutable]
    pub cancel_as_seller: u32,
    #[mutable]
    pub completed_as_buyer: u32,
    #[mutable]
    pub completed_as_seller: u32,
    #[mutable]
    pub volume_as_buyer: Decimal,
    #[mutable]
    pub volume_as_seller: Decimal,
    #[mutable]
    pub nostr_pub_key: String,
    
    pub deposit_amount: Decimal,
    // Amount required per transaction (deposit_amount/deposit_price = number of orders available for participation)
    pub channel_price: Decimal
}
