use scrypto::prelude::*;
use crate::ticket::TicketData;
use crate::escrow::*;
use crate::utils::*;

#[blueprint]
#[events(TakeTicketEvent, CreateEscrowEvent, BuyerPaidEvent, SellerReleasedEvent, SellerRequestCancelEvent, SellerCancelEvent, WithdrawByCreditEvent)]
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
            buyer_cancel => PUBLIC;
            buyer_paid => PUBLIC;
            withdraw => PUBLIC;
            seller_release => PUBLIC;
            seller_request_cancel => PUBLIC;
            seller_cancel => PUBLIC;
            //Temp
            get_credit => PUBLIC;
            get_escrow => PUBLIC;
        }
    }

    struct Lighter {
        ///
        /// public key of Lighter relay
        relay_public_key: Ed25519PublicKey,
        ///
        /// lighter.im
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
        /// escrow_res_mgr: ResourceManager,
        /// escrow map
        escrow_map: KeyValueStore<Hash, EscrowData>,
        ///
        /// escrow NFT vault
        // escrow_nft_vault: NonFungibleVault,
        ///
        /// asset vault by escrow
        escrow_vault_map: KeyValueStore<ResourceAddress, Vault>,
        ///
        /// credit for buyer
        user_credit: KeyValueStore<ResourceAddress, KeyValueStore<NonFungibleLocalId, Decimal>>,
        ///
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

            // let escrow_res_mgr = ResourceBuilder::new_bytes_non_fungible::<EscrowData>(OwnerRole::None)
            //     .metadata(metadata!(init{
            //         "symbol" => "ESCR", locked;
            //         "name" => "Lighter Escrow Token", locked;
            //     }))
            //     .mint_roles(mint_roles!( 
            //         minter => rule!(require(global_caller(component_address)));
            //         minter_updater => rule!(deny_all);
            //     ))
            //     .burn_roles(burn_roles!(
            //         burner => rule!(require(global_caller(component_address)));
            //         burner_updater => rule!(deny_all);
            //     ))
            //     .non_fungible_data_update_roles(non_fungible_data_update_roles!(
            //         non_fungible_data_updater => rule!(require(global_caller(component_address)));
            //         non_fungible_data_updater_updater => rule!(deny_all);
            //     ))
            //     .create_with_no_initial_supply();

            let relay_public_key = Ed25519PublicKey::from_str(&relay_pub_key).unwrap();
            let component = Self{
                admin_rule: rule!(require(admin_badge_addr)),
                op_rule: rule!(require(op_badge_addr)),
                ticket_vault: Vault::new(XRD),
                // escrow_nft_vault: NonFungibleVault::new(escrow_res_mgr.address()),
                escrow_map: KeyValueStore::new(),
                escrow_vault_map: KeyValueStore::new(),
                user_credit: KeyValueStore::new(),
                user_escrow: KeyValueStore::new(),
                relay_domain_name,
                relay_public_key,
                channel_price,
                ticket_res_mgr,
                // escrow_res_mgr,
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
        pub fn take_ticket(&mut self, nostr_nip_05: String, nostr_pub_key: String, mut bucket: Bucket) -> (Bucket, Bucket){
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
                volume_as_buyer: Decimal::zero(),
                volume_as_seller: Decimal::zero(),
                avg_paid_epochs: Decimal::zero(),
                avg_release_epochs: Decimal::zero(),
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
                nostr_nip_05: nostr_nip_05.clone(),
                nostr_pub_key,
                nft_id
            });
            (ticket,bucket)
        }


        fn add_pending_trade(& self, ticket_id: &NonFungibleLocalId, trade_id: u64){
            let ticket = self.ticket_res_mgr.get_non_fungible_data::<TicketData>(ticket_id);
            let cap = ticket.deposit_amount.checked_div(ticket.channel_price).unwrap().checked_ceiling().unwrap();
            assert!(Decimal::from(ticket.pending_order_ids.len() + 1) <= cap, "the ticket({}) reach the maximum number of parallel transactions.", ticket_id);
            let mut pending_order_ids = ticket.pending_order_ids;
            pending_order_ids.push(trade_id);
            self.ticket_res_mgr.update_non_fungible_data(ticket_id, "pending_order_ids", pending_order_ids);
        }

        fn update_ticket(&self, ticket_id: &NonFungibleLocalId, trade_id: u64){
            let ticket = self.ticket_res_mgr.get_non_fungible_data::<TicketData>(ticket_id);
            // pending order id
            let mut pending_order_ids = ticket.pending_order_ids;
            if let Ok(index) = pending_order_ids.binary_search(&trade_id){
                pending_order_ids.remove(index);
                self.ticket_res_mgr.update_non_fungible_data(ticket_id, "pending_order_ids", pending_order_ids);
            }
        }

        fn update_buyer_when_cancel(&self, buyer: &NonFungibleLocalId, trade_id: u64){
            let ticket = self.ticket_res_mgr.get_non_fungible_data::<TicketData>(buyer);
            // pending order id
            let mut pending_order_ids = ticket.pending_order_ids;
            if let Ok(index) = pending_order_ids.binary_search(&trade_id){
                pending_order_ids.remove(index);
                self.ticket_res_mgr.update_non_fungible_data(buyer, "pending_order_ids", pending_order_ids);
            }

            self.ticket_res_mgr.update_non_fungible_data(buyer, "cancel_as_buyer", ticket.cancel_as_buyer+1);
        }

        fn update_buyer(&self, buyer: &NonFungibleLocalId, trade_id: u64, amount:Decimal, paid_epochs: u64){
            let ticket = self.ticket_res_mgr.get_non_fungible_data::<TicketData>(buyer);
            // pending order id
            let mut pending_order_ids = ticket.pending_order_ids;
            if let Ok(index) = pending_order_ids.binary_search(&trade_id){
                pending_order_ids.remove(index);
                self.ticket_res_mgr.update_non_fungible_data(buyer, "pending_order_ids", pending_order_ids);
            }

            // paid epochs
            let completed_as_buyer = ticket.completed_as_buyer;
            let trade_amount = ticket.volume_as_buyer;
            let avg_paid_epochs = Decimal::from(ticket.avg_paid_epochs).checked_mul(completed_as_buyer).and_then(
                |value| value.checked_add(paid_epochs).and_then(
                    |sum| sum.checked_div(
                        Decimal::from(completed_as_buyer+1).checked_round(6, RoundingMode::ToNearestMidpointToEven).unwrap()
                    )
                )
            ).unwrap_or(ticket.avg_paid_epochs);
            self.ticket_res_mgr.update_non_fungible_data(buyer, "avg_paid_epochs", avg_paid_epochs);
            self.ticket_res_mgr.update_non_fungible_data(buyer, "completed_as_buyer", completed_as_buyer+1);
            self.ticket_res_mgr.update_non_fungible_data(buyer, "volume_as_buyer", trade_amount.checked_add(amount).unwrap());
        }

        fn update_seller(&self, seller: &NonFungibleLocalId, trade_id: u64, amount: Decimal, release_epochs: u64){
            let ticket = self.ticket_res_mgr.get_non_fungible_data::<TicketData>(seller);
            // pending order id
            let mut pending_order_ids = ticket.pending_order_ids;
            if let Ok(index) = pending_order_ids.binary_search(&trade_id){
                pending_order_ids.remove(index);
                self.ticket_res_mgr.update_non_fungible_data(seller, "pending_order_ids", pending_order_ids);
            }

            //release epochs
            let completed_as_seller = ticket.completed_as_seller;
            let trade_amount = ticket.volume_as_seller;
            let avg_release_epochs = Decimal::from(ticket.avg_release_epochs).checked_mul(completed_as_seller).and_then(
                |value| value.checked_add(release_epochs).and_then(
                    |sum| sum.checked_div(
                        Decimal::from(completed_as_seller+1).checked_round(6, RoundingMode::ToNearestMidpointToEven).unwrap()
                    )
                )
            ).unwrap_or(ticket.avg_paid_epochs);
            self.ticket_res_mgr.update_non_fungible_data(seller, "avg_paid_epochs", avg_release_epochs);
            self.ticket_res_mgr.update_non_fungible_data(seller, "completed_as_seller", completed_as_seller+1);
            self.ticket_res_mgr.update_non_fungible_data(seller, "volume_as_seller", trade_amount.checked_add(amount).unwrap());
            
        }

        fn update_ticket_when_release(& self, trade_id: u64, amount: Decimal, buyer: &NonFungibleLocalId, paid_epochs: u64, seller: &NonFungibleLocalId, release_epochs: u64){
            
            self.update_buyer(buyer, trade_id, amount, paid_epochs);
            self.update_seller(seller, trade_id, amount, release_epochs);
        }

        fn update_ticket_when_cancel(&self, trade_id: u64, buyer: &NonFungibleLocalId, seller: &NonFungibleLocalId){
            self.update_ticket(seller, trade_id);
            self.update_buyer_when_cancel(buyer, trade_id);
        }

        fn increase_buyer_credit(&mut self, token_addr: ResourceAddress, credit_amount: Decimal, buyer: &NonFungibleLocalId){
            // increase seller escrow
            if self.user_credit.get(&token_addr).is_some(){
                let mut user_credit_kv = self.user_credit.get_mut(&token_addr).unwrap();
                if user_credit_kv.get(buyer).is_some(){
                    let mut current = user_credit_kv.get_mut(buyer).unwrap();
                    *current = current.checked_add(credit_amount).unwrap();
                }
                else{
                    user_credit_kv.insert(buyer.clone(), credit_amount);
                    info!("{} credit: {}", buyer.clone(), credit_amount)
                }
            }
            else {
                let user_credit_kv = KeyValueStore::new();
                user_credit_kv.insert(buyer.clone(), credit_amount);
                self.user_credit.insert(token_addr.clone(), user_credit_kv);
            }
        }

        fn increase_seller_escrow(&mut self, token_addr: ResourceAddress, volume:Decimal, seller: &NonFungibleLocalId){
            // increase seller escrow
            if self.user_escrow.get(&token_addr).is_some(){
                let mut user_escrow_kv = self.user_escrow.get_mut(&token_addr).unwrap();
                if user_escrow_kv.get(seller).is_some(){
                    let mut current = user_escrow_kv.get_mut(seller).unwrap();
                    *current = current.checked_add(volume).unwrap();
                }
                else{
                    user_escrow_kv.insert(seller.clone(), volume);
                }
            }
            else {
                let token_escrow_map = KeyValueStore::new();
                token_escrow_map.insert(seller.clone(), volume);
                self.user_escrow.insert(token_addr.clone(), token_escrow_map);
            }
        }

        fn reduce_seller_escrow(&mut self, token_addr: &ResourceAddress, volume:Decimal, seller: &NonFungibleLocalId){
            // increase seller escrow
            if self.user_escrow.get(&token_addr).is_some(){
                let mut user_escrow_kv = self.user_escrow.get_mut(&token_addr).unwrap();
                if user_escrow_kv.get(seller).is_some(){
                    let mut current = user_escrow_kv.get_mut(seller).unwrap();
                    *current = current.checked_sub(volume).unwrap();
                    //TODO: remove
                }
            }
        }

        pub fn create_escrow(
            &mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            buyer_fee: Decimal,
            price: Decimal,
            volume: Decimal,
            currency: String,
            usd_rate: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            payee: String,
            signature: String,
            seller_ticket: NonFungibleBucket,
            mut token_bucket: Bucket
        ) -> (NonFungibleBucket, Bucket){
            assert!(seller_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            
            let amount = token_bucket.amount();
            let token_addr = token_bucket.resource_address();
            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, currency.clone(), usd_rate, buyer_fee, seller_fee, payment_method.clone(), payee.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "invalid escrow data.{}|{}", &args, &signature);
            
            let total = get_total_by_bp(volume, seller_fee);
            assert!(amount >= total, "escrow amount less than required.{}", total);
            let escrow_bucket = token_bucket.take_advanced(total, WithdrawStrategy::Rounded(RoundingMode::ToPositiveInfinity));
            // collect for seller escrow
            if self.escrow_vault_map.get(&token_addr).is_some(){
                self.escrow_vault_map.get_mut(&token_addr).unwrap().put(escrow_bucket);
            }
            else{
                self.escrow_vault_map.insert(token_addr, Vault::with_bucket(escrow_bucket));
            }
            
            self.increase_seller_escrow(token_addr, volume, &seller);
            self.add_pending_trade(&buyer, trade_id);
            self.add_pending_trade(&seller, trade_id);

            // let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            let escrow_data = EscrowData{
                last_epoch: Runtime::current_epoch().number(),
                paid_epochs: 0u64,
                request_cancel_epoch: 0u64,
                instruction: Instruction::Escrowed,
                gas_spent_by_relayer: Decimal::ZERO
            };
            self.escrow_map.insert(h.clone(), escrow_data);
            // self.escrow_nft_vault.put(
            //     self.escrow_res_mgr.mint_non_fungible(&escrow_nft_id, escrow_data).as_non_fungible()
            // );

            Runtime::emit_event(CreateEscrowEvent{
                payment_method:payment_method.clone(),
                currency: currency.clone(),
                escrow_id: h.to_string(),
                amount,
                seller,
                token_addr,
                trade_id,
                buyer,
                price,
                usd_rate,
                volume,
                buyer_fee,
                seller_fee
            });
            (seller_ticket, token_bucket)
        }

        pub fn buyer_paid(&mut self, 
            trade_id: u64,
            seller: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal,
            currency: String,
            usd_rate: Decimal,
            seller_fee: Decimal,
            buyer_fee: Decimal,
            payment_method: String,
            payee: String,
            signature: String,
            buyer_ticket: NonFungibleBucket
        ) -> NonFungibleBucket {
            assert!(buyer_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            let buyer = buyer_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, currency.clone(), usd_rate, buyer_fee, seller_fee, payment_method.clone(), payee.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "invalid escrow data:{} | {}", &args, &signature);
            
            // let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            // let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let mut escrow_data = self.escrow_map.get_mut(&h).unwrap();
            assert!(escrow_data.instruction == Instruction::Escrowed || escrow_data.instruction == Instruction::SellerRequestCancel, "current status not support turn to BuyerPaid!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            let current_epoch = Runtime::current_epoch().number();
            escrow_data.paid_epochs = current_epoch - escrow_data.last_epoch;
            escrow_data.instruction = Instruction::BuyerPaid;
            escrow_data.last_epoch = current_epoch;

            Runtime::emit_event(BuyerPaidEvent{
                payment_method:payment_method.clone(),
                currency: currency.clone(),
                escrow_id: h.to_string(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                usd_rate,
                volume,
                buyer_fee,
                seller_fee
            });
            
            buyer_ticket
        }

        pub fn buyer_cancel(&mut self,
            trade_id: u64,
            seller: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal,
            currency: String,
            usd_rate: Decimal,
            seller_fee: Decimal,
            buyer_fee: Decimal,
            payment_method: String,
            payee: String,
            signature: String,
            buyer_ticket: NonFungibleBucket
        ) -> NonFungibleBucket{
            assert!(buyer_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            let buyer = buyer_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, currency.clone(), usd_rate, buyer_fee, seller_fee, payment_method.clone(), payee.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "invalid escrow data:{} | {}", &args, &signature);
            
            // let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            // let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let mut escrow_data = self.escrow_map.get_mut(&h).unwrap();
            assert!(escrow_data.instruction == Instruction::Escrowed 
                || escrow_data.instruction == Instruction::SellerRequestCancel 
                || escrow_data.instruction == Instruction::BuyerPaid, "current status not support turn to BuyerCancelled!");
            // self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::BuyerPaid);
            escrow_data.instruction = Instruction::BuyerCancelled;
            escrow_data.last_epoch = Runtime::current_epoch().number();
            Runtime::emit_event(BuyerPaidEvent{
                payment_method:payment_method.clone(),
                currency: currency.clone(),
                escrow_id: h.to_string(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                usd_rate,
                buyer_fee,
                seller_fee
            });
            //TODO: update the buyer cancel.
            buyer_ticket
        }

        pub fn seller_release(&mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal,
            currency: String,
            usd_rate: Decimal,
            buyer_fee: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            payee: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> NonFungibleBucket{
            assert!(seller_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");

            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, currency.clone(), usd_rate, buyer_fee, seller_fee, payment_method.clone(), payee.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");
            // let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            
            let actual_credit = get_net_by_bp(volume, buyer_fee);
            self.reduce_seller_escrow(&token_addr, volume, &seller);
            self.increase_buyer_credit(token_addr, actual_credit, &buyer);
            
            let (paid_epochs, release_epochs) = self.update_escrow_when_release(&h);
            let usd_amount = price.checked_mul(volume).and_then(
                |currency_amount| currency_amount.checked_mul(usd_rate)
            ).unwrap_or(Decimal::zero());
            self.update_ticket_when_release(trade_id, usd_amount, &buyer, paid_epochs, &seller, release_epochs);
            

            Runtime::emit_event(SellerReleasedEvent{
                payment_method:payment_method.clone(),
                escrow_id: h.to_string(),
                currency: currency.clone(),
                buyer_credit: actual_credit,
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                usd_rate,
                buyer_fee,
                seller_fee
            });
            seller_ticket
        }

        fn update_escrow_when_release(&mut self, h: &Hash) -> (u64, u64){
            // let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let mut escrow_data = self.escrow_map.get_mut(h).unwrap();
            assert!(escrow_data.instruction == Instruction::BuyerPaid || escrow_data.instruction == Instruction::SellerRequestCancel, "current status not support turn to Release!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            // self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::Release);
            let release_epochs = Runtime::current_epoch().number() - escrow_data.last_epoch;
            escrow_data.instruction = Instruction::Released;
            escrow_data.last_epoch = 0u64;
            (escrow_data.paid_epochs, release_epochs)
        }

        //TEMP
        pub fn get_credit(&self, token_addr: ResourceAddress, user_id: NonFungibleLocalId) -> Decimal{
            if self.user_credit.get(&token_addr).is_some_and(|kv| kv.get(&user_id).is_some()){
                return *self.user_credit.get(&token_addr).unwrap().get(&user_id).unwrap();
            }
            Decimal::zero()
        }

        //TEMP
        pub fn get_escrow(&self, token_addr: ResourceAddress, user_id: NonFungibleLocalId) -> Decimal{
            if self.user_escrow.get(&token_addr).is_some_and(|kv| kv.get(&user_id).is_some()){
                return *self.user_escrow.get(&token_addr).unwrap().get(&user_id).unwrap();
            }
            Decimal::zero()
        }

        pub fn withdraw(&mut self, token_addr: ResourceAddress, amount: Decimal, ticket: NonFungibleBucket) ->(NonFungibleBucket, Bucket){
            assert!(ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            let user_id = ticket.non_fungible_local_id();
            assert!(self.user_credit.get(&token_addr).is_some_and(|kv| kv.get(&user_id).is_some()), "the user:{} has not exist credit!", &user_id);

            let mut kv = self.user_credit.get_mut(&token_addr).unwrap();
            let mut credit = kv.get_mut(&user_id).unwrap();
            assert!(*credit >= amount, "credit insuffice");
            *credit = *credit - amount;
            let bucket = self.escrow_vault_map.get_mut(&token_addr).unwrap().take_advanced(amount, WithdrawStrategy::Rounded(RoundingMode::ToNegativeInfinity));
            Runtime::emit_event(WithdrawByCreditEvent{
                ticket_id: user_id,
                token_addr,
                amount
            });
            (ticket, bucket)
        }

        pub fn seller_request_cancel(&mut self, 
            trade_id: u64,
            buyer: NonFungibleLocalId,
            token_addr: ResourceAddress,
            volume: Decimal,
            price: Decimal,
            currency: String,
            usd_rate: Decimal,
            buyer_fee: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            payee: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> NonFungibleBucket{
            assert!(seller_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, currency.clone(), usd_rate, buyer_fee, seller_fee, payment_method.clone(), payee.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");
            
            // let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            // let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let mut escrow_data = self.escrow_map.get_mut(&h).unwrap();
            let current_epoch = Runtime::current_epoch().number();
            assert!(escrow_data.instruction == Instruction::Escrowed && escrow_data.last_epoch + (self.payment_window_epochs as u64) <= current_epoch , "current status not support turn to SellerCancel!");    //"current status:{} not support turn to BuyerPaid!", escrow_data.instruction);
            escrow_data.request_cancel_epoch = current_epoch;
            escrow_data.instruction = Instruction::SellerRequestCancel;
            // self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "instruction", Instruction::SellerRequestCancel);
            // self.escrow_res_mgr.update_non_fungible_data(&escrow_nft_id, "cancel_after_epoch_by_seller", current_epoch + (self.payment_window_epochs as u64));
            Runtime::emit_event(SellerRequestCancelEvent{
                payment_method:payment_method.clone(),
                escrow_id: h.to_string(),
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
            currency: String,
            usd_rate: Decimal,
            buyer_fee: Decimal,
            seller_fee: Decimal,
            payment_method: String,
            payee: String,
            signature: String,
            seller_ticket: NonFungibleBucket
        ) -> (NonFungibleBucket, Bucket){
            assert!(seller_ticket.resource_address() == self.ticket_res_mgr.address(), "invalid ticket.");
            let seller = seller_ticket.non_fungible_local_id();
            let str_res_addr = Runtime::bech32_encode_address(token_addr);
            let args = format!("{},{},{},{},{},{},{},{},{},{},{},{}", trade_id, buyer, seller, str_res_addr, volume, price, currency.clone(), usd_rate, buyer_fee, seller_fee, payment_method.clone(), payee.clone());
            info!("args:{}", &args);
            let h =  keccak256_hash(args.clone());
            let sig = Ed25519Signature::from_str(&signature).unwrap();
            assert!(verify_ed25519(&h, &self.relay_public_key, &sig), "illegal escrow data");

            self.update_ticket_when_cancel(trade_id, &buyer, &seller);
            
            // let escrow_nft_id = NonFungibleLocalId::bytes(h.as_bytes()).unwrap();
            // let escrow_data = self.escrow_res_mgr.get_non_fungible_data::<EscrowData>(&escrow_nft_id);
            let mut escrow_data = self.escrow_map.get_mut(&h).unwrap();
            let current_epoch = Runtime::current_epoch().number();
            let timeout = escrow_data.instruction == Instruction::SellerRequestCancel && escrow_data.request_cancel_epoch + (self.payment_window_epochs as u64) <= current_epoch;
            assert!(timeout || escrow_data.instruction == Instruction::BuyerCancelled, "current status not support turn to SellerCancel!");
            escrow_data.instruction = Instruction::SellerCancelled;
            escrow_data.last_epoch = 0u64;
            
            
            
            let total = get_total_by_bp(volume, seller_fee);
            let mut vault = self.escrow_vault_map.get_mut(&token_addr).unwrap();
            let escrow_bucket = vault.take_advanced(total, WithdrawStrategy::Rounded(RoundingMode::ToNegativeInfinity));
            // self.reduce_seller_escrow(&token_addr, volume, &seller, seller_fee);

            // increase seller escrow
            if self.user_escrow.get(&token_addr).is_some(){
                let mut user_escrow_kv = self.user_escrow.get_mut(&token_addr).unwrap();
                if user_escrow_kv.get(&seller).is_some(){
                    let mut current = user_escrow_kv.get_mut(&seller).unwrap();
                    *current = current.checked_sub(volume).unwrap();
                    //TODO: remove
                }
            }
            

            Runtime::emit_event(SellerCancelEvent{
                payment_method:payment_method.clone(),
                escrow_id: h.to_string(),
                amount: escrow_bucket.amount(),
                token_addr,
                trade_id,
                buyer,
                seller,
                price,
                volume,
                buyer_fee,
                seller_fee
            });
            (seller_ticket, escrow_bucket)
        }
    }
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct TakeTicketEvent{
    nostr_nip_05: String,
    nostr_pub_key: String,
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
    currency: String,
    usd_rate: Decimal,
    volume: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    amount: Decimal,
    escrow_id: String
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct BuyerPaidEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    currency: String,
    usd_rate: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: String
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct SellerReleasedEvent{
    trade_id: u64,
    buyer: NonFungibleLocalId,
    seller: NonFungibleLocalId,
    token_addr: ResourceAddress,
    price: Decimal,
    volume: Decimal,
    currency: String,
    usd_rate: Decimal,
    buyer_credit: Decimal,
    buyer_fee: Decimal,
    seller_fee: Decimal,
    payment_method: String,
    escrow_id: String
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
    escrow_id: String
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
    amount: Decimal,
    escrow_id: String
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct WithdrawByCreditEvent{
    ticket_id: NonFungibleLocalId,
    token_addr: ResourceAddress,
    amount: Decimal
}