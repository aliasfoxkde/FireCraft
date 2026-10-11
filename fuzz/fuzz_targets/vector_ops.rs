#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    firecraft_fuzz::anchor_vector_ops(data);
});
