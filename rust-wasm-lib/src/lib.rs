// This is src/lib.rs
use wasm_bindgen::prelude::*;
use js_sys::{BigInt as JsBigInt, JsString};
use web_sys::console;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, Signed, ToPrimitive, Pow}; // ADDED Pow trait
use anyhow::{Result, bail, anyhow};
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use std::str::FromStr;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// --- Configuration (Rust equivalent) ---
const PROGRAMMER_CHAR_SET_STRING: &str = " \n\t\rabcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~←↑→↓↔∑√≈≠≤≥÷±∞€₹₽£¥₩¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ";
// REMOVED: NUM_SEQUENCE_ELEMENT_BIT_WIDTH (will be passed as parameter)

lazy_static! {
    static ref CHAR_SET_ARRAY: Vec<char> = {
        let mut chars: Vec<char> = PROGRAMMER_CHAR_SET_STRING.chars().collect();
        chars.sort_unstable();
        chars.dedup();
        chars
    };
    static ref CHAR_SET_BASE: BigInt = BigInt::from(CHAR_SET_ARRAY.len());
    // REMOVED: SEQUENCE_ELEMENT_BASE and SEQUENCE_MAX_VAL_BIGINT (will be dynamic)
}

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
        let index_js_string = js_index.to_string(10)?;
        let index_str = String::from(index_js_string);

        match BigInt::from_str(&index_str) {
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => {
                console::error_1(&format!("Failed to parse Rust BigInt from string: {:?}", e).into());
                Err(JsValue::from_str(&format!("Invalid BigInt format: {}", e)))
            }
        }
    }

    // --- Text Conversion ---
    #[wasm_bindgen(js_name = setTextData)]
    pub fn set_text_data(&mut self, text: &str) -> Result<(), JsValue> {
        match text_to_index(text) {
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Text Encoding Error: {}", e)))
        }
    }

    #[wasm_bindgen(js_name = getTextRepresentation)]
    pub fn get_text_representation(&self) -> Result<String, JsValue> {
        match index_to_text(&self.canonical_index) {
            Ok(text) => Ok(text),
            Err(e) => Err(JsValue::from_str(&format!("Index to Text Error: {}", e)))
        }
    }

    // --- Numerical Sequence (Variable Bit Depth) ---
    // MODIFIED: Added bit_depth parameter
    #[wasm_bindgen(js_name = setSequenceData)]
    pub fn set_sequence_data(&mut self, js_sequence_array: JsValue, bit_depth: u32) -> Result<(), JsValue> {
        if bit_depth == 0 || bit_depth > 32 { // Basic validation for bit_depth
             return Err(JsValue::from_str("Bit depth for sequence must be between 1 and 32."));
        }
        let sequence_vec: Vec<u32> = serde_wasm_bindgen::from_value(js_sequence_array)?;
        match sequence_to_index(&sequence_vec, bit_depth) { // Pass bit_depth
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Sequence Error: {}", e))),
        }
    }

    // MODIFIED: Added bit_depth parameter
    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32, bit_depth: u32) -> Result<JsValue, JsValue> {
        if bit_depth == 0 || bit_depth > 32 {
             return Err(JsValue::from_str("Bit depth for sequence must be between 1 and 32."));
        }
        match index_to_sequence(&self.canonical_index, target_length, bit_depth) { // Pass bit_depth
            Ok(sequence) => Ok(serde_wasm_bindgen::to_value(&sequence)?),
            Err(e) => Err(JsValue::from_str(&format!("Sequence Error: {}", e))),
        }
    }

    // MODIFIED: Added bit_depth parameter
    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self, bit_depth: u32) -> Result<u32, JsValue> {
         if bit_depth == 0 || bit_depth > 32 {
             return Err(JsValue::from_str("Bit depth for sequence must be between 1 and 32."));
        }
        Ok(calculate_min_sequence_length_rust(&self.canonical_index, bit_depth)) // Pass bit_depth
    }
}

// --- Text Conversion Logic (Rust) ---
fn text_to_index(text: &str) -> Result<BigInt> {
    let mut index = BigInt::zero();
    if text.is_empty() {
        return Ok(index);
    }
    for (i, ch) in text.chars().enumerate() {
        if let Some(char_val) = CHAR_SET_ARRAY.iter().position(|&c| c == ch) {
            index = index * &*CHAR_SET_BASE + BigInt::from(char_val);
        } else {
            bail!("Character '{}' (Unicode {}) at pos {} not in char set.", ch, ch as u32, i);
        }
    }
    Ok(index)
}

fn index_to_text(index: &BigInt) -> Result<String> {
    if index.is_zero() {
        return Ok(String::new());
    }
    if index.sign() == Sign::Minus {
        bail!("Negative index to text conversion is not supported.");
    }
    let mut text = String::new();
    let mut temp_index = index.clone();
    while temp_index > BigInt::zero() {
        let remainder = (&temp_index % &*CHAR_SET_BASE).to_usize().ok_or_else(|| anyhow!("Remainder too large"))?;
        if remainder < CHAR_SET_ARRAY.len() {
            text.insert(0, CHAR_SET_ARRAY[remainder]);
        } else {
            text.insert(0, '?'); // Should not happen if CHAR_SET_BASE is correct
        }
        temp_index /= &*CHAR_SET_BASE;
    }
    Ok(text)
}

// --- Numerical Sequence (Variable Bit Depth) Conversion ---
// MODIFIED: Added bit_depth parameter
fn sequence_to_index(sequence_array: &[u32], bit_depth: u32) -> Result<BigInt> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);
    let sequence_max_val_bigint: BigInt = &sequence_element_base - BigInt::one();

    let mut index = BigInt::zero();
    for &value in sequence_array {
        let val_bigint = BigInt::from(value);
        if val_bigint.sign() == Sign::Minus || val_bigint > sequence_max_val_bigint {
            bail!("Sequence value {} is out of the 0 to {} range for {}-bit elements.",
                  value, sequence_max_val_bigint, bit_depth);
        }
        index = index * &sequence_element_base + val_bigint;
    }
    Ok(index)
}

// MODIFIED: Added bit_depth parameter
fn index_to_sequence(index: &BigInt, target_length: u32, bit_depth: u32) -> Result<Vec<u32>> {
    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);

    if (target_length as i32) < 0 {
        bail!("Target sequence length cannot be negative.");
    }
    let mut decoded_sequence = Vec::with_capacity(target_length as usize);
    if index.is_zero() && target_length == 0 {
        return Ok(decoded_sequence);
    }
    if index.is_zero() && target_length > 0 {
        decoded_sequence.resize(target_length as usize, 0);
        return Ok(decoded_sequence);
    }
    if target_length == 0 && !index.is_zero() {
        bail!("Cannot represent a non-zero index with a target sequence length of 0 for {}-bit elements.", bit_depth);
    }
    if index.sign() == Sign::Minus {
        bail!("Negative index to sequence conversion is not supported.");
    }

    let mut temp_index = index.clone();
    for _ in 0..target_length {
        let remainder_bigint = &temp_index % &sequence_element_base;
        let remainder = remainder_bigint.to_u32().ok_or_else(|| anyhow!("Sequence element value too large for u32"))?;
        decoded_sequence.insert(0, remainder);
        temp_index /= &sequence_element_base;
    }

    if temp_index > BigInt::zero() {
        bail!("Index {} is too large to be represented by a sequence of length {} with {}-bit elements.", index, target_length, bit_depth);
    }
    Ok(decoded_sequence)
}

// MODIFIED: Added bit_depth parameter
fn calculate_min_sequence_length_rust(index: &BigInt, bit_depth: u32) -> u32 {
    if index.is_zero() {
        return 0; // An index of 0 requires 0 elements (empty sequence)
    }
    let sequence_element_base: BigInt = BigInt::from(2).pow(bit_depth);
    if sequence_element_base <= BigInt::one() { // Handles bit_depth = 0 or 1 if allowed (though UI limits to >=8)
        if index.is_zero() { return 0; }
        // For base 1, a non-zero index would theoretically require infinite elements.
        // This case should be prevented by UI/validation (bit_depth >= 8).
        // Return a large number or handle as error if this path is reachable.
        // For simplicity, let's assume bit_depth leads to base > 1.
        // If bit_depth could be 0 or 1, this needs robust error handling or a different length logic.
        // Given our UI constraints (>=8), this is fine.
    }

    let mut length = 0;
    let mut temp_index = index.clone();
    if temp_index.sign() == Sign::Minus {
        temp_index = temp_index.abs(); // Work with positive value for length calculation
    }

    // A non-zero index always requires at least one element if the base is > 1
    loop {
        temp_index /= &sequence_element_base;
        length += 1;
        if temp_index.is_zero() {
            break;
        }
    }
    length
}