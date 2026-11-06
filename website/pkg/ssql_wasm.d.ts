/* tslint:disable */
/* eslint-disable */

/**
 * Compile an SSQL query + optional `schema.सं` DDL string to SQL and return
 * a rich JSON response containing generated SQL, errors, lint diagnostics,
 * span-accurate structured diagnostics, Adhyetā token breakdowns, and resolved schema tables.
 *
 * Whether the query is accepted, and the SQL it compiles to, come only from
 * the core compiler pipeline the CLI runs (`Compiler::load_schema_from_ssql_string`
 * + `Compiler::compile_string`): the playground has no grammar of its own.
 * Everything else in the response (lint display, token breakdown, schema
 * summary, line/column placement of the compiler's message) is presentation.
 */
export function compile_ssql_wasm(source: string, schema_source: string, dialect: string): string;

/**
 * Format SSQL source code using the canonical SSQL formatter (`--fmt`).
 */
export function format_ssql_wasm(source: string): string;

/**
 * Run the SSQL style and linguistic purity linter (`--lint`) and return JSON diagnostics.
 */
export function lint_ssql_wasm(source: string): string;

/**
 * Reverse-transpile standard SQL into idiomatic SSQL (`--from-sql`) using the active `schema.सं`.
 */
export function transpile_sql_to_ssql_wasm(sql_source: string, schema_source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile_ssql_wasm: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly format_ssql_wasm: (a: number, b: number, c: number) => void;
    readonly lint_ssql_wasm: (a: number, b: number, c: number) => void;
    readonly transpile_sql_to_ssql_wasm: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
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
