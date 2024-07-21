use scrypto::prelude::*;
use crate::ticket::TicketData;
use crate::escrow::*;

#[blueprint]
#[events(TakeTicketEvent, CreateEscrowEvent, BuyerPaidEvent, SellerReleasedEvent, SellerRequestCancelEvent)]
mod lighter_radix{
    enable_method_auth! {
        roles{
            admin => updatable_by: [];
            operator => updatable_by: [admin];
        },
        methods {
            // new_pool => restrict_to: [admin, OWNER];
            take_ticket => PUBLIC;
            create_escrow => PUBLIC;
            buyer_paid => PUBLIC;
            seller_release => PUBLIC;
            seller_request_cancel => PUBLIC;
            seller_cancel => PUBLIC;
        }
    }

    struct Lighter {
        ///
        /// public key of Lighter relay
        relay_public_key: Ed25519PublicKey,
        ///
        /// 
        relay_domain_name: String,
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
        /// ResourceManager of escrow NFT.
        escrow_res_mgr: ResourceManager,
        ///
        /// escrow NFT vault
        escrow_nft_vault: NonFungibleVault,
        ///
        /// asset vault by escrow
        escrow_vault_map: KeyValueStore<ResourceAddress, Vault>,
        ///
        /// credit for buyer
        user_credit: KeyValueStore<ResourceAddress, KeyValueStore<NonFungibleLocalId, Decimal>>,
        /// escrow for seller.
        user_escrow: KeyValueStore<ResourceAddress, KeyValueStore<NonFungibleLocalId, Decimal>>,
        ///
        /// payment window epoch
        payment_window_epochs: u16,

        admin_rule: AccessRule,
        op_rule: AccessRule
    }

    impl Lighter {

        pub fn instantiate(
            channel_price: Decimal, payment_window_epochs: u16, relay_pub_key:String,
            relay_domain_name: String
        ) -> (Global<Lighter>, Bucket, Bucket){
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

            let ticket_res_mgr = ResourceBuilder::new_string_non_fungible::<TicketData>(OwnerRole::None)
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

            let escrow_res_mgr = ResourceBuilder::new_bytes_non_fungible::<EscrowData>(OwnerRole::None)
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
                escrow_nft_vault: NonFungibleVault::new(escrow_res_mgr.address()),
                escrow_vault_map: KeyValueStore::new(),
                user_credit: KeyValueStore::new(),
                user_escrow: KeyValueStore::new(),
                relay_domain_name,
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
            // info!("xxxx");
            (component, admin_badge.into(), op_badge.into())
            
        }

        ///
        /// take an Lighter NFT with a nostr public key.
        pub fn take_ticket(&mut self, nostr_nip_05: String, mut bucket: Bucket) -> (Bucket, Bucket){
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
                deposit_amount
            };

            let username = nostr_nip_05.trim_end_matches(&self.relay_domain_name);
            let underscore = "_";
            let escape_domain = self.relay_domain_name.replace("@", underscore).replace(".", underscore);
            let mut ticket_id = String::from(username);
            ticket_id.push_str(&escape_domain);
            let nft_id = NonFungibleLocalId::string(ticket_id.clone()).ok().unwrap();
            assert!(!self.ticket_res_mgr.non_fungible_exists(&nft_id), "ticket id exists! {}", nostr_nip_05.clone());
            
            let ticket = self.ticket_res_mgr.mint_non_fungible(&nft_id,data);
            error!("nostr_nip_05:{}, processed_order_cnt:{}, nft_id:{}", nostr_nip_05.clone(), processed_order_cnt, &nft_id);
            Runtime::emit_event(TakeTicketEvent{
                channel_count: processed_order_cnt,
                nft_id: nft_id,
                nostr_nip_05: nostr_nip_05.clone()
            });
            (ticket,bucket)
        }
    
        pub fn create_escrow(
            &mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            buyer_fee: Decimal,
            price: Decimal, 
            seller_fee: Decimal,
            payment_method: String,
            signature: String,
            seller_ticket: NonFungibleBucket,
            token_bucket: Bucket
        ) -> (NonFungibleBucket, NonFungibleLocalId){
            assert!(seller_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            
            let volume = token_bucket.amount();
            let token_addr = token_bucket.resource_address();
            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, buyer_fee, seller_fee, payment_method.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "invalid escrow data.{}|{}", &args, &signature);

            if self.escrow_vault_map.get(&token_addr).is_some(){
                self.escrow_vault_map.get_mut(&token_addr).unwrap().put(token_bucket);
            }
            else{
                self.escrow_vault_map.insert(token_addr, Vault::with_bucket(token_bucket));
            }
            //TODO: user escrow
            // self.user_escrow.get_mut(&token_addr).is_some()

            let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            let escrow_data = EscrowData{
                instruction: Instruction::Escrowed,
                cancel_after_epoch_by_seller: Runtime::current_epoch().number() + (self.payment_window_epochs as u64),
                gas_spent_by_relayer: Decimal::ZERO
            };
            self.escrow_nft_vault.put(
                self.escrow_res_mgr.mint_non_fungible(&escrow_nft_id, escrow_data).as_non_fungible()
            );
            Runtime::emit_event(CreateEscrowEvent{
                payment_method:payment_method.clone(),
                escrow_id: escrow_nft_id.clone(),
                seller,
                token_addr,
                trade_id,
                buyer,
                price,
                volume,
                buyer_fee,
                seller_fee
            });
            (seller_ticket, escrow_nft_id)
        }

        pub fn buyer_paid(&mut self, 
            trade_id: u64,
            seller: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal, 
            seller_fee: Decimal,
            buyer_fee: Decimal,
            payment_method: String,
            signature: String,
            buyer_ticket: NonFungibleBucket
        ) -> NonFungibleBucket {
            assert!(buyer_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            let buyer = buyer_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, buyer_fee, seller_fee, payment_method.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(true || verify_ed25519(&h, &self.relay_public_key, &sig), "invalid escrow data:{} | {}", &args, &signature);
            let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            
            let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            assert!(escrow_data.instruction == Instruction::Escrowed || escrow_data.instruction == Instruction::SellerRequestCancel, "current status not support turn to BuyerPaid!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::BuyerPaid);
            Runtime::emit_event(BuyerPaidEvent{
                payment_method:payment_method.clone(),
                escrow_id: escrow_nft_id.clone(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                buyer_fee,
                seller_fee
            });
            //TODO: 更新付款时间. 
            buyer_ticket
        }

        pub fn seller_release(&mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal, 
            buyer_fee: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> NonFungibleBucket{
            assert!(seller_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");

            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, buyer_fee, seller_fee, payment_method.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");
            let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            // let escrow_bucket = self.escrow_nft_vault.take_non_fungible(&escrow_nft_id);
            // escrow_bucket.take(amount)
            let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            assert!(escrow_data.instruction == Instruction::Escrowed || escrow_data.instruction == Instruction::SellerRequestCancel, "current status not support turn to BuyerPaid!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::Release);
            //TODO: 为买家计帐
            Runtime::emit_event(SellerReleasedEvent{
                payment_method:payment_method.clone(),
                escrow_id: escrow_nft_id.clone(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                buyer_fee,
                seller_fee
            });
            seller_ticket
        }

        pub fn seller_request_cancel(&mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal,
            buyer_fee: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> NonFungibleBucket{
            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, buyer_fee, seller_fee, payment_method.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");
            
            let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let current_epoch = Runtime::current_epoch().number();
            assert!(escrow_data.instruction == Instruction::Escrowed && escrow_data.cancel_after_epoch_by_seller <= current_epoch , "current status not support turn to SellerCancel!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::SellerRequestCancel);
            self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "cancel_after_epoch_by_seller", current_epoch + (self.payment_window_epochs as u64));
            Runtime::emit_event(SellerRequestCancelEvent{
                payment_method:payment_method.clone(),
                escrow_id: escrow_nft_id.clone(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                buyer_fee,
                seller_fee
            });

            seller_ticket
        }

        pub fn seller_cancel(&mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal, 
            buyer_fee: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> NonFungibleBucket{
            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, buyer_fee, seller_fee, payment_method.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");
            
            let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let current_epoch = Runtime::current_epoch().number();
            assert!(escrow_data.instruction == Instruction::SellerRequestCancel && escrow_data.cancel_after_epoch_by_seller <= current_epoch , "current status not support turn to SellerCancel!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::SellerCancel);
            Runtime::emit_event(SellerCancelEvent{
                payment_method:payment_method.clone(),
                escrow_id: escrow_nft_id.clone(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                buyer_fee,
                seller_fee
            });
            seller_ticket
        }
    }
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct TakeTicketEvent{
    nostr_nip_05: String,
    nft_id: NonFungibleLocalId,
    channel_count: Decimal
} 

#[derive(ScryptoSbor, ScryptoEvent)]
struct CreateEscrowEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: NonFungibleLocalId
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct BuyerPaidEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: NonFungibleLocalId
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct SellerReleasedEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: NonFungibleLocalId
}
#[derive(ScryptoSbor, ScryptoEvent)]
struct SellerRequestCancelEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: NonFungibleLocalId
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct SellerCancelEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: NonFungibleLocalId
}