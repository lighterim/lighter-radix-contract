use scrypto::prelude::*;
use scrypto_test::prelude::*;

use lighter_radix_contract::blueprint::lighter_radix_test;

#[test]
fn test_take_ticket() {
    // Setup the environment
    let mut ledger = LedgerSimulatorBuilder::new().build();

    // Create an account
    let (public_key, _private_key, account) = ledger.new_allocated_account();

    // Publish package
    let package_address = ledger.compile_and_publish(this_package!());

    let price = Decimal::from(10);
    let window: u16 = 10;
    // Test the `instantiate_hello` function.
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            package_address,
            "Lighter",
            "instantiate",
            manifest_args!(price, window),
        )
        .call_method(account, "deposit_batch", manifest_args!(ManifestExpression::EntireWorktop))
        .build();

    let receipt = ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );
    println!("{:?}\n", receipt);
    // info!("{}\n", receipt);
    let comp_result = receipt.expect_commit(true);
    let component = comp_result.new_component_addresses()[0];
    // let ticket_bucket = comp_result.new_resource_addresses()[0];

    let builder = ManifestBuilder::new();
    // let bucket = builder.generate_bucket_name("bucket");
    let manifest = builder
    .lock_fee_from_faucet()
    .withdraw_from_account(account, XRD, Decimal::from(20))
    .take_from_worktop(XRD, price, "bucket1")
    .with_name_lookup(|bld, lookup|{
        bld.call_method(component, "take_ticket", manifest_args!("buyer@lighter.im", lookup.bucket("bucket1")),)
    })
    .call_method(account, "deposit_batch", manifest_args!(ManifestExpression::EntireWorktop))
    .build();
    let receipt = ledger.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&public_key)]);
    let buyer_result = receipt.expect_commit(true);
    // let buyer = buyer_result.new_resource_addresses()[0];

    
    // println!("{:?}\n {}\n", receipt, Runtime::bech32_encode_address(buyer));
    // info!("{}\n", receipt);
    // receipt.expect_commit_success();
    // receipt.expect_commit(true);
}

#[test]
fn test_hello_with_test_environment() -> Result<(), RuntimeError> {
    // // Arrange
    // let mut env = TestEnvironment::new();
    // let package_address = 
    //     PackageFactory::compile_and_publish(this_package!(), &mut env, CompileProfile::Fast)?;

    // let mut lighter = Lighter::instantiate(package_address, &mut env)?;

    // // Act
    // let bucket = hello.free_token(&mut env)?;

    // // Assert
    // let amount = bucket.amount(&mut env)?;
    // assert_eq!(amount, dec!("1"));

    Ok(())
}
