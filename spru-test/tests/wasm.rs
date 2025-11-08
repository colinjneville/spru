#![cfg(target_arch = "wasm32")]

mod common;

use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
pub fn minimal_spru() {
    common::minimal_spru();
}
