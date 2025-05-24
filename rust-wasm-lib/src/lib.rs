// This is src/lib.rs
use wasm_bindgen::prelude::*;
use js_sys::{BigInt as JsBigInt}; 
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, Pow, ToPrimitive, Signed}; // Added Signed
use anyhow::{Result, bail, anyhow};
// lazy_static not needed
use serde::{Serialize}; // Only Serialize needed for Vec<BigInt> -> JsValue
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
                if index.sign() == Sign::Minus { // Enforce non-negative canonical index
                    return Err(JsValue::from_str("Canonical Index cannot be negative."));
                }
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Invalid BigInt format for Canonical Index: {}", e)))
        }
    }

    // setSequenceData REMOVED as sequence is now read-only output

    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32, bit_depth: u32) -> Result<JsValue, JsValue> {
        if bit_depth == 0 || bit_depth > MAX_SEQUENCE_ELEMENT_BIT_DEPTH {
             return Err(JsValue::from_str(&format!("Bit depth for sequence must be between 1 and {}.", MAX_SEQUENCE_ELEMENT_BIT_DEPTH)));
        }
        match index_to_sequence(&self.canonical_index, target_length, bit_depth) {
            Ok(sequence_of_bigints) => { // sequence_of_bigints is Vec<BigInt>
                serde_wasm_bindgen::to_value(&sequence_of_bigints) // Serializes Vec<BigInt> to JS Array<BigInt>
                    .map_err(|e| JsValue::from_str(&format!("Failed to serialize sequence: {}",e)))
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

// Internal function to convert canonical index to a sequence of BigInts
fn index_to_sequence(index: &BigInt, target_length: u32, bit_depth: u32) -> Result<Vec<BigInt>> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);

    if (target_length as i32) < 0 { bail!("Target sequence length cannot be negative."); }
    
    let mut decoded_sequence: Vec<BigInt> = Vec::with_capacity(target_length as usize);
    
    // Handle index 0: it should result in a sequence of zeros of target_length,
    // or an empty sequence if target_length is 0.
    if index.is_zero() {
        for _ in 0..target_length {
            decoded_sequence.push(BigInt::zero());
        }
        return Ok(decoded_sequence);
    }

    // The following conditions only apply if index is non-zero
    if target_length == 0 { // && !index.is_zero() implied
        bail!("Cannot represent a non-zero index with a target sequence length of 0.");
    }
    if index.sign() == Sign::Minus { 
        // This should be prevented by setCanonicalIndex, but defensive check here.
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
    if index.is_zero() { return 0; } // An index of 0 results in an empty sequence if length is dynamic.
                                    // If a view *must* show something, JS can decide to show a single "0".
    if bit_depth == 0 { return u32::MAX; } // Or error: 0-bit elements can't represent non-zero index

    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);
    if sequence_element_base <= BigInt::one() { // e.g. bit_depth = 0 (already handled) or bit_depth = 1 and base = 2^0 which is 1 if not handled.
                                                // This case actually implies bit_depth=0, which should yield base 1. Base 2 for bit_depth 1.
        return if index.is_zero() { 0 } else { u32::MAX }; // Infinite length for non-zero index
    }

    let mut length = 0;
    let mut temp_index = index.clone();
    // Canonical index is non-negative by this point due to setCanonicalIndex validation

    loop {
        temp_index /= &sequence_element_base;
        length += 1;
        if temp_index.is_zero() { break; }
    }
    length
}