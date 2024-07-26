use scrypto::prelude::*;


pub fn get_bp_fee(dec: Decimal) -> Decimal{
    dec.checked_div(Decimal::from_str("10000")).unwrap()
}