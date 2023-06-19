use cid::{multihash::Code, Cid};
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::DAG_CBOR;
use fvm_ipld_encoding::de::DeserializeOwned;
use fvm_shared::error::ErrorNumber;
use serde::ser;
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use thiserror::Error;

/**************************************************
 * Actor's state
 **************************************************/

#[derive(Serialize_tuple, Deserialize_tuple)]
struct ActorState {
    // TODO set your actors state properties here
    placeholder: u64
}

impl ActorState {
    #[allow(dead_code)]
    pub fn load(cid: &Cid) -> Self {
        let data = fvm_sdk::ipld::get(cid).unwrap();
        fvm_ipld_encoding::from_slice::<Self>(&data).unwrap()
    }
    #[allow(dead_code)]
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
#[allow(dead_code)]
pub fn deserialize_params<D: DeserializeOwned>(params: u32) -> D {
    let params = fvm_sdk::message::params_raw(params)
        .expect("Could not get message parameters")
        .expect("Expected message parameters but got none");

    let params = fvm_ipld_encoding::RawBytes::new(params.data);

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

#[allow(dead_code)]
fn return_ipld<T>(value: &T) -> std::result::Result<u32, IpldError>
    where
        T: ser::Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(fvm_sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
}
