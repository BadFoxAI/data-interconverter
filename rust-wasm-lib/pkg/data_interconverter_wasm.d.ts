/* tslint:disable */
/* eslint-disable */
export function set_panic_hook(): void;
export class AppState {
  free(): void;
  constructor();
  getCanonicalIndex(): bigint;
  setCanonicalIndex(js_idx: bigint): void;
  getSequenceRepresentation(tl: number, bd: number): any;
  calculateMinSequenceLength(bd: number): number;
  indexToTextSimple(): string;
  setIndexFromTextSimple(txt: string): void;
  executeJsonInstructionsToCI(json_string: string): bigint;
  generateJsonAnalysisReportForCurrentCI(strat_str: string): string;
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
  readonly appstate_indexToTextSimple: (a: number) => [number, number, number, number];
  readonly appstate_setIndexFromTextSimple: (a: number, b: number, c: number) => [number, number];
  readonly appstate_executeJsonInstructionsToCI: (a: number, b: number, c: number) => [number, number, number];
  readonly appstate_generateJsonAnalysisReportForCurrentCI: (a: number, b: number, c: number) => [number, number, number, number];
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
