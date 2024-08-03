use scrypto::prelude::*;


pub fn get_net_by_bp(base:Decimal, fee_rate: Decimal) -> Decimal{
    let bp = Decimal::from(10000);
    let net_rate = bp.checked_sub(fee_rate).unwrap();
    base.checked_mul(net_rate).expect("checked_mul").checked_div(bp).expect("checked_div")
}

pub fn get_fee_by_bp(base:Decimal, fee_rate: Decimal) -> Decimal{
    let bp = Decimal::from(10000);
    base.checked_mul(fee_rate).expect("checked_mul").checked_div(bp).expect("checked_div")
}

pub fn get_total_by_bp(base: Decimal, fee_rate: Decimal) -> Decimal{
    let bp = Decimal::from(10000);
    let net_rate = bp.checked_add(fee_rate).unwrap();
    base.checked_mul(net_rate).expect("checked_mul").checked_div(bp).expect("checked_div")
}