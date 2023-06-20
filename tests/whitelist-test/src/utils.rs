use fvm_ipld_encoding::de::DeserializeOwned;

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