use wasm_bindgen::prelude::*;
use js_sys::BigInt as JsBigInt;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, ToPrimitive}; // Removed Pow as it's not used directly now
use anyhow::{Result as AnyhowResult, bail, anyhow};
use serde::{Deserialize}; // Removed Serialize as it's not used directly now
use serde_json; 
use std::str::FromStr;
use std::collections::HashMap;

use lazy_static::lazy_static;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// --- Constants ---
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN: u32 = 1;
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX: u32 = 32;
const SIMPLE_TEXT_ALPHABET: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const SIMPLE_TEXT_BASE: u32 = 27; // Length of SIMPLE_TEXT_ALPHABET
const PADDING_CHAR: char = ' '; // Character for value 0

lazy_static! {
    static ref CHAR_TO_VAL: HashMap<char, u32> = {
        SIMPLE_TEXT_ALPHABET.chars().enumerate().map(|(i, c)| (c, i as u32)).collect()
    };
    static ref VAL_TO_CHAR: HashMap<u32, char> = {
        SIMPLE_TEXT_ALPHABET.chars().enumerate().map(|(i, c)| (i as u32, c)).collect()
    };
}

// --- JSON Instruction Structs (V1) ---
#[derive(Deserialize, Debug)]
#[serde(tag = "instruction_type")] 
enum Instruction {
    #[serde(rename = "LITERAL_BIGINT")]
    LiteralBigInt { value: String },

    #[serde(rename = "LITERAL_TEXT_TO_CI")]
    LiteralTextToCi {
        text_value: String,
        text_modality_alphabet_id: String, 
    },

    #[serde(rename = "REPEAT_TEXT_PATTERN_TO_CI")]
    RepeatTextPatternToCi {
        pattern_text: String,
        count: u32, 
        text_modality_alphabet_id: String,
    },
}

// --- AppState ---
#[wasm_bindgen]
pub struct AppState {
    canonical_index: BigInt,
}

#[wasm_bindgen]
impl AppState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AppState { canonical_index: BigInt::zero() }
    }

    #[wasm_bindgen(js_name = getCanonicalIndex)]
    pub fn get_canonical_index(&self) -> Result<JsBigInt, JsValue> {
        JsBigInt::from_str(&self.canonical_index.to_string())
            .map_err(|e| JsValue::from_str(&format!("Rust BigInt to JS BigInt fail: {:?}", e)))
    }

    #[wasm_bindgen(js_name = setCanonicalIndex)]
    pub fn set_canonical_index(&mut self, js_index: JsBigInt) -> Result<(), JsValue> {
        let idx_str_js = js_index.to_string(10).map_err(|_| JsValue::from_str("JSBigInt stringify fail."))?;
        let idx_str_rs = idx_str_js.as_string().ok_or_else(|| JsValue::from_str("JSString to Rust string fail."))?;
        match BigInt::from_str(&idx_str_rs) {
            Ok(idx) => {
                if idx.sign() == Sign::Minus { return Err(JsValue::from_str("CI cannot be negative.")); }
                self.canonical_index = idx; Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Invalid BigInt for CI: {}. Input: '{}'", e, idx_str_rs)))
        }
    }

    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, tl: u32, bd: u32) -> Result<JsValue, JsValue> {
        if bd < SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN || bd > SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX {
            return Err(JsValue::from_str("Invalid bit_depth for sequence."));
        }
        match index_to_sequence_u32_internal(&self.canonical_index, tl, bd) {
            Ok(s) => serde_wasm_bindgen::to_value(&s).map_err(|e| JsValue::from_str(&format!("Seq serialize fail: {}",e))),
            Err(e) => Err(JsValue::from_str(&format!("index_to_seq fail: {}",e))),
        }
    }

    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self, bd: u32) -> Result<u32, JsValue> {
        if bd < SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN || bd > SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX {
            return Err(JsValue::from_str("Invalid bit_depth for seq length calc."));
        }
        Ok(calculate_min_sequence_length_internal(&self.canonical_index, bd))
    }
    
    #[wasm_bindgen(js_name = indexToTextSimple)]
    pub fn index_to_text_simple(&self) -> Result<String, JsValue> {
        let min_len = calculate_min_text_length_simple_internal(&self.canonical_index);
        // If index is 0, min_len will be 0. We want to output at least one char (padding char).
        let target_len = if self.canonical_index.is_zero() { 1.max(min_len) } else { min_len }; // Ensures at least 1 for index 0
        index_to_text_simple_internal(&self.canonical_index, target_len).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = setIndexFromTextSimple)]
    pub fn set_index_from_text_simple(&mut self, text: &str) -> Result<(), JsValue> {
        match text_to_index_simple_internal(text) {
            Ok(idx) => { self.canonical_index = idx; Ok(()) },
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    #[wasm_bindgen(js_name = executeJsonInstructionsToCI)]
    pub fn execute_json_instructions_to_ci(&self, json_string: &str) -> Result<JsBigInt, JsValue> {
        let instruction: Instruction = serde_json::from_str(json_string)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

        match instruction {
            Instruction::LiteralBigInt { value } => {
                let bi = BigInt::from_str(&value)
                    .map_err(|e| JsValue::from_str(&format!("LITERAL_BIGINT parse error: {}", e)))?;
                if bi.sign() == Sign::Minus {
                    return Err(JsValue::from_str("LITERAL_BIGINT value cannot be negative for CI."));
                }
                JsBigInt::from_str(&bi.to_string())
                     .map_err(|e| JsValue::from_str(&format!("LITERAL_BIGINT result to JSBigInt fail: {:?}", e)))
            }
            Instruction::LiteralTextToCi { text_value, text_modality_alphabet_id } => {
                if text_modality_alphabet_id != "SIMPLE_TEXT_A_Z_SPACE" {
                    return Err(JsValue::from_str("Unsupported text_modality_alphabet_id"));
                }
                let bi = text_to_index_simple_internal(&text_value)
                    .map_err(|e| JsValue::from_str(&format!("LITERAL_TEXT_TO_CI conversion error: {}", e)))?;
                JsBigInt::from_str(&bi.to_string())
                    .map_err(|e| JsValue::from_str(&format!("LITERAL_TEXT_TO_CI result to JSBigInt fail: {:?}", e)))
            }
            Instruction::RepeatTextPatternToCi { pattern_text, count, text_modality_alphabet_id } => {
                if text_modality_alphabet_id != "SIMPLE_TEXT_A_Z_SPACE" {
                    return Err(JsValue::from_str("Unsupported text_modality_alphabet_id"));
                }
                if pattern_text.is_empty() || count == 0 {
                    return JsBigInt::from_str("0") // Empty string or repeating 0 times yields index 0
                           .map_err(|e| JsValue::from_str(&format!("Repeat 0/empty to JSBigInt fail: {:?}", e)));
                }

                let full_text = pattern_text.repeat(count as usize);
                let bi = text_to_index_simple_internal(&full_text)
                    .map_err(|e| JsValue::from_str(&format!("REPEAT_TEXT_PATTERN_TO_CI conversion error: {}", e)))?;
                JsBigInt::from_str(&bi.to_string())
                    .map_err(|e| JsValue::from_str(&format!("REPEAT_TEXT_PATTERN_TO_CI result to JSBigInt fail: {:?}", e)))
            }
        }
    }
}

// --- Internal Helper Functions ---

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

fn index_to_text_simple_internal(index: &BigInt, target_length: u32) -> AnyhowResult<String> {
    if index.sign() == Sign::Minus { bail!("Negative index to text fail."); }
    // target_length is expected to be >= 1 if index is 0, due to logic in public AppState::index_to_text_simple
    if target_length == 0 {
        // This state implies index is non-zero but target_length is 0, which is problematic.
        // calculate_min_text_length_simple_internal would return >0 for non-zero index.
        // The public function should ensure target_length >= min_len.
        // If somehow called directly with target_length 0 for non-zero index, it's an issue.
        if !index.is_zero() {
             bail!("Target length 0 for non-zero index ('{}') is invalid.", index);
        }
        // If index is zero AND target_length is zero (shouldn't happen from public API), return empty or padding.
        // Let's return single padding char to be consistent with public API's minimum length of 1 for index 0.
        return Ok(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR).to_string());
    }

    let mut temp_index = index.clone(); 
    let base = BigInt::from(SIMPLE_TEXT_BASE); 
    let mut chars: Vec<char> = Vec::new();

    if temp_index.is_zero() { 
        // If index is 0, fill with PADDING_CHAR up to target_length (which will be >= 1)
        for _ in 0..target_length {
            chars.push(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR));
        } 
        return Ok(chars.into_iter().collect());
    }

    // Loop for non-zero index
    loop {
        // THE FIX IS HERE: temp_index.clone() for the % operation
        let remainder_val = (temp_index.clone() % &base).to_u32()
            .ok_or_else(|| anyhow!("Remainder too large for u32 in text conversion. Index: {}, Base: {}", temp_index, base))?;
        temp_index /= &base; 
        chars.push(VAL_TO_CHAR.get(&remainder_val).copied().unwrap_or('?')); // '?' for safety, but should always find
        if temp_index.is_zero() {
            break;
        }
    }
    
    // Pad with PADDING_CHAR if current length is less than target_length
    while chars.len() < target_length as usize {
        chars.push(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR));
    }
    
    Ok(chars.into_iter().rev().collect())
}

fn calculate_min_text_length_simple_internal(index: &BigInt) -> u32 {
    if index.is_zero() { return 0; } // The public API index_to_text_simple will adjust this to 1.
    if index.sign() == Sign::Minus { return u32::MAX; } // Or an error.

    let mut length = 0u32; 
    let mut temp_index = index.clone(); 
    let base = BigInt::from(SIMPLE_TEXT_BASE);

    if base <= BigInt::one() { return u32::MAX; } // Invalid base for this logic.

    loop {
        temp_index /= &base; 
        length += 1; 
        if temp_index.is_zero() { break; } 
        if length == u32::MAX { break; } // Safety break
    } 
    length
}

fn index_to_sequence_u32_internal(index: &BigInt, tl: u32, bd: u32) -> AnyhowResult<Vec<u32>> { 
    if index.sign() == Sign::Minus { bail!("Negative CI ('{}') for sequence not supported.", index); }
    if index.is_zero() { return Ok(vec![0u32; tl as usize]); }
    if tl == 0 { bail!("Non-zero CI ('{}') needs target_length > 0 for sequence.", index); }
    let base = BigInt::one() << bd; 
    let mut seq = vec![0u32; tl as usize]; 
    let mut temp_idx = index.clone();
    for i in (0..tl).rev() {
        let rem = temp_idx.clone() % &base; // Cloned here too for safety, though /= might not move if it's a Ref impl. Better safe.
        temp_idx /= &base; 
        seq[i as usize] = rem.to_u32().ok_or_else(|| anyhow!("Val '{}' too big for u32 (idx {}, bd {}).", rem, i, bd))?; 
    }
    if !temp_idx.is_zero() { bail!("Index '{}' too large for seq len {} (bd {}). Rem: '{}'", index, tl, bd, temp_idx); } 
    Ok(seq)
}

fn calculate_min_sequence_length_internal(index: &BigInt, bd: u32) -> u32 { 
    if index.is_zero() { return 0; } 
    let base = BigInt::one() << bd; 
    let mut len = 0u32; 
    let mut temp_idx = index.clone();
    if base <= BigInt::one() { return u32::MAX; } // Prevent infinite loop for bd=0
    loop {
        temp_idx /= &base; 
        len += 1; 
        if temp_idx.is_zero() { break; } 
        if len == u32::MAX { break; }
    } 
    len
}