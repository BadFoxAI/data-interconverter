// This is src/lib.rs
use wasm_bindgen::prelude::*;
use js_sys::{BigInt as JsBigInt, Array as JsArray}; 
// use web_sys::console; // Only if console_error_panic_hook needs it & it's enabled in Cargo.toml
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, Signed, ToPrimitive, Pow};
use anyhow::{Result, bail, anyhow};
// lazy_static is removed as CHAR_SET_ARRAY is removed
use serde::{Serialize}; // Deserialize might not be needed if setSequenceData is removed
use std::str::FromStr;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

const MAX_SEQUENCE_ELEMENT_BIT_DEPTH: u32 = 256; 

#[wasm_bindgen]
pub struct AppState {
    canonical_index: BigInt,
}

#[wasm_bindgen]
impl AppState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> AppState {
        AppState {
            canonical_index: BigInt::zero(),
        }
    }

    #[wasm_bindgen(js_name = getCanonicalIndex)]
    pub fn get_canonical_index(&self) -> JsBigInt { 
        JsBigInt::from(JsValue::from_str(&self.canonical_index.to_string()))
    }

    #[wasm_bindgen(js_name = setCanonicalIndex)]
    pub fn set_canonical_index(&mut self, js_index: JsBigInt) -> Result<(), JsValue> { 
        let index_str = String::from(js_index.to_string(10).map_err(|_e| JsValue::from_str("Failed to stringify JS BigInt"))?);
        match BigInt::from_str(&index_str) {
            Ok(index) => {
                if index.sign() == Sign::Minus {
                    return Err(JsValue::from_str("Canonical Index cannot be negative."));
                }
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Invalid BigInt format for CI: {}", e)))
        }
    }

    // setSequenceData is REMOVED because sequence input is removed from UI
    // User only inputs canonical_index directly.

    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32, bit_depth: u32) -> Result<JsValue, JsValue> {
        if bit_depth == 0 || bit_depth > MAX_SEQUENCE_ELEMENT_BIT_DEPTH {
             return Err(JsValue::from_str(&format!("Bit depth for sequence must be between 1 and {}.", MAX_SEQUENCE_ELEMENT_BIT_DEPTH)));
        }
        match index_to_sequence(&self.canonical_index, target_length, bit_depth) {
            Ok(sequence_of_rust_bigints) => { // This is Vec<num_bigint::BigInt>
                // Manually construct JsArray of JsBigInt to ensure JS receives Array<BigInt>
                let js_array = JsArray::new_with_length(sequence_of_rust_bigints.len() as u32);
                for (i, rust_bigint) in sequence_of_rust_bigints.iter().enumerate() {
                    let js_bigint_val = JsBigInt::from(JsValue::from_str(&rust_bigint.to_string()));
                    js_array.set(i as u32, js_bigint_val.into()); 
                }
                Ok(js_array.into()) 
            },
            Err(e) => Err(JsValue::from_str(&format!("Index to Sequence Error: {}", e))),
        }
    }

    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self, bit_depth: u32) -> Result<u32, JsValue> {
         if bit_depth == 0 || bit_depth > MAX_SEQUENCE_ELEMENT_BIT_DEPTH {
             return Err(JsValue::from_str(&format!("Bit depth for sequence must be between 1 and {}.", MAX_SEQUENCE_ELEMENT_BIT_DEPTH)));
        }
        Ok(calculate_min_sequence_length_rust(&self.canonical_index, bit_depth))
    }
}

// text_to_index and index_to_text are REMOVED

// Internal function to convert canonical index to a sequence of BigInts
fn index_to_sequence(index: &BigInt, target_length: u32, bit_depth: u32) -> Result<Vec<BigInt>> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);

    if (target_length as i32) < 0 { bail!("Target sequence length cannot be negative."); }
    
    let mut decoded_sequence: Vec<BigInt> = Vec::with_capacity(target_length as usize);
    if index.is_zero() {
        for _ in 0..target_length {
            decoded_sequence.push(BigInt::zero());
        }
        return Ok(decoded_sequence);
    }
    if target_length == 0 && !index.is_zero() {
        bail!("Cannot represent a non-zero index with a target sequence length of 0.");
    }
    if index.sign() == Sign::Minus { 
        bail!("Negative canonical index to sequence conversion is not supported."); 
    }

    let mut temp_index = index.clone();
    for _ in 0..target_length {
        let remainder_bigint = &temp_index % &sequence_element_base;
        decoded_sequence.insert(0, remainder_bigint);
        temp_index /= &sequence_element_base;
    }

    if temp_index > BigInt::zero() {
        bail!("Index {} is too large for sequence of length {} with {}-bit elements.", index, target_length, bit_depth);
    }
    Ok(decoded_sequence)
}

// Internal function to calculate minimum sequence length
fn calculate_min_sequence_length_rust(index: &BigInt, bit_depth: u32) -> u32 {
    if index.is_zero() { return 0; }
    if bit_depth == 0 { return u32::MAX; } 

    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);
    if sequence_element_base <= BigInt::one() { return u32::MAX; }

    let mut length = 0;
    let mut temp_index = index.clone();
    
    loop {
        temp_index /= &sequence_element_base;
        length += 1;
        if temp_index.is_zero() { break; }
    }
    length
}