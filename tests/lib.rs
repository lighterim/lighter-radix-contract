use scrypto::prelude::*;
use scrypto_test::prelude::*;

use lighter_radix_contract::blueprint::lighter_radix_test;

// #[test]
// fn test_take_ticket() {
//     // Setup the environment
//     let mut ledger = LedgerSimulatorBuilder::new().build();

//     // Create an account
//     let (public_key, _private_key, account) = ledger.new_allocated_account();

//     // Publish package
//     let package_address = ledger.compile_and_publish(this_package!());

//     let price = Decimal::from(10);
//     let window: u16 = 10;
//     let relay_pub_key = "6d187b0f2e66d74410e92e2dc92a5141a55c241646ce87acbcad4ab413170f9b";
//     let domain_name = "@lighter.im";
//     // Test the `instantiate_hello` function.
//     let manifest = ManifestBuilder::new()
//         .lock_fee_from_faucet()
//         .call_function(
//             package_address,
//             "Lighter",
//             "instantiate",
//             manifest_args!(price, window, relay_pub_key, domain_name),
//         )
//         .call_method(account, "deposit_batch", manifest_args!(ManifestExpression::EntireWorktop))
//         .build();

//     let receipt = ledger.execute_manifest(
//         manifest,
//         vec![NonFungibleGlobalId::from_public_key(&public_key)],
//     );
//     println!("{:?}\n", receipt);
//     // info!("{}\n", receipt);
//     let comp_result = receipt.expect_commit(true);
//     let component = comp_result.new_component_addresses()[0];
//     // let ticket_bucket = comp_result.new_resource_addresses()[0];

//     let builder = ManifestBuilder::new();
//     // let bucket = builder.generate_bucket_name("bucket");
//     let manifest = builder
//     .lock_fee_from_faucet()
//     .withdraw_from_account(account, XRD, Decimal::from(20))
//     .take_from_worktop(XRD, price, "bucket1")
//     .with_name_lookup(|bld, lookup|{
//         bld.call_method(component, "take_ticket", manifest_args!("buyer@lighter.im", lookup.bucket("bucket1")),)
//     })
//     .call_method(account, "deposit_batch", manifest_args!(ManifestExpression::EntireWorktop))
//     .build();
//     let receipt = ledger.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&public_key)]);
//     let buyer_result = receipt.expect_commit(true);
//     let buyer = buyer_result.new_resource_addresses()[0];

    
//     // println!("{:?}\n {}\n", receipt, Runtime::bech32_encode_address(buyer));
//     // info!("{}\n", receipt);
//     // receipt.expect_commit_success();
//     // receipt.expect_commit(true);
// }

#[test]
fn sign_and_verify() {
    
    // let test_sk = "0000000000000000000000000000000000000000000000000000000000000001";
    // let test_pk = "4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29";    
    // let test_signature = "cf0ca64435609b85ab170da339d415bbac87d678dfd505969be20adc6b5971f4ee4b4620c602bcbc34fd347596546675099d696265f4a42a16df343da1af980e";
    // let test_signature="a7241553838fdf6fa07045b5473f6b7b637dc56b7ef68628976c058ae0db10a231f5aea28634adf9c8f89cf78c6f57e0580d3cd7a4d3659eb59cbf515946d609";
    // let msg = "Test";
    let msg = "1,<dust_lighter_im>,<dust2_lighter_im>,resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3,1004,0.03,20,40,alipay";
    let test_sk = "$$$$$";                                                                      
    let test_pk = "a5bc3d9296bda1e52f96bf0a65238998877dbddb0703bd37ef1f18a6ffce458a";
    let test_signature = "9067f6160bcde97ccc697e53e093a0c8fc8ae6743bbd80cc432ea172aad2c01e07cf7222c0c29e4ba7f8c99c3d20b5febd40b424c5b7e47255f11f713ad4730b";

    let test_message_hash = keccak256_hash(msg);  //hash(msg);
    
    let sk = Ed25519PrivateKey::from_bytes(&hex::decode(test_sk).unwrap()).unwrap();
    let pk = Ed25519PublicKey::from_str(test_pk).unwrap();
    let sig = Ed25519Signature::from_str(test_signature).unwrap();

    
    // error!("sk:{}", sk.)
    assert_eq!(sk.public_key(), pk);
    assert_eq!(sk.sign(&test_message_hash), sig);
    assert!(verify_ed25519(&test_message_hash, &pk, &sig));
}
