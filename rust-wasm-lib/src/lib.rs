// This is src/lib.rs
use wasm_bindgen::prelude::*;
use js_sys::BigInt as JsBigInt;
// web_sys::console might be needed by console_error_panic_hook
// use web_sys::console; 
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, Signed, ToPrimitive, Pow};
use anyhow::{Result, bail, anyhow};
// lazy_static not needed as no CHAR_SET or fixed sequence constants at this level
use serde::{Deserialize, Serialize}; // For Vec<u32> <-> JS Array<Number>
use std::str::FromStr;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// Hardcoded bit depth for this simplified version
const SEQUENCE_ELEMENT_BIT_DEPTH: u32 = 24;

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
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Invalid BigInt format: {}", e)))
        }
    }

    // Text modality functions REMOVED

    // Sequence functions now assume fixed 24-bit depth and Vec<u32>
    #[wasm_bindgen(js_name = setSequenceData)]
    pub fn set_sequence_data(&mut self, js_sequence_array: JsValue) -> Result<(), JsValue> {
        let sequence_vec: Vec<u32> = serde_wasm_bindgen::from_value(js_sequence_array)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize sequence: {}", e)))?;
        match sequence_to_index(&sequence_vec) { // No bit_depth param needed here
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Sequence to Index Error: {}", e))),
        }
    }

    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32) -> Result<JsValue, JsValue> {
        match index_to_sequence(&self.canonical_index, target_length) { // No bit_depth param
            Ok(sequence) => { // sequence is Vec<u32>
                serde_wasm_bindgen::to_value(&sequence)
                    .map_err(|e| JsValue::from_str(&format!("Failed to serialize sequence: {}",e)))
            },
            Err(e) => Err(JsValue::from_str(&format!("Index to Sequence Error: {}", e))),
        }
    }

    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self) -> Result<u32, JsValue> { // No bit_depth param
        Ok(calculate_min_sequence_length_rust(&self.canonical_index))
    }
}

// --- Numerical Sequence (Fixed 24-bit, u32 elements) Conversion ---
fn sequence_to_index(sequence_array: &[u32]) -> Result<BigInt> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(SEQUENCE_ELEMENT_BIT_DEPTH);
    let sequence_max_val: u32 = (1u32 << SEQUENCE_ELEMENT_BIT_DEPTH) - 1; // 2^24 - 1

    let mut index = BigInt::zero();
    for &value in sequence_array {
        if value > sequence_max_val {
            bail!("Sequence value {} is out of the 0 to {} range for {}-bit elements.",
                  value, sequence_max_val, SEQUENCE_ELEMENT_BIT_DEPTH);
        }
        index = index * &sequence_element_base + BigInt::from(value);
    }
    Ok(index)
}

fn index_to_sequence(index: &BigInt, target_length: u32) -> Result<Vec<u32>> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(SEQUENCE_ELEMENT_BIT_DEPTH);

    if (target_length as i32) < 0 { bail!("Target sequence length cannot be negative."); }
    
    let mut decoded_sequence: Vec<u32> = Vec::with_capacity(target_length as usize);
    if index.is_zero() {
        decoded_sequence.resize(target_length as usize, 0);
        return Ok(decoded_sequence);
    }
    if target_length == 0 && !index.is_zero() {
        bail!("Cannot represent a non-zero index with a target sequence length of 0.");
    }
    if index.sign() == Sign::Minus { bail!("Negative index to sequence conversion is not supported."); }

    let mut temp_index = index.clone();
    for _ in 0..target_length {
        let remainder_bigint = &temp_index % &sequence_element_base;
        let remainder = remainder_bigint.to_u32()
            .ok_or_else(|| anyhow!("Sequence element value derived from index is too large for u32. This should not happen if elements are {}-bit.", SEQUENCE_ELEMENT_BIT_DEPTH))?;
        decoded_sequence.insert(0, remainder);
        temp_index /= &sequence_element_base;
    }

    if temp_index > BigInt::zero() {
        bail!("Index {} is too large for sequence of length {} with {}-bit elements.", index, target_length, SEQUENCE_ELEMENT_BIT_DEPTH);
    }
    Ok(decoded_sequence)
}

fn calculate_min_sequence_length_rust(index: &BigInt) -> u32 {
    if index.is_zero() { return 0; }
    
    let sequence_element_base: BigInt = BigInt::from(2).pow(SEQUENCE_ELEMENT_BIT_DEPTH);
    // Base will be > 1 for 24-bit depth, so no need to check sequence_element_base <= One here.

    let mut length = 0;
    let mut temp_index = index.clone();
    if temp_index.sign() == Sign::Minus { temp_index = temp_index.abs(); }

    loop {
        temp_index /= &sequence_element_base;
        length += 1;
        if temp_index.is_zero() { break; }
    }
    length
}