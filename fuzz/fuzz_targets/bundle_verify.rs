#![no_main]
#![forbid(unsafe_code)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| qlrs_testkit::oracles::bundle_verify(data));
