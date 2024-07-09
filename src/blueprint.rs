use scrypto::prelude::*;
use crate::ticket::TicketData;

#[blueprint]
// #[events(SupplyEvent, WithdrawEvent, CreateCDPEvent, ExtendBorrowEvent, AdditionCollateralEvent, WithdrawCollateralEvent, RepayEvent, LiquidationEvent)]
mod lighter_radix{
    


    enable_method_auth! {
        roles{
            admin => updatable_by: [];
            operator => updatable_by: [admin];
        },
        methods {
            // new_pool => restrict_to: [admin, OWNER];
            take_ticket => PUBLIC;
        }
    }

    struct Lighter {
        ///
        /// ResourceManager for Ticket NFT. 
        ticket_res_mgr: ResourceManager,
        ///
        /// price per a trade channel on the Ticket.
        channel_price: Decimal,
        ///
        /// ticket valult
        ticket_vault: Vault,
        ///
        /// id
        ticket_id_counter: u64,
        admin_rule: AccessRule,
        op_rule: AccessRule
    }

    impl Lighter {

        pub fn instantiate(channel_price: Decimal) -> (Global<Lighter>, Bucket, Bucket){
            let admin_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata!(
                    init {
                        "name" => "Admin Badge".to_owned(), locked;
                        "description" => 
                        "This is a LighterIM admin badge used to authenticate the admin.".to_owned(), locked;
                    }
                ))
                .mint_initial_supply(1);
            let op_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata!(
                    init {
                        "name" => "Operator Badge".to_owned(), locked;
                        "description" => 
                        "This is a LighterIM operator badge used to authenticate the operator.".to_owned(), locked;
                    }
                ))
                .mint_initial_supply(1);
            
            let admin_badge_addr = admin_badge.resource_address();
            let op_badge_addr = op_badge.resource_address();
            let (address_reservation, component_address) =
            Runtime::allocate_component_address(Lighter::blueprint_id());

            let ticket_res_mgr = ResourceBuilder::new_integer_non_fungible::<TicketData>(OwnerRole::None)
                .metadata(metadata!(init{
                    "symbol" => "CDP", locked;
                    "name" => "DeXian CDP Token", locked;
                }))
                .mint_roles(mint_roles!( 
                    minter => rule!(require(global_caller(component_address)));
                    minter_updater => rule!(deny_all);
                ))
                .burn_roles(burn_roles!(
                    burner => rule!(require(global_caller(component_address)));
                    burner_updater => rule!(deny_all);
                ))
                .non_fungible_data_update_roles(non_fungible_data_update_roles!(
                    non_fungible_data_updater => rule!(require(global_caller(component_address)));
                    non_fungible_data_updater_updater => rule!(deny_all);
                ))
                .create_with_no_initial_supply();

            let component = Self{
                admin_rule: rule!(require(admin_badge_addr)),
                op_rule: rule!(require(op_badge_addr)),
                ticket_vault: Vault::new(XRD),
                ticket_id_counter: 1,
                channel_price,
                ticket_res_mgr
            }.instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(admin_badge_addr))))
            .with_address(address_reservation)
            .roles(roles! {
                admin => rule!(require(admin_badge_addr));
                operator => rule!(require(op_badge_addr));
            })
            .globalize();
            
            (component, admin_badge.into(), op_badge.into())
            
        }

        pub fn take_ticket(&mut self, nostr_pub_key: String, mut bucket: Bucket) -> Bucket{
            assert!(bucket.resource_address() == XRD && bucket.amount() >= self.channel_price, "unknow resource bucket or invalid amount");
            // assert!(true, "nostr public key invalid");

            let processed_order_cnt = bucket.amount().checked_div(self.channel_price).map(|x| x.checked_floor().unwrap());
            assert!(processed_order_cnt < Some(Decimal::one()), "Deposit is too small");
            // if processed_order_cnt > Decimal::from("10") {
            //     processed_order_cnt = Decimal::from("10");
            // }
            let deposit_amount = self.channel_price * processed_order_cnt.unwrap();
            self.ticket_vault.put(bucket.take(deposit_amount));
            let price = self.channel_price;

            let data = TicketData{
                pending_order_ids: Vec::new(),
                cancel_as_buyer: 0,
                cancel_as_seller: 0,
                completed_as_buyer: 0,
                completed_as_seller: 0,
                volume_as_buyer: Decimal::ZERO,
                volume_as_seller: Decimal::ZERO,
                channel_price: price,
                nostr_pub_key,
                deposit_amount
            };
            
            let ticket = self.ticket_res_mgr.mint_non_fungible(
                &NonFungibleLocalId::from(self.ticket_id_counter),
                data
            );
            self.ticket_id_counter += 1;
            ticket
        }
    }
}
