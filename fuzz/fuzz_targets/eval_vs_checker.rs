#![no_main]
#![forbid(unsafe_code)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| qlrs_testkit::oracles::eval_vs_checker(data));
