#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The first 8 bytes seed the same generator the stable driver
    // uses; shorter inputs pad with zeros.
    let mut seed_bytes = [0u8; 8];
    for (slot, byte) in seed_bytes.iter_mut().zip(data.iter()) {
        *slot = *byte;
    }
    let seed = u64::from_le_bytes(seed_bytes);
    firecraft_fuzz::anchor_engine_commands(seed);
});
