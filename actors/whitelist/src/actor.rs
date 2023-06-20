use std::collections::HashMap;
use fvm_shared::address::Address;
use frc42_dispatch::match_method;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

use crate::utils;

#[no_mangle]
fn invoke(input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
                NO_DATA_BLOCK_ID
            },
            "SetAdministrator" => {
                SetAdministrator(input);
                NO_DATA_BLOCK_ID
            },
            "SetWhitelist" => {
                SetWhitelist(input);
                NO_DATA_BLOCK_ID
            },
            "IsWhitelisted" => {
                IsWhitelisted(input)
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
fn Constructor() {
    let initial_state = utils::ActorState {
        admin: Address::new_id(0),
        whitelist: HashMap::new()
    };

    let cid = initial_state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn SetAdministrator(input: u32) {
    let new_administrator: Address = utils::deserialize_params(input);

    let mut current_state: utils::ActorState = utils::ActorState::load(&fvm_sdk::sself::root().unwrap());

    assert_eq!(current_state.admin.id().unwrap(), 0, "Adminstrator is already set");

    current_state.admin = new_administrator;

    let cid = current_state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn IsWhitelisted(input: u32) -> u32 {
    let address: Address = utils::deserialize_params(input);

    let current_state: utils::ActorState = utils::ActorState::load(&fvm_sdk::sself::root().unwrap());

    return match current_state.whitelist.get(&address) {
        Some(boolean) => {
            utils::return_ipld(boolean).unwrap()
        }
        _ => utils::return_ipld(&false).unwrap()
    }
}

#[allow(non_snake_case)]
fn SetWhitelist(input: u32) {
    let (address, whitelist): (Address, bool) = utils::deserialize_params(input);

    let mut current_state: utils::ActorState = utils::ActorState::load(&fvm_sdk::sself::root().unwrap());

    current_state.whitelist.insert(address, whitelist);

    let cid = current_state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}