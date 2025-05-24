use wasm_bindgen::prelude::*;
use js_sys::BigInt as JsBigInt;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, ToPrimitive, FromPrimitive}; // Removed CheckedDiv as it's not used
use anyhow::{Result as AnyhowResult, bail, anyhow};
use serde::{Deserialize}; // Removed Serialize as it's not directly used by Instruction for now
use serde_json::{json, Value as JsonValue}; 
use std::str::FromStr;
use std::collections::HashMap;

use lazy_static::lazy_static; 
use web_sys::console;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// --- Constants ---
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN: u32 = 1;
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX: u32 = 32;
const SIMPLE_TEXT_ALPHABET_STRING: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const SIMPLE_TEXT_BASE_U32: u32 = 27; 
const PADDING_CHAR: char = ' '; 
const ADDITION_SEARCH_ITERATION_LIMIT: u32 = 10000; 

lazy_static! { 
    static ref CHAR_TO_VAL: HashMap<char, u32> = SIMPLE_TEXT_ALPHABET_STRING.chars().enumerate().map(|(i, c)| (c, i as u32)).collect();
    static ref VAL_TO_CHAR: HashMap<u32, char> = SIMPLE_TEXT_ALPHABET_STRING.chars().enumerate().map(|(i, c)| (i as u32, c)).collect();
    static ref SIMPLE_TEXT_BASE_BIGINT: BigInt = BigInt::from(SIMPLE_TEXT_BASE_U32);
}

#[derive(Deserialize, Debug, Clone)] 
#[serde(tag = "instruction_type")] 
enum Instruction { 
    #[serde(rename = "LITERAL_BIGINT")] LiteralBigInt { value: String },
    #[serde(rename = "LITERAL_TEXT_TO_CI")] LiteralTextToCi { text_value: String, text_modality_alphabet_id: String },
    #[serde(rename = "REPEAT_TEXT_PATTERN_TO_CI")] RepeatTextPatternToCi { pattern_text: String, count: u32, text_modality_alphabet_id: String },
    #[serde(rename = "EVALUATE_ADDITION")] EvaluateAddition { operand1_value: String, operand2_value: String },
}

#[wasm_bindgen]
pub struct AppState { canonical_index: BigInt, }

#[wasm_bindgen]
impl AppState {
    #[wasm_bindgen(constructor)] pub fn new() -> Self { AppState { canonical_index: BigInt::zero() } }
    #[wasm_bindgen(js_name=getCanonicalIndex)] pub fn get_canonical_index(&self)->Result<JsBigInt,JsValue>{JsBigInt::from_str(&self.canonical_index.to_string()).map_err(|e|JsValue::from_str(&format!("RBigIntToJSFail:{:?}",e)))}
    #[wasm_bindgen(js_name=setCanonicalIndex)] pub fn set_canonical_index(&mut self,js_idx:JsBigInt)->Result<(),JsValue>{let s0=js_idx.to_string(10).map_err(|_|JsValue::from_str("JSBigIntStrFail"))?;let s1=s0.as_string().ok_or_else(||JsValue::from_str("JSStr->RustStrFail"))?;match BigInt::from_str(&s1){Ok(i)=>{if i.sign()==Sign::Minus{return Err(JsValue::from_str("CI neg err"));}self.canonical_index=i;Ok(())},Err(e)=>Err(JsValue::from_str(&format!("InvBigIntCI:{}.In:'{}'",e,s1)))}}
    #[wasm_bindgen(js_name=getSequenceRepresentation)] pub fn get_sequence_representation(&self,tl:u32,bd:u32)->Result<JsValue,JsValue>{if bd<SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN||bd>SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX{return Err(JsValue::from_str("InvSeqBitDepth"));}match index_to_sequence_u32_internal(&self.canonical_index,tl,bd){Ok(s)=>serde_wasm_bindgen::to_value(&s).map_err(|e|JsValue::from_str(&format!("SeqSerFail:{}",e))),Err(e)=>Err(JsValue::from_str(&format!("IdxToSeqFail:{}",e)))}}
    #[wasm_bindgen(js_name=calculateMinSequenceLength)] pub fn calculate_min_sequence_length(&self,bd:u32)->Result<u32,JsValue>{if bd<SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN||bd>SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX{return Err(JsValue::from_str("InvSeqBitDepthLenCalc"));}Ok(calculate_min_sequence_length_internal(&self.canonical_index,bd))}
    #[wasm_bindgen(js_name=indexToTextSimple)] pub fn index_to_text_simple(&self)->Result<String,JsValue>{let ml=calculate_min_text_length_simple_internal(&self.canonical_index);let tl=if self.canonical_index.is_zero(){1.max(ml)}else{ml};index_to_text_simple_internal(&self.canonical_index,tl).map_err(|e|JsValue::from_str(&e.to_string()))}
    #[wasm_bindgen(js_name=setIndexFromTextSimple)] pub fn set_index_from_text_simple(&mut self,txt:&str)->Result<(),JsValue>{match text_to_index_simple_internal(txt){Ok(i)=>{self.canonical_index=i;Ok(())},Err(e)=>Err(JsValue::from_str(&e.to_string()))}}
    #[wasm_bindgen(js_name=executeJsonInstructionsToCI)] pub fn execute_json_instructions_to_ci(&self,json_s:&str)->Result<JsBigInt,JsValue>{ console::log_1(&format!("execute_json: Received JSON string: {}", json_s).into());let instr:Instruction=serde_json::from_str(json_s).map_err(|e|JsValue::from_str(&format!("JSONParseErr:{}",e)))?;console::log_1(&format!("execute_json: Parsed instruction: {:?}", instr).into());match instr{Instruction::LiteralBigInt{value}=>{let bi=BigInt::from_str(&value).map_err(|e|JsValue::from_str(&format!("LitBigIntParseErr:{}",e)))?;if bi.sign()==Sign::Minus{return Err(JsValue::from_str("LitBigIntNegErr"));}JsBigInt::from_str(&bi.to_string()).map_err(|e|JsValue::from_str(&format!("LitBigIntToJSFail:{:?}",e)))},Instruction::LiteralTextToCi{text_value,text_modality_alphabet_id}=>{if text_modality_alphabet_id!="SIMPLE_TEXT_A_Z_SPACE"{return Err(JsValue::from_str("UnsuppTxtModId"));}let bi=text_to_index_simple_internal(&text_value).map_err(|e|JsValue::from_str(&format!("LitTxtToCIConvErr:{}",e)))?;JsBigInt::from_str(&bi.to_string()).map_err(|e|JsValue::from_str(&format!("LitTxtToCIResToJSFail:{:?}",e)))},Instruction::RepeatTextPatternToCi{pattern_text,count,text_modality_alphabet_id}=>{if text_modality_alphabet_id!="SIMPLE_TEXT_A_Z_SPACE"{return Err(JsValue::from_str("UnsuppTxtModId"));}if pattern_text.is_empty()||count==0{return JsBigInt::from_str("0").map_err(|e|JsValue::from_str(&format!("Rep0EmptyToJSFail:{:?}",e)));}let ft=pattern_text.repeat(count as usize);let bi=text_to_index_simple_internal(&ft).map_err(|e|JsValue::from_str(&format!("RepTxtPattToCIConvErr:{}",e)))?;JsBigInt::from_str(&bi.to_string()).map_err(|e|JsValue::from_str(&format!("RepTxtPattToCIResToJSFail:{:?}",e)))},Instruction::EvaluateAddition{operand1_value,operand2_value}=>{console::log_1(&format!("execute_json:EvalAdd:{} + {}",operand1_value,operand2_value).into());let op1=BigInt::from_str(&operand1_value).map_err(|e|JsValue::from_str(&format!("ADDop1ParseErr:{}",e)))?;let op2=BigInt::from_str(&operand2_value).map_err(|e|JsValue::from_str(&format!("ADDop2ParseErr:{}",e)))?;let sum=op1+op2;if sum.sign()==Sign::Minus{return Err(JsValue::from_str("ADDresNegErr"));}JsBigInt::from_str(&sum.to_string()).map_err(|e|JsValue::from_str(&format!("ADDresToJSFail:{:?}",e)))}}}

    #[wasm_bindgen(js_name = generateJsonAnalysisReportForCurrentCI)]
    pub fn generate_json_analysis_report_for_current_ci(&self, _strategy: String) -> Result<String, JsValue> {
        console::log_1(&format!("generateReport: CI_M_target = {}", self.canonical_index.to_string()).into());

        let ci_target = &self.canonical_index; 
        let ci_target_str = ci_target.to_string();
        let mut analyses: Vec<JsonValue> = Vec::new();
        
        let mut best_cost: usize = usize::MAX;
        let mut recommended_instruction_json_value: JsonValue = json!(null);

        let lit_bi_instr=json!({"instruction_type":"LITERAL_BIGINT","value":ci_target_str.clone()});
        if let Ok(s)=serde_json::to_string(&lit_bi_instr){best_cost=s.len();recommended_instruction_json_value=lit_bi_instr.clone();analyses.push(json!({"lens_id":"LITERAL_BIGINT","instruction":lit_bi_instr.clone(),"estimated_cost":best_cost}));console::log_1(&format!("generateReport:LensLITERAL_BIGINTcost:{}",best_cost).into());}

        match self.index_to_text_simple() {
            Ok(text_repr_raw) => {
                let text_repr = if text_repr_raw.is_empty() { PADDING_CHAR.to_string() } else { text_repr_raw };
                let lit_txt_instr=json!({"instruction_type":"LITERAL_TEXT_TO_CI","text_value":text_repr.clone(),"text_modality_alphabet_id":"SIMPLE_TEXT_A_Z_SPACE"});
                if let Ok(s)=serde_json::to_string(&lit_txt_instr){let cost=s.len();analyses.push(json!({"lens_id":"LITERAL_TEXT_A_Z_SPACE","instruction":lit_txt_instr.clone(),"estimated_cost":cost}));console::log_1(&format!("generateReport:LensLITERAL_TEXTcost:{}",cost).into());if cost<best_cost{best_cost=cost;recommended_instruction_json_value=lit_txt_instr.clone();console::log_1(&"generateReport:LITERAL_TEXTnewBest.".into());}}
                if let Some((p,c))=find_simple_repetition(&text_repr){if c>1{let rep_txt_instr=json!({"instruction_type":"REPEAT_TEXT_PATTERN_TO_CI","pattern_text":p.clone(),"count":c,"text_modality_alphabet_id":"SIMPLE_TEXT_A_Z_SPACE"});if let Ok(s)=serde_json::to_string(&rep_txt_instr){let cost=s.len();analyses.push(json!({"lens_id":"REPEAT_TEXT_A_Z_SPACE_P_N","instruction":rep_txt_instr.clone(),"estimated_cost":cost}));console::log_1(&format!("generateReport:LensREPEAT_TEXTcost:{}.P:'{}',N:{}",cost,p,c).into());if cost<best_cost{best_cost=cost;recommended_instruction_json_value=rep_txt_instr.clone();console::log_1(&"generateReport:REPEAT_TEXTnewBest.".into());}}}}
            },
            Err(e) => { console::log_1(&format!("generateReport: Err CI to text for analysis, skipping text lenses. Err: {:?}", e).into()); }
        }

        console::log_1(&"generateReport: Starting EVALUATE_ADDITION lens.".into());
        if *ci_target > BigInt::one() { 
            let two = BigInt::from(2u32);
            let mut a = BigInt::one();
            let mut iterations = 0u32;
            loop {
                if iterations >= ADDITION_SEARCH_ITERATION_LIMIT { console::log_1(&format!("generateReport: ADDITION lens reached iteration limit ({})", ADDITION_SEARCH_ITERATION_LIMIT).into()); break; }
                let limit_a = ci_target.checked_div(&two).unwrap_or_else(|| ci_target.clone()); 
                if a > limit_a { break; }
                let b = ci_target - &a; 
                // CORRECTED LINES: Use Sign::Minus
                if a.sign() == Sign::Minus || b.sign() == Sign::Minus {
                    a += BigInt::one(); iterations += 1; continue;
                }
                let add_instr = json!({ "instruction_type": "EVALUATE_ADDITION", "operand1_value": a.to_string(), "operand2_value": b.to_string() });
                if let Ok(add_instr_str) = serde_json::to_string(&add_instr) {
                    let current_cost = add_instr_str.len();
                    if current_cost < best_cost { 
                        analyses.push(json!({ "lens_id": "EVALUATE_ADDITION_A_B", "instruction": add_instr.clone(), "estimated_cost": current_cost, "details": {"A": a.to_string(), "B": b.to_string()} }));
                        best_cost = current_cost; recommended_instruction_json_value = add_instr.clone();
                        console::log_1(&format!("generateReport: EVALUATE_ADDITION new best (A={}, B={}), cost: {}", a.to_string(), b.to_string(), current_cost).into());
                    }
                }
                if a == limit_a && ci_target % &two != BigInt::zero() { break; }
                a += BigInt::one(); iterations += 1;
            }
        } else { console::log_1(&"generateReport: CI_target <= 1, skipping EVALUATE_ADDITION lens.".into()); }
        
        let report = json!({ "ci_analyzed": ci_target_str, "analysis_by_lens": analyses, "recommended_instruction_for_save": recommended_instruction_json_value });
        console::log_1(&format!("generateReport: Final rec type: {}", report["recommended_instruction_for_save"]["instruction_type"]).into());
        serde_json::to_string_pretty(&report).map_err(|e| JsValue::from_str(&format!("FailSerFinalReport:{}",e)))
    }
}

// --- Internal Helper Functions ---
fn find_simple_repetition(text:&str)->Option<(String,u32)>{let len=text.len();if len==0{return None;}if len==1&&text.chars().next().unwrap_or_default()==PADDING_CHAR{return None;}for pl in 1..=(len/2){if len%pl==0{let ptn=&text[0..pl];let cnt=(len/pl)as u32;let mut im=true;for i in 1..cnt{let si=(i*pl as u32)as usize;let ei=si+pl;if&text[si..ei]!=ptn{im=false;break;}}if im{return Some((ptn.to_string(),cnt));}}}None}
fn text_to_index_simple_internal(text:&str)->AnyhowResult<BigInt>{let mut i=BigInt::zero();let b=&*SIMPLE_TEXT_BASE_BIGINT;for c_in_t in text.chars(){let cv=CHAR_TO_VAL.get(&c_in_t.to_ascii_uppercase()).ok_or_else(||anyhow!("Char '{}' not in alpha '{}'",c_in_t,SIMPLE_TEXT_ALPHABET_STRING))?;i=i*b+BigInt::from(*cv);}Ok(i)}
fn index_to_text_simple_internal(idx:&BigInt,tl:u32)->AnyhowResult<String>{if idx.sign()==Sign::Minus{bail!("Neg idx to txt fail.");}if tl==0{if !idx.is_zero(){bail!("TL0 for non-zero idx('{}') invalid.",idx);}return Ok(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR).to_string());}let mut ti=idx.clone();let b=&*SIMPLE_TEXT_BASE_BIGINT;let mut cs:Vec<char>=Vec::new();if ti.is_zero(){for _ in 0..tl{cs.push(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR));}return Ok(cs.into_iter().collect());}loop{let rv=(ti.clone()%b).to_u32().ok_or_else(||anyhow!("Rem too big for u32. Idx:{}, Base:{}",ti,b))?;ti/=b;cs.push(VAL_TO_CHAR.get(&rv).copied().unwrap_or('?'));if ti.is_zero(){break;}}while cs.len()<tl as usize{cs.push(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR));}Ok(cs.into_iter().rev().collect())}
fn calculate_min_text_length_simple_internal(idx:&BigInt)->u32{if idx.is_zero(){return 0;}if idx.sign()==Sign::Minus{return u32::MAX;}let mut l=0u32;let mut ti=idx.clone();let b=&*SIMPLE_TEXT_BASE_BIGINT;if b<=&BigInt::one(){return u32::MAX;}loop{ti/=b;l+=1;if ti.is_zero(){break;}if l==u32::MAX{break;}}l}
fn index_to_sequence_u32_internal(idx:&BigInt,tl:u32,bd:u32)->AnyhowResult<Vec<u32>>{if idx.sign()==Sign::Minus{bail!("Neg CI('{}') for seq n/a.",idx);}if idx.is_zero(){return Ok(vec![0u32;tl as usize]);}if tl==0{bail!("Non-zero CI('{}') needs TL>0 for seq.",idx);}let b=BigInt::one()<<bd;let mut s=vec![0u32;tl as usize];let mut ti=idx.clone();for i in(0..tl).rev(){let r=ti.clone()%&b;ti/=&b;s[i as usize]=r.to_u32().ok_or_else(||anyhow!("Val '{}' too big for u32(idx{},bd{}).",r,i,bd))?;}if !ti.is_zero(){bail!("Idx '{}' too big for seq len {}(bd{}).Rem:'{}'",idx,tl,bd,ti);}Ok(s)}
fn calculate_min_sequence_length_internal(idx:&BigInt,bd:u32)->u32{if idx.is_zero(){return 0;}let b=BigInt::one()<<bd;let mut l=0u32;let mut ti=idx.clone();if b<=BigInt::one(){return u32::MAX;}loop{ti/=&b;l+=1;if ti.is_zero(){break;}if l==u32::MAX{break;}}l}