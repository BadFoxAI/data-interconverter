use wasm_bindgen::prelude::*;
use js_sys::BigInt as JsBigInt;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, ToPrimitive, FromPrimitive}; 
use anyhow::{Result as AnyhowResult, bail, anyhow};
use serde::{Deserialize, Serialize}; 
use serde_json::{json, Value as JsonValue}; 
use std::str::FromStr;
use std::collections::HashMap;

use lazy_static::lazy_static; 
use web_sys::console;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() { console_error_panic_hook::set_once(); }

// --- Constants ---
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN: u32 = 1;
const SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX: u32 = 32;
const SIMPLE_TEXT_ALPHABET_STRING: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const SIMPLE_TEXT_BASE_U32: u32 = 27; 
const PADDING_CHAR: char = ' '; 
const ADDITION_SEARCH_ITERATION_LIMIT: u32 = 1000; 
const MAX_ADDITION_ANALYSES_TO_SHOW: usize = 5; 

const INTERNAL_REF_PATTERNS: &[&str] = &[
    " ABCDEFGHIJKLMNOPQRSTUVWXYZ", 
    "AEIOU",
    "HELLO WORLD",
];

lazy_static! { 
    static ref CHAR_TO_VAL: HashMap<char, u32> = SIMPLE_TEXT_ALPHABET_STRING.chars().enumerate().map(|(i, c)| (c, i as u32)).collect();
    static ref VAL_TO_CHAR: HashMap<u32, char> = SIMPLE_TEXT_ALPHABET_STRING.chars().enumerate().map(|(i, c)| (i as u32, c)).collect();
    static ref SIMPLE_TEXT_BASE_BIGINT: BigInt = BigInt::from(SIMPLE_TEXT_BASE_U32);
}

#[derive(Deserialize, Serialize, Debug, Clone)] 
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
    #[wasm_bindgen(constructor)] pub fn new()->Self{AppState{canonical_index:BigInt::zero()}}
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

        // Lens 1: LITERAL_BIGINT (Baseline)
        let lit_bi_instr = json!({"instruction_type":"LITERAL_BIGINT","value":ci_target_str.clone()});
        if let Ok(s)=serde_json::to_string(&lit_bi_instr){
            best_cost=s.len(); 
            recommended_instruction_json_value=lit_bi_instr.clone();
            analyses.push(json!({"lens_id":"LITERAL_BIGINT","instruction":lit_bi_instr.clone(),"estimated_cost":best_cost}));
            console::log_1(&format!("generateReport: Lens LITERAL_BIGINT cost:{}",best_cost).into());
        } else {
            return Err(JsValue::from_str("Failed to serialize initial LITERAL_BIGINT instruction"));
        }

        // Lens 2 & 3: Text-based
        match self.index_to_text_simple() {
            Ok(text_repr_raw) => {
                let text_repr = if text_repr_raw.is_empty() { PADDING_CHAR.to_string() } else { text_repr_raw };
                
                // Lens 2: LITERAL_TEXT_TO_CI
                let lit_txt_instr=json!({"instruction_type":"LITERAL_TEXT_TO_CI","text_value":text_repr.clone(),"text_modality_alphabet_id":"SIMPLE_TEXT_A_Z_SPACE"});
                if let Ok(s)=serde_json::to_string(&lit_txt_instr){
                    let cost=s.len();
                    analyses.push(json!({"lens_id":"LITERAL_TEXT_A_Z_SPACE","instruction":lit_txt_instr.clone(),"estimated_cost":cost}));
                    console::log_1(&format!("generateReport:LensLITERAL_TEXTcost:{}",cost).into());
                    if cost<best_cost{best_cost=cost;recommended_instruction_json_value=lit_txt_instr.clone();console::log_1(&"generateReport:LITERAL_TEXTnewBest.".into());}
                }

                // Lens 3a: Check against INTERNAL_REF_PATTERNS
                console::log_1(&format!("generateReport: Analyzing text_to_analyze: '{}' against internal refs.", text_repr).into());
                for ref_pattern_str_slice in INTERNAL_REF_PATTERNS.iter() {
                    let ref_pattern_str = *ref_pattern_str_slice; 
                    if !text_repr.is_empty() && !ref_pattern_str.is_empty() && text_repr.len() % ref_pattern_str.len() == 0 {
                        let count=(text_repr.len()/ref_pattern_str.len())as u32;
                        if count > 0 { 
                            let reconstructed:String=std::iter::repeat(ref_pattern_str).take(count as usize).collect();
                            if reconstructed==text_repr { 
                                console::log_1(&format!("generateReport:Match! Text is ref_pattern '{}' repeated {} times.",ref_pattern_str,count).into());
                                if count > 1 { // Meaningful repetition
                                    let repeat_instr=json!({"instruction_type":"REPEAT_TEXT_PATTERN_TO_CI","pattern_text":ref_pattern_str.to_string(),"count":count,"text_modality_alphabet_id":"SIMPLE_TEXT_A_Z_SPACE"});
                                    if let Ok(s)=serde_json::to_string(&repeat_instr){
                                        let cost=s.len();
                                        analyses.push(json!({"lens_id":format!("REPEAT_INTERNAL_REF_{}",ref_pattern_str.replace(" ","_")),"instruction":repeat_instr.clone(),"estimated_cost":cost}));
                                        console::log_1(&format!("generateReport:Lens REPEAT_INTERNAL_REF '{}' cost:{}. Count:{}",ref_pattern_str,cost,count).into());
                                        if cost<best_cost{best_cost=cost;recommended_instruction_json_value=repeat_instr.clone();console::log_1(&format!("generateReport:REPEAT_INTERNAL_REF '{}' newBest.",ref_pattern_str).into());}
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Lens 3b: Generic P*N Repetition (find_simple_repetition)
                if let Some((pattern,count))=find_simple_repetition(&text_repr){
                    if count>1{ 
                        let generic_repeat_instr=json!({"instruction_type":"REPEAT_TEXT_PATTERN_TO_CI","pattern_text":pattern.clone(),"count":count,"text_modality_alphabet_id":"SIMPLE_TEXT_A_Z_SPACE"});
                        if let Ok(s)=serde_json::to_string(&generic_repeat_instr){
                            let cost=s.len();
                            let lens_id_str = format!("REPEAT_GENERIC_PN_{}",pattern.replace(" ","_"));
                            // Avoid re-adding if this generic pattern was identical to an internal ref pattern already processed
                            let already_covered_by_internal_ref = INTERNAL_REF_PATTERNS.contains(&pattern.as_str()) &&
                                analyses.iter().any(|a| 
                                    a.get("lens_id").and_then(|id|id.as_str()) == Some(&format!("REPEAT_INTERNAL_REF_{}", pattern.replace(" ","_"))) &&
                                    a.get("instruction").and_then(|i|i.get("count")).and_then(|c|c.as_u64()) == Some(count as u64)
                                );

                            if !already_covered_by_internal_ref {
                                analyses.push(json!({"lens_id":lens_id_str,"instruction":generic_repeat_instr.clone(),"estimated_cost":cost}));
                                console::log_1(&format!("generateReport:Lens REPEAT_GENERIC_PN cost:{}. P:'{}',N:{}",cost,pattern,count).into());
                                if cost<best_cost{best_cost=cost;recommended_instruction_json_value=generic_repeat_instr.clone();console::log_1(&"generateReport:REPEAT_GENERIC_PN newBest.".into());}
                            } else {
                                console::log_1(&format!("generateReport:Skipping REPEAT_GENERIC_PN for P:'{}' as it's covered by internal ref.",pattern).into());
                            }
                        }
                    }
                }
            },
            Err(e)=>{console::log_1(&format!("generateReport:Err CI to text for analysis,skip txt lenses.Err:{:?}",e).into());}
        }

        // Lens 4: EVALUATE_ADDITION
        console::log_1(&"generateReport:Starting EVALUATE_ADDITION lens.".into());
        if *ci_target>BigInt::one(){
            let two=BigInt::from(2u32);
            let mut a=BigInt::one();
            let mut iterations=0u32;
            let mut addition_analyses_count=0;
            loop{
                if iterations>=ADDITION_SEARCH_ITERATION_LIMIT{console::log_1(&format!("generateReport:ADDITION lens reached iteration limit ({})",ADDITION_SEARCH_ITERATION_LIMIT).into());break;}
                // Ensure a does not exceed target/2 to avoid duplicate pairs (A,B) vs (B,A) and for efficiency
                let limit_a = ci_target.checked_div(&two).unwrap_or_else(|| ci_target.clone()); // ci_target / 2
                if a > limit_a { break; }
                let b = ci_target - &a; 
                // Operands should be non-negative. Since a starts at 1 and ci_target > 1, and a <= target/2, b will also be >= 1.
                // So no Sign::Minus check strictly needed here for a,b under these loop conditions.
                
                let add_instr=json!({"instruction_type":"EVALUATE_ADDITION","operand1_value":a.to_string(),"operand2_value":b.to_string()});
                if let Ok(add_instr_str)=serde_json::to_string(&add_instr){
                    let current_cost=add_instr_str.len();
                    // Add to analyses for visibility, up to a limit
                    if addition_analyses_count<MAX_ADDITION_ANALYSES_TO_SHOW {
                        analyses.push(json!({"lens_id":"EVALUATE_ADDITION_A_B","instruction":add_instr.clone(),"estimated_cost":current_cost,"details":{"A":a.to_string(),"B":b.to_string()}}));
                        addition_analyses_count+=1;
                        console::log_1(&format!("generateReport:ADDITION lens adding to analyses (A={}, B={}), cost:{}",a.to_string(),b.to_string(),current_cost).into());
                    }
                    if current_cost<best_cost{
                        best_cost=current_cost; // CRITICAL FIX: Update best_cost
                        recommended_instruction_json_value=add_instr.clone();
                        console::log_1(&format!("generateReport:EVALUATE_ADDITION new best (A={}, B={}), cost:{}",a.to_string(),b.to_string(),current_cost).into());
                        // If we found a new best for addition, we might add it to analyses even if over MAX_ADDITION_ANALYSES_TO_SHOW limit
                        // For now, the above push handles adding it if it's among the first few, or if it becomes the best.
                        // This logic could be refined to ensure the *best* A+B is always in analyses.
                        if addition_analyses_count >= MAX_ADDITION_ANALYSES_TO_SHOW { // If it's a new best but we already showed 5, ensure this best one is also shown
                            let already_added_as_best = analyses.iter().any(|item| {
                                if let Some(instr_val) = item.get("instruction") {
                                    instr_val == &recommended_instruction_json_value
                                } else { false }
                            });
                            if !already_added_as_best { // Add it if this specific best combo wasn't one of the first N shown
                                analyses.push(json!({"lens_id":"EVALUATE_ADDITION_A_B_BEST","instruction":add_instr.clone(),"estimated_cost":current_cost,"details":{"A":a.to_string(),"B":b.to_string()}}));
                            }
                        }
                    }
                }
                // Ensure `a` doesn't create `b < a` if `target` is odd and `a` just passed `target/2`.
                // The `a > limit_a` check at the start of the loop already handles this for the next iteration.
                a+=BigInt::one();iterations+=1;
            }
        } else { console::log_1(&"generateReport:CI_target <= 1,skipping EVALUATE_ADDITION lens.".into());}
        
        let report = json!({ "ci_analyzed": ci_target_str, "analysis_by_lens": analyses, "recommended_instruction_for_save": recommended_instruction_json_value });
        console::log_1(&format!("generateReport:Final rec type:{}",report["recommended_instruction_for_save"]["instruction_type"].as_str().unwrap_or("UNKNOWN_TYPE")).into());
        serde_json::to_string_pretty(&report).map_err(|e| JsValue::from_str(&format!("FailSerFinalReport:{}",e)))
    }

    #[wasm_bindgen(js_name = runInternalValidationSuite)]
    pub fn run_internal_validation_suite(&mut self) -> String { /* ... same as the last correct full version ... */
        let mut report_string=String::new();report_string.push_str("--- Internal Validation Suite V1.2 Starting ---\n");console::log_1(&"--- Internal Validation Suite V1.2 Starting ---".into());let original_ci=self.canonical_index.clone();
        let mut run_one_test=|rs_ref:&mut String,tn:&str,setup_ci_res:AnyhowResult<BigInt>,exp_rec_type:&str,exp_pat:Option<&str>,exp_cnt:Option<u32>|{rs_ref.push_str(&format!("\n--- Test Case: {} ---\n",tn));let setup_ci=match setup_ci_res{Ok(ci)=>ci,Err(e)=>{rs_ref.push_str(&format!(" ERROR setting up CI for test:{}\n",e));console::log_1(&format!("ERROR setting up CI for test'{}':{}",tn,e).into());return;}};rs_ref.push_str(&format!("Setting CI to:{}\n",setup_ci));self.canonical_index=setup_ci.clone();match self.generate_json_analysis_report_for_current_ci("internal_suite".to_string()){Ok(json_report_s)=>{rs_ref.push_str(&format!(" Raw Report JSON(first 500c):\n {}\n...\n",json_report_s.chars().take(500).collect::<String>()));if let Ok(parsed_r)=serde_json::from_str::<JsonValue>(&json_report_s){if let Some(rec_i)=parsed_r.get("recommended_instruction_for_save"){let rec_t=rec_i.get("instruction_type").and_then(|v|v.as_str()).unwrap_or("null_type");rs_ref.push_str(&format!(" Recommended Instr Type:{}\n",rec_t));if rec_t==exp_rec_type{let mut dm=true;if let Some(ep)=exp_pat{if rec_i.get("pattern_text").and_then(|v|v.as_str())!=Some(ep){dm=false;rs_ref.push_str(&format!(" PATTERN MISMATCH! Exp:'{}',Got:{:?}\n",ep,rec_i.get("pattern_text")));}}if let Some(ec)=exp_cnt{if rec_i.get("count").and_then(|v|v.as_u64())!=Some(ec as u64){dm=false;rs_ref.push_str(&format!(" COUNT MISMATCH! Exp:{},Got:{:?}\n",ec,rec_i.get("count")));}}if dm{rs_ref.push_str(&format!(" SUCCESS:Correct instr type('{}')&details.\n",rec_t));}else{rs_ref.push_str(&format!(" FAILURE:Correct type('{}')but details mismatch.\n",rec_t));}}else{rs_ref.push_str(&format!(" FAILURE:Expected rec_instr_type'{}',Got '{}'.\n",exp_rec_type,rec_t));}}else{rs_ref.push_str(" ERROR:No recommended_instruction_for_save in report.\n");}}else{rs_ref.push_str(" ERROR:Could not parse generated JSON report for details.\n");}},Err(e)=>{rs_ref.push_str(&format!(" ERROR generating report:{:?}\n",e.as_string().unwrap_or_default()));}}};
        let az_pt=SIMPLE_TEXT_ALPHABET_STRING;let txt_az_x2=format!("{}{}",az_pt,az_pt);run_one_test(&mut report_string,"Internal AZ Pattern x2",text_to_index_simple_internal(&txt_az_x2), "REPEAT_TEXT_PATTERN_TO_CI",Some(az_pt),Some(2));
        let txt_ababab="ABABAB";run_one_test(&mut report_string,"Generic Text Repeat 'ABABAB'",text_to_index_simple_internal(txt_ababab),"REPEAT_TEXT_PATTERN_TO_CI",Some("AB"),Some(3));
        let ci_200=BigInt::from(200u32);run_one_test(&mut report_string,"Small Number 200",Ok(ci_200),"LITERAL_BIGINT",None,None);
        let ci_0=BigInt::zero();run_one_test(&mut report_string,"Zero CI",Ok(ci_0),"LITERAL_BIGINT",None,None); // Changed expected for CI 0 to LITERAL_BIGINT
        self.canonical_index=original_ci;report_string.push_str("\n--- Internal Validation Suite Finished ---\n");console::log_1(&report_string.clone().into());report_string
    }
}

// --- Internal Helper Functions ---
// (Ensure these are the full, correct versions from your last working state)
fn find_simple_repetition(text:&str)->Option<(String,u32)>{let len=text.len();if len==0{return None;}if len==1&&text.chars().next().unwrap_or_default()==PADDING_CHAR{return None;}for pl in 1..=(len/2){if len%pl==0{let ptn=&text[0..pl];let cnt=(len/pl)as u32;let mut im=true;for i in 1..cnt{let si=(i*pl as u32)as usize;let ei=si+pl;if&text[si..ei]!=ptn{im=false;break;}}if im{return Some((ptn.to_string(),cnt));}}}None}
fn text_to_index_simple_internal(text:&str)->AnyhowResult<BigInt>{let mut i=BigInt::zero();let b=&*SIMPLE_TEXT_BASE_BIGINT;for c_in_t in text.chars(){let cv=CHAR_TO_VAL.get(&c_in_t.to_ascii_uppercase()).ok_or_else(||anyhow!("Char '{}' not in alpha '{}'",c_in_t,SIMPLE_TEXT_ALPHABET_STRING))?;i=i*b+BigInt::from(*cv);}Ok(i)}
fn index_to_text_simple_internal(idx:&BigInt,tl:u32)->AnyhowResult<String>{if idx.sign()==Sign::Minus{bail!("Neg idx to txt fail.");}if tl==0{if !idx.is_zero(){bail!("TL0 for non-zero idx('{}') invalid.",idx);}return Ok(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR).to_string());}let mut ti=idx.clone();let b=&*SIMPLE_TEXT_BASE_BIGINT;let mut cs:Vec<char>=Vec::new();if ti.is_zero(){for _ in 0..tl{cs.push(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR));}return Ok(cs.into_iter().collect());}loop{let rv=(ti.clone()%b).to_u32().ok_or_else(||anyhow!("Rem too big for u32. Idx:{}, Base:{}",ti,b))?;ti/=b;cs.push(VAL_TO_CHAR.get(&rv).copied().unwrap_or('?'));if ti.is_zero(){break;}}while cs.len()<tl as usize{cs.push(VAL_TO_CHAR.get(&0).copied().unwrap_or(PADDING_CHAR));}Ok(cs.into_iter().rev().collect())}
fn calculate_min_text_length_simple_internal(idx:&BigInt)->u32{if idx.is_zero(){return 0;}if idx.sign()==Sign::Minus{return u32::MAX;}let mut l=0u32;let mut ti=idx.clone();let b=&*SIMPLE_TEXT_BASE_BIGINT;if b<=&BigInt::one(){return u32::MAX;}loop{ti/=b;l+=1;if ti.is_zero(){break;}if l==u32::MAX{break;}}l}
fn index_to_sequence_u32_internal(idx:&BigInt,tl:u32,bd:u32)->AnyhowResult<Vec<u32>>{if idx.sign()==Sign::Minus{bail!("Neg CI('{}') for seq n/a.",idx);}if idx.is_zero(){return Ok(vec![0u32;tl as usize]);}if tl==0{bail!("Non-zero CI('{}') needs TL>0 for seq.",idx);}let b=BigInt::one()<<bd;let mut s=vec![0u32;tl as usize];let mut ti=idx.clone();for i in(0..tl).rev(){let r=ti.clone()%&b;ti/=&b;s[i as usize]=r.to_u32().ok_or_else(||anyhow!("Val '{}' too big for u32(idx{},bd{}).",r,i,bd))?;}if !ti.is_zero(){bail!("Idx '{}' too big for seq len {}(bd{}).Rem:'{}'",idx,tl,bd,ti);}Ok(s)}
fn calculate_min_sequence_length_internal(idx:&BigInt,bd:u32)->u32{if idx.is_zero(){return 0;}let b=BigInt::one()<<bd;let mut l=0u32;let mut ti=idx.clone();if b<=BigInt::one(){return u32::MAX;}loop{ti/=&b;l+=1;if ti.is_zero(){break;}if l==u32::MAX{break;}}l}