use scrypto::prelude::*;

#[derive(ScryptoSbor, Clone, Debug, PartialEq, Eq, NonFungibleData)]
pub struct EscrowData {
    // pub trade_id: u64,
    // pub token_addr: ResourceAddress,
    // pub buyer: NonFungibleLocalId,
    // pub seller: NonFungibleLocalId,
    // pub price: Decimal, // as buyer. trade.amount = volume * price * (1- buy fee rate)
    // pub volume: Decimal, // as seller. trade.volume = amount * (1-sell fee rate)
    // pub buyer_fee_rate: Decimal,
    // pub seller_fee_rate: Decimal,
    pub cancel_after_epoch_by_seller: u64,
    pub gas_spent_by_relayer: Decimal
}