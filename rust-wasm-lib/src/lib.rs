// This is src/lib.rs
use wasm_bindgen::prelude::*;
use js_sys::{BigInt as JsBigInt}; 
// web_sys::console is not directly used by this version of lib.rs,
// but wasm-pack might include it if other dependencies need it,
// or if console_error_panic_hook is enabled.
// For a minimal build, if nothing else needs it, it could be removed from Cargo.toml's web-sys features.
// use web_sys::console; 
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, Signed, ToPrimitive, Pow};
use anyhow::{Result, bail, anyhow};
// lazy_static is no longer needed as CHAR_SET_ARRAY/BASE are removed
// use lazy_static::lazy_static; 
use serde::{Serialize, Deserialize}; 
use std::str::FromStr;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// PROGRAMMER_CHAR_SET_STRING REMOVED
const MAX_SEQUENCE_ELEMENT_BIT_DEPTH: u32 = 256; 

// lazy_static block for CHAR_SET_ARRAY and CHAR_SET_BASE REMOVED

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

    // setTextData REMOVED
    // getTextRepresentation REMOVED

    #[wasm_bindgen(js_name = setSequenceData)]
    pub fn set_sequence_data(&mut self, js_sequence_array_of_bigints: JsValue, bit_depth: u32) -> Result<(), JsValue> {
        if bit_depth == 0 || bit_depth > MAX_SEQUENCE_ELEMENT_BIT_DEPTH {
             return Err(JsValue::from_str(&format!("Bit depth for sequence must be between 1 and {}.", MAX_SEQUENCE_ELEMENT_BIT_DEPTH)));
        }
        let sequence_vec: Vec<BigInt> = serde_wasm_bindgen::from_value(js_sequence_array_of_bigints)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize sequence: {}", e)))?;
        match sequence_to_index(&sequence_vec, bit_depth) {
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Sequence to Index Error: {}", e))),
        }
    }

    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32, bit_depth: u32) -> Result<JsValue, JsValue> {
        if bit_depth == 0 || bit_depth > MAX_SEQUENCE_ELEMENT_BIT_DEPTH {
             return Err(JsValue::from_str(&format!("Bit depth for sequence must be between 1 and {}.", MAX_SEQUENCE_ELEMENT_BIT_DEPTH)));
        }
        match index_to_sequence(&self.canonical_index, target_length, bit_depth) {
            Ok(sequence_of_bigints) => {
                serde_wasm_bindgen::to_value(&sequence_of_bigints)
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

// text_to_index function REMOVED
// index_to_text function REMOVED

// --- Numerical Sequence (Variable Bit Depth, BigInt elements) ---
fn sequence_to_index(sequence_array: &[BigInt], bit_depth: u32) -> Result<BigInt> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);
    let sequence_max_val_bigint: BigInt = &sequence_element_base - BigInt::one();

    let mut index = BigInt::zero();
    for val_bigint in sequence_array {
        if val_bigint.sign() == Sign::Minus || *val_bigint > sequence_max_val_bigint {
            bail!("Sequence value {} is out of the 0 to {} range for {}-bit elements.",
                  val_bigint, sequence_max_val_bigint, bit_depth);
        }
        index = index * &sequence_element_base + val_bigint;
    }
    Ok(index)
}

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
    if index.sign() == Sign::Minus { bail!("Negative index to sequence conversion is not supported."); }

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

fn calculate_min_sequence_length_rust(index: &BigInt, bit_depth: u32) -> u32 {
    if index.is_zero() { return 0; }
    if bit_depth == 0 { return if index.is_zero() { 0 } else { u32::MAX }; }

    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);
    if sequence_element_base <= BigInt::one() { return if index.is_zero() { 0 } else { u32::MAX }; }

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