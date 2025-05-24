use wasm_bindgen::prelude::*;
use js_sys::BigInt as JsBigInt;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, ToPrimitive, Pow};
use anyhow::{Result as AnyhowResult, bail, anyhow};
use serde::Serialize;
use std::str::FromStr;
use std::collections::HashMap; // For character mapping

use lazy_static::lazy_static; // To initialize complex statics like HashMaps

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN: u32 = 1;
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX: u32 = 32;

// --- Simple Text Modality Constants and Mappings ---
const SIMPLE_TEXT_ALPHABET: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // Space is 0, A is 1, etc.
const SIMPLE_TEXT_BASE: u32 = 27; // Length of SIMPLE_TEXT_ALPHABET

lazy_static! {
    // Character to value mapping
    static ref CHAR_TO_VAL: HashMap<char, u32> = {
        let mut map = HashMap::new();
        for (i, c) in SIMPLE_TEXT_ALPHABET.chars().enumerate() {
            map.insert(c, i as u32);
        }
        map
    };
    // Value to character mapping
    static ref VAL_TO_CHAR: HashMap<u32, char> = {
        let mut map = HashMap::new();
        for (i, c) in SIMPLE_TEXT_ALPHABET.chars().enumerate() {
            map.insert(i as u32, c);
        }
        map
    };
}
// PADDING_CHAR will be the character for value 0 (Space in this case)
const PADDING_CHAR: char = ' '; // Or whatever VAL_TO_CHAR.get(&0) would be.

// --- End Simple Text Modality Constants ---

#[wasm_bindgen]
pub struct AppState {
    canonical_index: BigInt,
}

#[wasm_bindgen]
impl AppState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AppState {
            canonical_index: BigInt::zero(),
        }
    }

    #[wasm_bindgen(js_name = getCanonicalIndex)]
    pub fn get_canonical_index(&self) -> Result<JsBigInt, JsValue> {
        JsBigInt::from_str(&self.canonical_index.to_string())
            .map_err(|js_err| JsValue::from_str(&format!("Failed to convert Rust BigInt to JS BigInt: {:?}", js_err)))
    }

    #[wasm_bindgen(js_name = setCanonicalIndex)]
    pub fn set_canonical_index(&mut self, js_index: JsBigInt) -> Result<(), JsValue> {
        let index_str_js = js_index.to_string(10)
            .map_err(|_e| JsValue::from_str("Failed to stringify JS BigInt (radix 10)."))?;
        let index_str_rust = index_str_js.as_string()
            .ok_or_else(|| JsValue::from_str("JS BigInt to_string did not return a valid string representation."))?;
        match BigInt::from_str(&index_str_rust) {
            Ok(index) => {
                if index.sign() == Sign::Minus {
                    return Err(JsValue::from_str("Canonical Index cannot be negative."));
                }
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Invalid BigInt format for CI: {}. Input: '{}'", e, index_str_rust)))
        }
    }

    // --- Sequence Modality Functions (Existing) ---
    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32, bit_depth: u32) -> Result<JsValue, JsValue> {
        if bit_depth < SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN || bit_depth > SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX {
             return Err(JsValue::from_str(&format!("Seq bit depth must be {}-{}. Got: {}", 
                SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN, SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX, bit_depth)));
        }
        match index_to_sequence_u32_internal(&self.canonical_index, target_length, bit_depth) {
            Ok(s) => serde_wasm_bindgen::to_value(&s).map_err(|e| JsValue::from_str(&format!("Serialize Vec<u32> failed: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("index_to_sequence_u32_internal error: {}", e))),
        }
    }

    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self, bit_depth: u32) -> Result<u32, JsValue> {
         if bit_depth < SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN || bit_depth > SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX {
             return Err(JsValue::from_str(&format!("Seq bit depth for length calc must be {}-{}. Got: {}",
                SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN, SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX, bit_depth)));
        }
        Ok(calculate_min_sequence_length_internal(&self.canonical_index, bit_depth))
    }

    // --- Simple Text Modality Functions (New) ---
    #[wasm_bindgen(js_name = textToIndexSimple)]
    pub fn text_to_index_simple(&self, text: &str) -> Result<JsBigInt, JsValue> {
        match text_to_index_simple_internal(text) {
            Ok(bi) => JsBigInt::from_str(&bi.to_string())
                        .map_err(|e| JsValue::from_str(&format!("Failed to convert result BigInt to JSBigInt: {:?}", e))),
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }
    
    /// Converts the canonical_index to its simple text representation.
    /// Outputs the text with the minimum number of characters required.
    /// If index is 0, outputs a single padding character (space).
    #[wasm_bindgen(js_name = indexToTextSimple)]
    pub fn index_to_text_simple(&self) -> Result<String, JsValue> {
        let min_len = calculate_min_text_length_simple_internal(&self.canonical_index);
        // If index is 0, min_len will be 0. We want to output at least one char (padding char).
        let target_len = if min_len == 0 && self.canonical_index.is_zero() { 1 } else { min_len };
        index_to_text_simple_internal(&self.canonical_index, target_len)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = setIndexFromTextSimple)]
    pub fn set_index_from_text_simple(&mut self, text: &str) -> Result<(), JsValue> {
        match text_to_index_simple_internal(text) {
            Ok(index) => {
                // text_to_index_simple_internal should only produce non-negative BigInts
                self.canonical_index = index;
                Ok(())
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }
}

// --- Internal Helper Functions for Sequences (Existing) ---
fn index_to_sequence_u32_internal(index: &BigInt, tl: u32, bd: u32) -> AnyhowResult<Vec<u32>> { /* ... same as before ... */ 
    if index.sign()==Sign::Minus{bail!("Negative CI ('{}') for sequence not supported.",index);}
    if index.is_zero(){return Ok(vec![0u32;tl as usize]);}
    if tl==0{bail!("Non-zero CI ('{}') needs target_length > 0.",index);}
    let base = BigInt::one()<<bd; let mut seq=vec![0u32;tl as usize]; let mut temp_idx=index.clone();
    for i in(0..tl).rev(){let rem=&temp_idx%&base; temp_idx/=&base; seq[i as usize]=rem.to_u32().ok_or_else(||anyhow!("Val '{}' too big for u32 (idx {}, bd {}).",rem,i,bd))?; }
    if !temp_idx.is_zero(){bail!("Index '{}' too large for seq len {} (bd {}). Rem: '{}'",index,tl,bd,temp_idx);} Ok(seq)
}
fn calculate_min_sequence_length_internal(index: &BigInt, bd: u32) -> u32 { /* ... same as before ... */ 
    if index.is_zero(){return 0;} let base=BigInt::one()<<bd; let mut len=0u32; let mut temp_idx=index.clone();
    loop{temp_idx/=&base; len+=1; if temp_idx.is_zero(){break;} if len==u32::MAX{break;}} len
}


// --- Internal Helper Functions for Simple Text (New) ---

/// Converts text (using SIMPLE_TEXT_ALPHABET) to a BigInt.
/// Text is treated as a base-N number where N is SIMPLE_TEXT_BASE.
/// Characters not in the alphabet are effectively ignored or could cause error.
/// For simplicity, we'll make unknown characters an error.
fn text_to_index_simple_internal(text: &str) -> AnyhowResult<BigInt> {
    let mut index = BigInt::zero();
    let base = BigInt::from(SIMPLE_TEXT_BASE);

    for char_in_text in text.chars() {
        let char_val = CHAR_TO_VAL.get(&char_in_text.to_ascii_uppercase()) // Be case-insensitive for input
            .ok_or_else(|| anyhow!("Character '{}' not in simple alphabet '{}'", char_in_text, SIMPLE_TEXT_ALPHABET))?;
        
        index = index * &base + BigInt::from(*char_val);
    }
    Ok(index)
}

/// Converts a BigInt index to its text representation using SIMPLE_TEXT_ALPHABET.
/// `target_length` ensures the output string has at least this many characters, padding with PADDING_CHAR if needed.
fn index_to_text_simple_internal(index: &BigInt, target_length: u32) -> AnyhowResult<String> {
    if index.sign() == Sign::Minus {
        bail!("Cannot convert negative index to text.");
    }
    if target_length == 0 && !index.is_zero() {
        // This case should ideally be handled by the caller ensuring target_length > 0
        // if the index is non-zero. calculate_min_text_length_simple_internal helps.
        bail!("Target length cannot be 0 for a non-zero index.");
    }
    if target_length == 0 && index.is_zero() {
        // Special case: index 0 with target_length 0 might mean "empty string" or "single padding char".
        // The public API index_to_text_simple ensures target_length is at least 1 for index 0.
        return Ok(String::new()); // Or Ok(PADDING_CHAR.to_string()) - decided by public fn
    }

    let mut temp_index = index.clone();
    let base = BigInt::from(SIMPLE_TEXT_BASE);
    let mut chars: Vec<char> = Vec::new();

    if temp_index.is_zero() {
        // If index is 0, fill with padding_char up to target_length
        for _ in 0..target_length {
            chars.push(VAL_TO_CHAR.get(&0).copied().unwrap_or('?')); // Should always be PADDING_CHAR
        }
        return Ok(chars.into_iter().collect());
    }

    while !temp_index.is_zero() {
        let remainder_val = (&temp_index % &base).to_u32().unwrap_or(0); // Should fit u32
        temp_index /= &base;
        chars.push(VAL_TO_CHAR.get(&remainder_val).copied().unwrap_or('?')); // '?' for safety, but should always find
    }

    // Pad with PADDING_CHAR (char for value 0) if current length is less than target_length
    while chars.len() < target_length as usize {
        chars.push(VAL_TO_CHAR.get(&0).copied().unwrap_or('?'));
    }
    
    // The characters are generated in reverse order (LSB first), so reverse them.
    Ok(chars.into_iter().rev().collect())
}

/// Calculates the minimum number of characters (using SIMPLE_TEXT_ALPHABET)
/// needed to represent the given BigInt index.
fn calculate_min_text_length_simple_internal(index: &BigInt) -> u32 {
    if index.is_zero() {
        return 0; // 0 itself requires 0 chars by this logic, but UI might want 1 (padding char).
                  // The public API index_to_text_simple will handle this, outputting 1 char for index 0.
    }
    if index.sign() == Sign::Minus { return u32::MAX; } // Or handle error, negative not supported for text

    let mut length = 0u32;
    let mut temp_index = index.clone();
    let base = BigInt::from(SIMPLE_TEXT_BASE);

    if base <= BigInt::one() { return u32::MAX; } // Invalid base

    loop {
        temp_index /= &base;
        length += 1;
        if temp_index.is_zero() { break; }
        if length == u32::MAX { break; } // Safety break
    }
    length
}