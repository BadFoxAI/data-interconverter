use wasm_bindgen::prelude::*;
use js_sys::BigInt as JsBigInt; // JavaScript BigInt type from js_sys.
use num_bigint::{BigInt, Sign};  // Rust's arbitrary-precision BigInt type.
use num_traits::{Zero, One, ToPrimitive, Pow}; // Traits for numeric operations.
use anyhow::{Result as AnyhowResult, bail, anyhow}; // Error handling for internal Rust functions.
use serde::Serialize; // For serializing Rust types to JsValue.
use std::str::FromStr; // For parsing strings into numbers.

// Enable panic messages to be logged to the browser console in debug builds.
#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    // This function is called once when the WASM module starts.
    // It sets up a hook that forwards Rust panics to `console.error`.
    console_error_panic_hook::set_once();
}

// Defines the supported range of bit depths for sequence elements when using u32.
// This range (1-32) ensures that element values fit within a u32 and can be
// represented precisely as JavaScript Numbers (since u32::MAX < Number.MAX_SAFE_INTEGER).
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN: u32 = 1;
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX: u32 = 32;

/// Main application state, holding the central Canonical Index.
/// All data representations are derived from `canonical_index`.
#[wasm_bindgen]
pub struct AppState {
    canonical_index: BigInt,
}

#[wasm_bindgen]
impl AppState {
    /// Constructor for AppState.
    /// Initializes the `canonical_index` to zero.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AppState {
            canonical_index: BigInt::zero(),
        }
    }

    /// Retrieves the current `canonical_index` as a JavaScript BigInt.
    /// Returns an error (JsValue) if the conversion from Rust's BigInt to JS BigInt fails.
    #[wasm_bindgen(js_name = getCanonicalIndex)]
    pub fn get_canonical_index(&self) -> Result<JsBigInt, JsValue> {
        // Convert the internal Rust BigInt to a string, then parse that string into a JsBigInt.
        // This is a common and robust way to transfer BigInts across the WASM boundary.
        JsBigInt::from_str(&self.canonical_index.to_string())
            .map_err(|js_err| JsValue::from_str(&format!("Failed to convert Rust BigInt to JS BigInt: {:?}", js_err)))
    }

    /// Sets the `canonical_index` from a JavaScript BigInt.
    /// The provided index must be non-negative.
    /// Returns an error (JsValue) if parsing fails or if the index is negative.
    #[wasm_bindgen(js_name = setCanonicalIndex)]
    pub fn set_canonical_index(&mut self, js_index: JsBigInt) -> Result<(), JsValue> {
        // Convert the incoming JsBigInt to a JS String (radix 10).
        let index_str_js = js_index.to_string(10)
            .map_err(|_e| JsValue::from_str("Failed to stringify JS BigInt (radix 10)."))?;
        
        // Convert the JS String to a Rust String.
        let index_str_rust = index_str_js.as_string()
            .ok_or_else(|| JsValue::from_str("JS BigInt to_string did not return a valid string representation."))?;

        // Parse the Rust String into a num_bigint::BigInt.
        match BigInt::from_str(&index_str_rust) {
            Ok(index) => {
                // Ensure the parsed index is not negative.
                if index.sign() == Sign::Minus {
                    return Err(JsValue::from_str("Canonical Index cannot be negative."));
                }
                self.canonical_index = index;
                Ok(())
            },
            Err(e) => Err(JsValue::from_str(&format!("Invalid BigInt format for Canonical Index: {}. Input string was: '{}'", e, index_str_rust)))
        }
    }

    /// Converts the `canonical_index` into a sequence of u32 values,
    /// representing elements of a given `bit_depth`.
    /// The resulting `Vec<u32>` is serialized to a JavaScript Array of Numbers.
    /// `target_length` specifies the desired length of the output sequence.
    /// `bit_depth` must be between `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN` and `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX`.
    /// Returns an error (JsValue) if parameters are invalid or if the conversion/serialization fails.
    #[wasm_bindgen(js_name = getSequenceRepresentation)]
    pub fn get_sequence_representation(&self, target_length: u32, bit_depth: u32) -> Result<JsValue, JsValue> {
        // Validate the requested bit_depth.
        if bit_depth < SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN || bit_depth > SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX {
             return Err(JsValue::from_str(&format!(
                "Sequence element bit depth must be between {} and {}. Got: {}", 
                SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN, SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX, bit_depth
            )));
        }
        
        // Perform the conversion from BigInt to a Vec<u32> using an internal helper function.
        match index_to_sequence_u32_internal(&self.canonical_index, target_length, bit_depth) {
            Ok(sequence_of_u32s) => {
                // Serialize the Vec<u32> into a JsValue. serde_wasm_bindgen handles this conversion
                // to a JavaScript Array containing Numbers. This is precise because all u32 values
                // fit within JS Number's safe integer range.
                serde_wasm_bindgen::to_value(&sequence_of_u32s)
                    .map_err(|e| JsValue::from_str(&format!("Failed to serialize Vec<u32> to JsValue: {}", e)))
            },
            Err(e) => Err(JsValue::from_str(&format!("Error in index_to_sequence_u32_internal: {}", e))),
        }
    }

    /// Calculates the minimum number of elements required to represent the
    /// current `canonical_index`, given a specific `bit_depth` for each element.
    /// `bit_depth` must be between `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN` and `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX`.
    /// Returns an error (JsValue) if `bit_depth` is invalid.
    #[wasm_bindgen(js_name = calculateMinSequenceLength)]
    pub fn calculate_min_sequence_length(&self, bit_depth: u32) -> Result<u32, JsValue> {
        // Validate the requested bit_depth.
         if bit_depth < SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN || bit_depth > SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX {
             return Err(JsValue::from_str(&format!(
                "Bit depth for sequence length calculation must be between {} and {}. Got: {}",
                SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN, SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX, bit_depth
            )));
        }
        // Use an internal helper function for the actual calculation.
        Ok(calculate_min_sequence_length_internal(&self.canonical_index, bit_depth))
    }
}

// --- Internal Helper Functions ---

/// Internal Rust function to convert a BigInt index to a sequence of u32s.
/// The sequence is generated such that `sequence[0]` is the most significant element
/// (big-endian style for the sequence elements).
///
/// # Arguments
/// * `index` - The non-negative BigInt to convert.
/// * `target_length` - The exact desired length of the output sequence.
/// * `bit_depth` - The number of bits per element (must be 1-32, validated by caller).
///
/// # Returns
/// An `AnyhowResult` containing the `Vec<u32>` sequence or an error describing the failure.
fn index_to_sequence_u32_internal(index: &BigInt, target_length: u32, bit_depth: u32) -> AnyhowResult<Vec<u32>> {
    // Pre-condition check: index must not be negative. (Sign::Minus case)
    // This function assumes bit_depth is already validated (1-32).
    if index.sign() == Sign::Minus { 
        bail!("Negative canonical index ('{}') to sequence conversion is not supported.", index); 
    }

    // Handle the case of index 0.
    if index.is_zero() {
        // For an index of 0, the sequence consists of `target_length` zero values.
        // If target_length is 0 (as determined by calculate_min_sequence_length for index 0),
        // this correctly returns an empty Vec.
        return Ok(vec![0u32; target_length as usize]);
    }

    // If the index is non-zero, a target_length of 0 is invalid.
    // This case should ideally be prevented by UI logic that uses calculate_min_sequence_length,
    // as that function will return a length > 0 for a non-zero index.
    if target_length == 0 { // && !index.is_zero() is implied by the check above.
        bail!("Cannot represent a non-zero index ('{}') with a target sequence length of 0.", index);
    }
    
    // Calculate the base for each element in the sequence (e.g., 2^8 for 8-bit elements).
    // `BigInt::one() << bit_depth` is equivalent to `2.pow(bit_depth)`.
    let sequence_element_base: BigInt = BigInt::one() << bit_depth;

    // Initialize the sequence vector with zeros.
    let mut decoded_sequence: Vec<u32> = vec![0u32; target_length as usize];
    let mut temp_index = index.clone(); // Clone the index for mutable operations.

    // Iterate from the least significant element of the conceptual number (rightmost)
    // towards the most significant, placing them into the vector from the end (MSB of sequence).
    // This corresponds to standard big-endian representation of the sequence.
    for i in (0..target_length).rev() { // Iterates from target_length-1 down to 0.
        // Get the current element's value (remainder) and update the index for the next iteration.
        let remainder_bigint = &temp_index % &sequence_element_base;
        temp_index /= &sequence_element_base;
        
        // Convert the BigInt remainder to u32.
        // This conversion is safe because `remainder_bigint` will be `< sequence_element_base`.
        // If `bit_depth <= 32`, then `sequence_element_base <= 2^32`, so `remainder_bigint` fits in u32.
        decoded_sequence[i as usize] = remainder_bigint.to_u32().ok_or_else(|| {
            // This error should ideally not occur if bit_depth is correctly constrained.
            // It would imply remainder_bigint > u32::MAX, which means sequence_element_base was > 2^32.
            anyhow!(
                "Sequence element value '{}' (for output index {}) could not be converted to u32. This indicates an unexpected issue as bit_depth is {}. Base was {}.", 
                remainder_bigint, i, bit_depth, sequence_element_base
            )
        })?;
    }

    // After filling all elements in the sequence, if `temp_index` is still not zero,
    // it means the original index was too large to be represented by the given
    // `target_length` and `bit_depth`. This state should ideally be prevented if
    // `target_length` is correctly determined by `calculate_min_sequence_length_internal`.
    if !temp_index.is_zero() {
        bail!(
            "Index '{}' is too large to be represented by a sequence of length {} with {}-bit elements. Remainder after conversion: '{}'", 
            index, target_length, bit_depth, temp_index
        );
    }

    Ok(decoded_sequence)
}

/// Internal Rust function to calculate the minimum sequence length required
/// to represent a given BigInt index with elements of a specific bit_depth.
///
/// # Arguments
/// * `index` - The non-negative BigInt.
/// * `bit_depth` - The number of bits per element (must be 1-32, validated by caller).
///
/// # Returns
/// The minimum length (u32) of the sequence. Returns 0 if the index is zero.
fn calculate_min_sequence_length_internal(index: &BigInt, bit_depth: u32) -> u32 {
    // This function assumes bit_depth is already validated (1-32).
    // If bit_depth were 0, it would lead to division by zero or an infinite loop.

    if index.is_zero() { 
        return 0; // An index of 0 is represented by an empty sequence (0 elements).
    }

    // The base for each element. Since bit_depth >= 1, sequence_element_base will be >= 2.
    let sequence_element_base: BigInt = BigInt::one() << bit_depth;

    let mut length = 0u32;
    let mut temp_index = index.clone();
    
    // Repeatedly divide the index by the base until it becomes zero, counting each division as an element.
    loop {
        temp_index /= &sequence_element_base;
        length += 1;
        if temp_index.is_zero() { break; } // Stop when the index is fully decomposed.
        
        // Safety break for extremely large numbers combined with very small bit_depths
        // to prevent u32 overflow for `length` or extremely long processing.
        // A length of u32::MAX corresponds to over 4 billion elements.
        if length == u32::MAX { break; } 
    }
    length
}