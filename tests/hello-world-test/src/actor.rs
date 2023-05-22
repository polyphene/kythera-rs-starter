use cid::{multihash::Code, Cid};
use frc42_dispatch::match_method;
use frc42_dispatch::method_hash;
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_ipld_encoding::{de::DeserializeOwned, RawBytes};
use fvm_sdk::sys::ErrorNumber;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::address::Address;
use fvm_shared::bigint::Zero;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::sys::SendFlags;
use serde::ser;
use thiserror::Error;

/**************************************************
 * Actor's state
 **************************************************/

#[derive(Serialize_tuple, Deserialize_tuple)]
struct ActorState {
    value: u32,
}

impl ActorState {
    pub fn load(cid: &Cid) -> Self {
        let data = fvm_sdk::ipld::get(cid).unwrap();
        fvm_ipld_encoding::from_slice::<Self>(&data).unwrap()
    }

    pub fn save(&self) -> Cid {
        let serialized = fvm_ipld_encoding::to_vec(self).unwrap();
        let block = Block {
            codec: DAG_CBOR,
            data: serialized,
        };
        fvm_sdk::ipld::put(
            Code::Blake2b256.into(),
            32,
            block.codec,
            block.data.as_ref(),
        )
        .unwrap()
    }
}

/**************************************************
 * IPLD Utils
 **************************************************/

/// Deserialize message parameters into given struct.
pub fn deserialize_params<D: DeserializeOwned>(params: u32) -> D {
    let params = fvm_sdk::message::params_raw(params)
        .expect("Could not get message parameters")
        .expect("Expected message parameters but got none");

    let params = RawBytes::new(params.data);

    params
        .deserialize()
        .expect("Should be able to deserialize message params into arguments of called method")
}

#[derive(Error, Debug)]
enum IpldError {
    #[error("ipld encoding error: {0}")]
    Encoding(#[from] fvm_ipld_encoding::Error),
    #[error("ipld blockstore error: {0}")]
    Blockstore(#[from] ErrorNumber),
}

fn return_ipld<T>(value: &T) -> std::result::Result<u32, IpldError>
where
    T: ser::Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(fvm_sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
}

#[no_mangle]
fn invoke(input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        fvm_sdk::vm::exit(
            ExitCode::USR_ASSERTION_FAILED.value(),
            None,
            Some(&format!("{info}")),
        )
    }));

    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
                NO_DATA_BLOCK_ID
            },
            "Setup" => {
                Setup();
                NO_DATA_BLOCK_ID
            },
            "TestConstructorSetup" => {
                TestConstructorSetup();
                NO_DATA_BLOCK_ID
            },
            "TestMethodParameter" => {
                TestMethodParameter(input)
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
    let state = ActorState { value: 1 };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn Setup() {
    let mut state = ActorState::load(&fvm_sdk::sself::root().unwrap());
    state.value += 1;
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

// Tests that both the `Constructor` and the `Setup` method are called by Kythera `Tester`.
#[allow(non_snake_case)]
fn TestConstructorSetup() {
    let state = ActorState::load(&fvm_sdk::sself::root().unwrap());
    let value = state.value;
    if state.value != 2u32 {
        fvm_sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("value is different was not called {value}")),
        )
    }
}

// Tests that the target actor Id is properly passed to test methods. At the same time, it also ensures
// that `Constructor` is called on target actors as the value we are expecting is initialized there.
#[allow(non_snake_case)]
fn TestMethodParameter(input: u32) -> u32 {
    let target_actor_id: u64 = deserialize_params(input);

    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("HelloWorld"),
        None,
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let who_are_you: String = RawBytes::new(
        res.return_data
            .expect("Should be able to get result from HelloWorld of target actor")
            .data,
    )
    .deserialize()
    .unwrap();

    assert_eq!(who_are_you, String::from("Basic Target Actor"));

    return_ipld(&target_actor_id).unwrap()
}
