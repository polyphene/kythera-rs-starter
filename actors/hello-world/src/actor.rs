use cid::{multihash::Code, Cid};
use frc42_dispatch::match_method;
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk::sys::ErrorNumber;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;
use serde::ser;
use thiserror::Error;

/**************************************************
 * Actor's state
 **************************************************/

#[derive(Serialize_tuple, Deserialize_tuple)]
struct ActorState {
    who_am_i: String,
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
fn invoke(_input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
                NO_DATA_BLOCK_ID
            },
            "HelloWorld" => {
                HelloWorld()
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

// `Constructor` for the target actor we are using in our tests.
#[allow(non_snake_case)]
fn Constructor() {
    // This value should always be set in the `Constructor`. It allows us to test that constructor
    // for target actors are properly called.
    let state = ActorState {
        who_am_i: String::from("Basic Target Actor"),
    };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn HelloWorld() -> u32 {
    let state = ActorState::load(&fvm_sdk::sself::root().unwrap());

    return_ipld(&state.who_am_i).unwrap()
}
