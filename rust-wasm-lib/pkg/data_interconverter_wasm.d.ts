/* tslint:disable */
/* eslint-disable */
export function set_panic_hook(): void;
/**
 * Main application state, holding the central Canonical Index.
 * All data representations are derived from `canonical_index`.
 */
export class AppState {
  free(): void;
  /**
   * Constructor for AppState.
   * Initializes the `canonical_index` to zero.
   */
  constructor();
  /**
   * Retrieves the current `canonical_index` as a JavaScript BigInt.
   * Returns an error (JsValue) if the conversion from Rust's BigInt to JS BigInt fails.
   */
  getCanonicalIndex(): bigint;
  /**
   * Sets the `canonical_index` from a JavaScript BigInt.
   * The provided index must be non-negative.
   * Returns an error (JsValue) if parsing fails or if the index is negative.
   */
  setCanonicalIndex(js_index: bigint): void;
  /**
   * Converts the `canonical_index` into a sequence of u32 values,
   * representing elements of a given `bit_depth`.
   * The resulting `Vec<u32>` is serialized to a JavaScript Array of Numbers.
   * `target_length` specifies the desired length of the output sequence.
   * `bit_depth` must be between `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN` and `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX`.
   * Returns an error (JsValue) if parameters are invalid or if the conversion/serialization fails.
   */
  getSequenceRepresentation(target_length: number, bit_depth: number): any;
  /**
   * Calculates the minimum number of elements required to represent the
   * current `canonical_index`, given a specific `bit_depth` for each element.
   * `bit_depth` must be between `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MIN` and `SUPPORTED_U32_SEQUENCE_BIT_DEPTH_MAX`.
   * Returns an error (JsValue) if `bit_depth` is invalid.
   */
  calculateMinSequenceLength(bit_depth: number): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_appstate_free: (a: number, b: number) => void;
  readonly appstate_new: () => number;
  readonly appstate_getCanonicalIndex: (a: number) => [number, number, number];
  readonly appstate_setCanonicalIndex: (a: number, b: any) => [number, number];
  readonly appstate_getSequenceRepresentation: (a: number, b: number, c: number) => [number, number, number];
  readonly appstate_calculateMinSequenceLength: (a: number, b: number) => [number, number, number];
  readonly set_panic_hook: () => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
