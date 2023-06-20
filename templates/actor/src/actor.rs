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
fn invoke(_input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
                NO_DATA_BLOCK_ID
            },
            // TODO add your entry point as match variant
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
fn Constructor() {
    // TODO add Constructor logic
}