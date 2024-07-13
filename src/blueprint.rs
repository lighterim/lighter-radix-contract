use scrypto::prelude::*;
use crate::ticket::TicketData;
use crate::escrow::EscrowData;

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
            update_nostr_pub_key => PUBLIC;
            create_escrow => PUBLIC;
        }
    }

    struct Lighter {
        ///
        /// public key of Lighter relay
        relay_public_key: Ed25519PublicKey,
        ///
        /// ResourceManager for Ticket NFT. 
        ticket_res_mgr: ResourceManager,
        ///
        /// price per a trade channel on the Ticket.
        channel_price: Decimal,
        ///
        /// ticket vault
        ticket_vault: Vault,
        ///
        /// integer id of Ticket NFT.
        ticket_id_counter: u64,
        ///
        /// ResourceManager of escrow NFT.
        escrow_res_mgr: ResourceManager,
        ///
        /// escrow NFT vault
        escrow_vault: NonFungibleVault,
        ///
        /// payment window epoch
        payment_window_epochs: u16,

        admin_rule: AccessRule,
        op_rule: AccessRule
    }

    impl Lighter {

        pub fn instantiate(channel_price: Decimal, payment_window_epochs: u16, relay_pub_key:String) -> (Global<Lighter>, Bucket, Bucket){
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
                    "symbol" => "LTT", locked;
                    "name" => "Lighter Ticket Token", locked;
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

            let escrow_res_mgr = ResourceBuilder::new_integer_non_fungible::<EscrowData>(OwnerRole::None)
                .metadata(metadata!(init{
                    "symbol" => "ESCR", locked;
                    "name" => "Lighter Escrow Token", locked;
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

            let relay_public_key = Ed25519PublicKey::from_str(&relay_pub_key).unwrap();
            let component = Self{
                admin_rule: rule!(require(admin_badge_addr)),
                op_rule: rule!(require(op_badge_addr)),
                ticket_vault: Vault::new(XRD),
                escrow_vault: NonFungibleVault::new(escrow_res_mgr.address()),
                ticket_id_counter: 1,
                relay_public_key,
                channel_price,
                ticket_res_mgr,
                escrow_res_mgr,
                payment_window_epochs
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

        ///
        /// take an Lighter NFT with a nostr public key.
        pub fn take_ticket(&mut self, nostr_pub_key: String, mut bucket: Bucket) -> (Bucket, Bucket){
            assert!(bucket.resource_address() == XRD && bucket.amount() >= self.channel_price, "unknow resource bucket or invalid amount");
            // assert!(true, "nostr public key invalid");

            let processed_order_cnt = bucket.amount().checked_div(self.channel_price).map(|x| x.checked_floor().unwrap()).unwrap();
            info!("processed_order_cnt:{}, channel_price:{}", processed_order_cnt, self.channel_price);
            assert!(processed_order_cnt >= Decimal::one(), "The deposition is too small");
            // if processed_order_cnt > Decimal::from("10") {
            //     processed_order_cnt = Decimal::from("10");
            // }
            let deposit_amount = self.channel_price * processed_order_cnt;
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
            (ticket,bucket)
        }

        pub fn update_nostr_pub_key(&self, nostr_pub_key: String, bucket: NonFungibleBucket) -> NonFungibleBucket{
            let nft_id = bucket.non_fungible_local_id();
            // let ticket = self.ticket_res_mgr.get_non_fungible_data::<TicketData>(&nft_id);
            self.ticket_res_mgr.update_non_fungible_data(&nft_id, "nostr_pub_key", nostr_pub_key);
            bucket
        }
    
        pub fn create_escrow(
            &mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId, 
            volume: Decimal, 
            price: Decimal, 
            fee: Decimal,
            payment_method: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> NonFungibleLocalId{
            let seller = seller_ticket.non_fungible_local_id();
            let args = format!("{},{},{},{},{},{},{}", trade_id, buyer, seller, volume, price, fee, payment_method);
            let h =  keccak256_hash(args);
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");
            let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            let escrow_data = EscrowData{
                cancel_after_epoch_by_seller: Runtime::current_epoch().number() + (self.payment_window_epochs as u64),
                gas_spent_by_relayer: Decimal::ZERO
            };
            self.escrow_vault.put(
                self.escrow_res_mgr.mint_non_fungible(&escrow_nft_id, escrow_data).as_non_fungible()
            );
            escrow_nft_id
        }
    }
}
