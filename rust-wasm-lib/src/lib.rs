// This is src/lib.rs
use wasm_bindgen::prelude::*;
// Removed: Uint8Array (from js_sys), ImageData (from web_sys)
use js_sys::{BigInt as JsBigInt, JsString};
use web_sys::console;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, Signed, ToPrimitive};
use anyhow::{Result, bail, anyhow};
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize}; // Keep for serde_wasm_bindgen
use std::str::FromStr; // Needed for BigInt::from_str

// REMOVED: mod image_processing;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// --- Configuration (Rust equivalent) ---
const PROGRAMMER_CHAR_SET_STRING: &str = " \n\t\rabcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~←↑→↓↔∑√≈≠≤≥÷±∞€₹₽£¥₩¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ";
const NUM_SEQUENCE_ELEMENT_BIT_WIDTH: u32 = 24;

lazy_static! {
    static ref CHAR_SET_ARRAY: Vec<char> = {
        let mut chars: Vec<char> = PROGRAMMER_CHAR_SET_STRING.chars().collect();
        chars.sort_unstable();
        chars.dedup();
        chars
    };
    static ref CHAR_SET_BASE: BigInt = BigInt::from(CHAR_SET_ARRAY.len());
    static ref SEQUENCE_ELEMENT_BASE: BigInt = BigInt::from(2).pow(NUM_SEQUENCE_ELEMENT_BIT_WIDTH);
    static ref SEQUENCE_MAX_VAL_BIGINT: BigInt = &*SEQUENCE_ELEMENT_BASE - BigInt::one();
}

/// AppState holds the central canonical BigInt index and provides methods
/// for converting to/from various data modalities.
#[wasm_bindgen]
pub struct AppState {
    canonical_index: BigInt,
}

#[wasm_bindgen]
impl AppState {
    /// Creates a new instance of AppState, initializing the canonical index to zero.
    #[wasm_bindgen(constructor)]
    pub fn new() -> AppState {
        AppState {
            canonical_index: BigInt::zero(),
        }
    }

    /// Getter for the current canonical index. Returns a JavaScript BigInt.
    #[wasm_bindgen(js_name = getCanonicalIndex)]
    pub fn get_canonical_index(&self) -> JsBigInt {
        JsBigInt::from(JsValue::from_str(&self.canonical_index.to_string()))
    }

    /// Setter for the canonical index. Takes a JavaScript BigInt.
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

    /// Sets the canonical index based on the provided text string.
    /// Characters not in the defined character set will cause an error.
    #[wasm_bindgen(js_name = setTextData)]
    pub fn set_text_data(&mut self, text: &str) -> Result<(), JsValue> {
        console::log_1(&format!("Rust: setTextData called with: \"{}\"", text).into());
        match text_to_index(text) {
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => {
                console::error_1(&format!("Rust: Text Encoding Error: {:?}", e).into());
                Err(JsValue::from_str(&format!("Text Encoding Error: {}", e)))
            }
        }
    }

    /// Returns the text representation of the current canonical index.
    /// Negative indices are not supported.
    #[wasm_bindgen(js_name = getTextRepresentation)]
    pub fn get_text_representation(&self) -> Result<String, JsValue> {
        match index_to_text(&self.canonical_index) {
            Ok(text) => Ok(text),
            Err(e) => {
                console::error_1(&format!("Rust: Index to Text Error: {:?}", e).into());
                Err(JsValue::from_str(&format!("Index to Text Error: {}", e)))
            }
        }
    }

    // --- IMAGE MODALITY REMOVED ---
    // All functions like process_uploaded_image, get_image_data_representation,
    // and calculate_optimal_image_dimensions have been REMOVED.

    // --- Numerical Sequence (24-bit elements) ---

    /// Sets the canonical index based on a sequence of 24-bit unsigned integers.
    /// Values outside the 0 to (2^24 - 1) range will cause an error.
    #[wasm_bindgen(js_name = setSequenceData)]
    pub fn set_sequence_data(&mut self, js_sequence_array: JsValue) -> Result<(), JsValue> {
        let sequence_vec: Vec<u32> = serde_wasm_bindgen::from_value(js_sequence_array)?;
        match sequence_to_index(&sequence_vec) {
            Ok(index) => {
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Sequence Error: {}", e))),
        }
    }

    /// Returns the sequence of 24-bit unsigned integers representation of the current canonical index.
    /// `target_length` specifies the desired length of the sequence, padding with leading zeros if necessary.
    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32) -> Result<JsValue, JsValue> {
        match index_to_sequence(&self.canonical_index, target_length) {
            Ok(sequence) => Ok(serde_wasm_bindgen::to_value(&sequence)?),
            Err(e) => Err(JsValue::from_str(&format!("Sequence Error: {}", e))),
        }
    }

    /// Calculates the minimum number of 24-bit elements required to represent the current canonical index.
    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self) -> u32 {
        calculate_min_sequence_length_rust(&self.canonical_index)
    }
}

// --- Text Conversion Logic (Rust) ---

/// Converts a text string into a BigInt index based on a predefined character set.
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

/// Converts a BigInt index into a text string using the predefined character set.
/// Negative indices are not supported.
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
        let remainder = (&temp_index % &*CHAR_SET_BASE).to_usize().ok_or_else(|| anyhow!("Remainder for character index too large to convert"))?;
        let char_index = remainder;

        if char_index < CHAR_SET_ARRAY.len() {
            text.insert(0, CHAR_SET_ARRAY[char_index]);
        } else {
            text.insert(0, '?');
            console::warn_1(&format!("Index to text: Character index {} out of bounds for CHAR_SET_ARRAY (len {}).", char_index, CHAR_SET_ARRAY.len()).into());
        }
        temp_index /= &*CHAR_SET_BASE;
    }
    Ok(text)
}

// --- Numerical Sequence (24-bit elements) Conversion ---

/// Converts a sequence of 24-bit unsigned integers into a BigInt index.
fn sequence_to_index(sequence_array: &[u32]) -> Result<BigInt> {
    let mut index = BigInt::zero();
    for &value in sequence_array {
        let val_bigint = BigInt::from(value);
        if val_bigint.sign() == Sign::Minus || val_bigint > *SEQUENCE_MAX_VAL_BIGINT {
            bail!("Sequence value {} is out of the 0 to {} range for {}-bit elements.",
                  value, *SEQUENCE_MAX_VAL_BIGINT, NUM_SEQUENCE_ELEMENT_BIT_WIDTH);
        }
        index = index * &*SEQUENCE_ELEMENT_BASE + val_bigint;
    }
    Ok(index)
}

/// Converts a BigInt index into a sequence of 24-bit unsigned integers.
/// `target_length` determines the length, padding with leading zeros if necessary.
/// Negative indices are not supported.
fn index_to_sequence(index: &BigInt, target_length: u32) -> Result<Vec<u32>> {
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
        bail!("Cannot represent a non-zero index with a target sequence length of 0.");
    }
    if index.sign() == Sign::Minus {
        bail!("Negative index to sequence conversion is not supported.");
    }

    let mut temp_index = index.clone();

    for _ in 0..target_length {
        let remainder = (&temp_index % &*SEQUENCE_ELEMENT_BASE).to_u32().ok_or_else(|| anyhow!("Remainder for sequence element too large to convert"))?;
        decoded_sequence.insert(0, remainder);
        temp_index /= &*SEQUENCE_ELEMENT_BASE;
    }

    if temp_index > BigInt::zero() {
        bail!("Index {} is too large to be represented by a sequence of length {}.", index, target_length);
    }
    Ok(decoded_sequence)
}

/// Calculates the minimum number of 24-bit elements required to represent the given index.
fn calculate_min_sequence_length_rust(index: &BigInt) -> u32 {
    if index.is_zero() {
        return 0;
    }
    let mut length = 0;
    let mut temp_index = index.clone();
    if temp_index.sign() == Sign::Minus {
        temp_index = temp_index.abs();
    }

    loop {
        temp_index /= &*SEQUENCE_ELEMENT_BASE;
        length += 1;
        if temp_index.is_zero() {
            break;
        }
    }
    length
}