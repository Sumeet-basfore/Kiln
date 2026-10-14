#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let loaded = kiln_config::load_str("fuzz.toml", source);
        let _ = loaded.config;
        let _ = loaded.diagnostics;
    }
});
