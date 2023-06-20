use cid::{multihash::Code};
use std::collections::HashMap;
use frc42_dispatch::{method_hash, match_method};
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::{RawBytes, DAG_CBOR};
use fvm_ipld_encoding::ipld_block::IpldBlock;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::bigint::Zero;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::address::Address;
use fvm_shared::sys::SendFlags;
use serde_tuple::{Serialize_tuple};

use crate::utils;

#[no_mangle]
fn invoke(input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Setup" => {
                Setup();
                NO_DATA_BLOCK_ID
            },
            "TestFailNotAdmin" => {
                TestFailNotAdmin(input);
                NO_DATA_BLOCK_ID
            },
            "TestHappyPath" => {
                TestHappyPath(input);
                NO_DATA_BLOCK_ID
            },
            _ => {
                fvm_sdk::vm::abort(
                    ExitCode::USR_UNHANDLED_MESSAGE.value(),
                    Some("Unknown method number"),
                );
            }
        }
    )
}

#[allow(non_snake_case)]
fn Setup() {
    // TODO add Setup logic
}

/// Expect the test to fail as we are trying to whitelist while not being the admin
#[allow(non_snake_case)]
fn TestFailNotAdmin(input: u32) {
    let target_actor_id: u64 = utils::deserialize_params(input);

    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("SetWhitelist"),
        Some(
            IpldBlock::serialize(
                DAG_CBOR,
                &(Address::new_id(fvm_sdk::message::receiver()), true),
            )
                .unwrap(),
        ),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);
}

/// Expect the test to fail as we are trying to whitelist while not being the admin
#[allow(non_snake_case)]
fn TestHappyPath(input: u32) {
    let target_actor_id: u64 = utils::deserialize_params(input);

    set_target_admin(Address::new_id(target_actor_id), Address::new_id(fvm_sdk::message::receiver()));

    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("SetWhitelist"),
        Some(
            IpldBlock::serialize(
                DAG_CBOR,
                &(Address::new_id(fvm_sdk::message::receiver()), true),
            )
                .unwrap(),
        ),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("IsWhitelisted"),
        Some(
            IpldBlock::serialize(
                DAG_CBOR,
                &Address::new_id(fvm_sdk::message::receiver()),
            )
                .unwrap(),
        ),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let is_whitelisted: bool = RawBytes::new(
        res.return_data
            .expect("Should be able to get result from IsWhitelisted of target actor")
            .data,
    )
        .deserialize()
        .unwrap();

    assert!(is_whitelisted);
}


fn set_target_admin(target: Address,  address: Address) {
    #[derive(Serialize_tuple)]
    pub struct TargetState {
        pub(crate) admin: Address,
        pub(crate) whitelist: HashMap<Address, bool>
    }

    let new_state = TargetState {
        admin: address,
        whitelist: HashMap::new()
    };

    let serialized = fvm_ipld_encoding::to_vec(&new_state).unwrap();
    let block = Block {
        codec: DAG_CBOR,
        data: serialized,
    };

    let cid = fvm_sdk::ipld::put(
        Code::Blake2b256.into(),
        32,
        block.codec,
        block.data.as_ref(),
    )
        .unwrap();

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Alter"),
        Some(
            IpldBlock::serialize(
                DAG_CBOR,
                &(target, cid.to_string()),
            )
                .unwrap(),
        ),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);
}