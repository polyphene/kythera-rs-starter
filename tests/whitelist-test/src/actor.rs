use frc42_dispatch::match_method;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

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
            "Setup" => {
                Setup();
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

#[allow(non_snake_case)]
fn Setup() {
    // TODO add Setup logic
}