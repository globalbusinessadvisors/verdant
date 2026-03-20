#![no_main]

use libfuzzer_sys::fuzz_target;
use verdant_qudag::message::QuDagMessage;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize arbitrary bytes as a QuDagMessage.
    // This should never panic, regardless of input.
    let _result: Result<QuDagMessage, _> = postcard::from_bytes(data);
});
