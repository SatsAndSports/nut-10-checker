/* tslint:disable */
/* eslint-disable */
export function main(): void;
export function greet(name: string): string;
/**
 * Get the denominations (amounts) supported by the active keyset
 */
export function get_mint_denominations(mint_url: string): Promise<any>;
/**
 * Check if a mint supports NUT-11 (P2PK)
 */
export function check_nut11_support(mint_url: string): Promise<boolean>;
/**
 * Generate a new wallet seed (12 words)
 */
export function generate_wallet_seed(): string;
/**
 * Create a wallet for a mint
 */
export function create_wallet(mint_url: string, seed_words: string): Promise<any>;
/**
 * Mint tokens from a paid quote
 */
export function mint_tokens(mint_url: string, seed_words: string, quote_id: string): Promise<bigint>;
/**
 * Get wallet balance with breakdown by state
 */
export function get_wallet_balance_by_state(mint_url: string, seed_words: string): Promise<any>;
/**
 * Create a Cashu token from wallet proofs
 */
export function create_token(mint_url: string, seed_words: string, amount: bigint): Promise<string>;
/**
 * Get wallet balance (unspent only)
 */
export function get_wallet_balance(mint_url: string, seed_words: string): Promise<bigint>;
/**
 * Generate two keypairs for testing
 */
export function generate_test_keypairs(): any;
/**
 * Create a mint quote (request to mint tokens)
 */
export function create_mint_quote(mint_url: string, seed_words: string, amount_sat: bigint): Promise<any>;
/**
 * Check if a mint quote has been paid
 */
export function check_mint_quote_status(mint_url: string, seed_words: string, quote_id: string): Promise<boolean>;
/**
 * Swap existing tokens for P2PK 2-of-2 multisig tokens (creates 1, 2 sat proofs)
 */
export function swap_to_p2pk_2of2(mint_url: string, seed_words: string, alice_pubkey: string, bob_pubkey: string, denominations: any, use_sig_all: boolean): Promise<any>;
/**
 * Add proofs to wallet storage as Unspent
 */
export function add_proofs_to_wallet(mint_url: string, seed_words: string, proofs_json: any): Promise<void>;
/**
 * Check specific proofs' states from the mint (without updating local storage)
 */
export function check_specific_proofs_state(mint_url: string, proofs_json: any): Promise<any>;
/**
 * Check proof states from the mint and update local storage
 */
export function check_proofs_state(mint_url: string, seed_words: string): Promise<any>;
/**
 * Attempt to swap (spend) proofs with SigAll signatures
 */
export function swap_with_sig_all(mint_url: string, proofs_json: any, secret_keys: string[]): Promise<any>;
/**
 * Attempt to swap (spend) proofs with individual P2PK signatures
 */
export function swap_with_p2pk(mint_url: string, proofs_json: any, secret_keys: string[]): Promise<any>;
/**
 * Sign proofs with SigAll and return the signed proofs (without submitting)
 * Used for testing - allows examining or reusing signatures
 */
export function sign_proofs_with_sig_all(mint_url: string, proofs_json: any, secret_keys: string[]): Promise<any>;
/**
 * Create and submit a SwapRequest from already-signed proofs
 * Used for testing SigAll - allows submitting a subset of proofs with mismatched signatures
 */
export function submit_signed_proofs(mint_url: string, signed_proofs_json: any): Promise<any>;
/**
 * Create blinded outputs with no spending conditions
 * Returns: { outputs, secrets, blinding_factors }
 */
export function create_blinded_outputs(mint_url: string, amounts: BigUint64Array): Promise<any>;
/**
 * Create blinded outputs with P2PK spending conditions
 * Returns: { outputs, secrets, blinding_factors }
 */
export function create_blinded_outputs_with_p2pk(mint_url: string, amounts: BigUint64Array, spending_conditions_json: any): Promise<any>;
/**
 * Create spending conditions for P2PK 2-of-2 multisig with SigAll
 */
export function create_spending_conditions_p2pk_2of2(pubkey1: string, pubkey2: string, locktime?: bigint | null): any;
/**
 * Create spending conditions with locktime and refund key
 * Before locktime: requires primary_pubkey signature
 * After locktime: requires refund_pubkey signature
 */
export function create_spending_conditions_with_locktime_refund(primary_pubkey: string, refund_pubkey: string, locktime: bigint): any;
/**
 * Create spending conditions with m-of-n multisig before and after locktime
 * Before locktime: requires num_sigs from [primary_pubkey, ...additional_pubkeys]
 * After locktime: requires num_sigs_refund from refund_keys
 */
export function create_spending_conditions_multisig_locktime(primary_pubkey: string, additional_pubkeys: string[], refund_pubkeys: string[], locktime: bigint, num_sigs: bigint, num_sigs_refund: bigint): any;
/**
 * Create a SwapRequest from input proofs and blinded outputs (does not sign)
 */
export function create_swap_request(inputs_json: any, outputs_json: any): any;
/**
 * Sign a SwapRequest with SigAll flag
 */
export function sign_swap_request_sigall(swap_request_json: any, secret_keys: string[]): any;
/**
 * Sign a SwapRequest with individual P2PK signatures (not SigAll)
 */
export function sign_swap_request_p2pk(swap_request_json: any, secret_keys: string[]): any;
/**
 * Submit a SwapRequest to the mint
 */
export function submit_swap_request(mint_url: string, swap_request_json: any): Promise<any>;
/**
 * Unblind signatures to create proofs
 */
export function unblind_signatures(mint_url: string, signatures_json: any, secrets_json: any, blinding_factors_json: any): Promise<any>;
/**
 * Test context that holds mint connection and cached state
 */
export class TestContext {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new test context for a mint
   */
  constructor(mint_url: string, seed_words: string);
  /**
   * Get unspent proofs from wallet that sum to at least the target amount
   * Returns: { proofs, ys } where ys are needed for state management
   */
  get_proofs_to_spend(amount: bigint): Promise<any>;
  /**
   * Create blinded outputs with no spending conditions
   * Returns: { outputs, secrets, blinding_factors }
   */
  create_blinded_outputs(amounts: BigUint64Array): Promise<any>;
  /**
   * Create blinded outputs with spending conditions
   * Returns: { outputs, secrets, blinding_factors }
   */
  create_blinded_outputs_with_conditions(amounts: BigUint64Array, spending_conditions_json: any): Promise<any>;
  /**
   * Submit a swap request and update wallet state
   * Returns: { success, signatures?, error? }
   */
  submit_swap(swap_request_json: any, input_ys_json: any): Promise<any>;
  /**
   * Unblind signatures to create proofs
   */
  unblind_signatures(signatures_json: any, secrets_json: any, blinding_factors_json: any): Promise<any>;
  /**
   * Add proofs to wallet storage as Unspent
   */
  add_proofs_to_wallet(proofs_json: any): Promise<void>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_testcontext_free: (a: number, b: number) => void;
  readonly testcontext_new: (a: number, b: number, c: number, d: number) => any;
  readonly testcontext_get_proofs_to_spend: (a: number, b: bigint) => any;
  readonly testcontext_create_blinded_outputs: (a: number, b: number, c: number) => any;
  readonly testcontext_create_blinded_outputs_with_conditions: (a: number, b: number, c: number, d: any) => any;
  readonly testcontext_submit_swap: (a: number, b: any, c: any) => any;
  readonly testcontext_unblind_signatures: (a: number, b: any, c: any, d: any) => any;
  readonly testcontext_add_proofs_to_wallet: (a: number, b: any) => any;
  readonly main: () => void;
  readonly greet: (a: number, b: number) => [number, number];
  readonly get_mint_denominations: (a: number, b: number) => any;
  readonly check_nut11_support: (a: number, b: number) => any;
  readonly generate_wallet_seed: () => [number, number];
  readonly create_wallet: (a: number, b: number, c: number, d: number) => any;
  readonly mint_tokens: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
  readonly get_wallet_balance_by_state: (a: number, b: number, c: number, d: number) => any;
  readonly create_token: (a: number, b: number, c: number, d: number, e: bigint) => any;
  readonly get_wallet_balance: (a: number, b: number, c: number, d: number) => any;
  readonly generate_test_keypairs: () => [number, number, number];
  readonly create_mint_quote: (a: number, b: number, c: number, d: number, e: bigint) => any;
  readonly check_mint_quote_status: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
  readonly swap_to_p2pk_2of2: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: any, j: number) => any;
  readonly add_proofs_to_wallet: (a: number, b: number, c: number, d: number, e: any) => any;
  readonly check_specific_proofs_state: (a: number, b: number, c: any) => any;
  readonly check_proofs_state: (a: number, b: number, c: number, d: number) => any;
  readonly swap_with_sig_all: (a: number, b: number, c: any, d: number, e: number) => any;
  readonly swap_with_p2pk: (a: number, b: number, c: any, d: number, e: number) => any;
  readonly sign_proofs_with_sig_all: (a: number, b: number, c: any, d: number, e: number) => any;
  readonly submit_signed_proofs: (a: number, b: number, c: any) => any;
  readonly create_blinded_outputs: (a: number, b: number, c: number, d: number) => any;
  readonly create_blinded_outputs_with_p2pk: (a: number, b: number, c: number, d: number, e: any) => any;
  readonly create_spending_conditions_p2pk_2of2: (a: number, b: number, c: number, d: number, e: number, f: bigint) => [number, number, number];
  readonly create_spending_conditions_with_locktime_refund: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number];
  readonly create_spending_conditions_multisig_locktime: (a: number, b: number, c: number, d: number, e: number, f: number, g: bigint, h: bigint, i: bigint) => [number, number, number];
  readonly create_swap_request: (a: any, b: any) => [number, number, number];
  readonly sign_swap_request_sigall: (a: any, b: number, c: number) => [number, number, number];
  readonly sign_swap_request_p2pk: (a: any, b: number, c: number) => [number, number, number];
  readonly submit_swap_request: (a: number, b: number, c: any) => any;
  readonly unblind_signatures: (a: number, b: number, c: any, d: any, e: any) => any;
  readonly rustsecp256k1_v0_10_0_context_create: (a: number) => number;
  readonly rustsecp256k1_v0_10_0_context_destroy: (a: number) => void;
  readonly rustsecp256k1_v0_10_0_default_illegal_callback_fn: (a: number, b: number) => void;
  readonly rustsecp256k1_v0_10_0_default_error_callback_fn: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h57d8dd306e15d6a6: (a: number, b: number) => void;
  readonly closure1034_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure1955_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
