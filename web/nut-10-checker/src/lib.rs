use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use bip39::Mnemonic;

// CDK imports
use cdk::nuts::{SecretKey, CurrencyUnit, PublicKey, SpendingConditions, Conditions, SigFlag, SwapRequest, Proofs, BlindedMessage, Nut10Secret};
use cdk::wallet::{HttpClient, MintConnector, WalletBuilder};
use cdk::Amount;
use cdk_common::mint_url::MintUrl;
use cdk::dhke::{blind_message, construct_proofs};
use cdk::secret::Secret;
use std::str::FromStr;

mod wallet_db;
use wallet_db::LocalStorageWalletDatabase;

/// Helper function to select denominations that sum to target
fn select_denominations(target: u64, available: &[u64]) -> Result<Vec<u64>, String> {
    let mut sorted = available.to_vec();
    sorted.sort_by(|a, b| b.cmp(a)); // Sort descending (largest first)

    let mut remaining = target;
    let mut selected = Vec::new();

    for &amount in &sorted {
        while remaining >= amount {
            selected.push(amount);
            remaining -= amount;
        }
    }

    if remaining > 0 {
        return Err(format!("Cannot make {} sat from available denominations {:?}", target, available));
    }

    web_sys::console::log_1(&format!("select_denominations: target={}, available={:?}, selected={:?}",
        target, available, selected).into());

    Ok(selected)
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! NUT-10 Checker is ready.", name)
}

/// Get the denominations (amounts) supported by the active keyset
#[wasm_bindgen]
pub async fn get_mint_denominations(mint_url: String) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let http_client = HttpClient::new(url);

    // Get active keyset
    let keysets = http_client.get_mint_keysets().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get keysets: {:?}", e)))?;
    let active_keyset_id = keysets.keysets.iter()
        .find(|k| k.active)
        .ok_or_else(|| JsValue::from_str("No active keyset found"))?
        .id;

    // Get the keys for the active keyset
    let all_keys = http_client.get_mint_keys().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get mint keys: {:?}", e)))?;
    let keyset = all_keys.iter()
        .find(|k| k.id == active_keyset_id)
        .ok_or_else(|| JsValue::from_str("Active keyset not found in keys"))?;

    // Extract amounts from the keys (BTreeMap<Amount, PublicKey>)
    // Filter to only amounts that fit in JavaScript Number.MAX_SAFE_INTEGER (2^53 - 1)
    const MAX_SAFE_JS_INT: u64 = 9007199254740991; // 2^53 - 1

    let mut amounts: Vec<u64> = keyset.keys.iter()
        .map(|(amt, _pubkey)| u64::from(*amt))
        .filter(|&amt| amt <= MAX_SAFE_JS_INT)
        .collect();
    amounts.sort(); // Sort for consistency

    web_sys::console::log_1(&format!("get_mint_denominations: keyset_id={}, amounts={:?}",
        active_keyset_id, amounts).into());

    Ok(serde_wasm_bindgen::to_value(&amounts)?)
}

/// Check if a mint supports NUT-11 (P2PK)
#[wasm_bindgen]
pub async fn check_nut11_support(mint_url: String) -> Result<bool, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let http_client = HttpClient::new(url);

    let mint_info = http_client.get_mint_info().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get mint info: {:?}", e)))?;

    Ok(mint_info.nuts.nut11.supported)
}

/// Generate a new wallet seed (12 words)
#[wasm_bindgen]
pub fn generate_wallet_seed() -> String {
    use getrandom::getrandom;
    let mut entropy = [0u8; 16];
    getrandom(&mut entropy).expect("Failed to generate entropy");
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("Failed to create mnemonic");
    mnemonic.to_string()
}

/// Create a wallet for a mint
#[wasm_bindgen]
pub async fn create_wallet(
    mint_url: String,
    seed_words: String,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    // Create unique storage key for this mint
    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let _wallet = WalletBuilder::new()
        .mint_url(url)
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    Ok(serde_wasm_bindgen::to_value(&"Wallet created")?)
}

/// Mint tokens from a paid quote
#[wasm_bindgen]
pub async fn mint_tokens(
    mint_url: String,
    seed_words: String,
    quote_id: String,
) -> Result<u64, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    // Create unique storage key for this mint
    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url)
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    // Mint the tokens
    let proofs = wallet.mint(&quote_id, Default::default(), None).await
        .map_err(|e| JsValue::from_str(&format!("Failed to mint: {:?}", e)))?;

    // Sum up the proof amounts
    let total: u64 = proofs.iter().map(|p| u64::from(p.amount)).sum();

    Ok(total)
}

/// Get wallet balance with breakdown by state
#[wasm_bindgen]
pub async fn get_wallet_balance_by_state(
    mint_url: String,
    seed_words: String,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    // Create unique storage key for this mint
    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url.clone())
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    // Get all proofs
    let all_proofs = wallet.localstore
        .get_proofs(
            Some(wallet.mint_url.clone()),
            Some(wallet.unit.clone()),
            None,
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to get proofs: {:?}", e)))?;

    // Calculate balance by state
    let mut unspent: u64 = 0;
    let mut pending: u64 = 0;
    let mut spent: u64 = 0;
    let mut reserved: u64 = 0;
    let mut pending_spent: u64 = 0;

    for proof_info in all_proofs {
        let amount = u64::from(proof_info.proof.amount);
        match proof_info.state {
            cdk_common::nuts::State::Unspent => unspent += amount,
            cdk_common::nuts::State::Pending => pending += amount,
            cdk_common::nuts::State::Spent => spent += amount,
            cdk_common::nuts::State::Reserved => reserved += amount,
            cdk_common::nuts::State::PendingSpent => pending_spent += amount,
        }
    }

    let result = serde_json::json!({
        "unspent": unspent,
        "pending": pending,
        "spent": spent,
        "reserved": reserved,
        "pending_spent": pending_spent,
        "total": unspent + pending + spent + reserved + pending_spent,
    });

    Ok(serde_wasm_bindgen::to_value(&result)?)
}

/// Get wallet balance (unspent only)
#[wasm_bindgen]
pub async fn get_wallet_balance(
    mint_url: String,
    seed_words: String,
) -> Result<u64, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    // Create unique storage key for this mint
    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url)
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    let balance = wallet.total_balance().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get balance: {:?}", e)))?;

    Ok(u64::from(balance))
}

/// Helper function to safely execute a swap with proper state management
///
/// Preconditions:
/// - Input proofs must be in Unspent state
///
/// State transitions:
/// - Success: Unspent → Pending → [deleted]
/// - Failure: Unspent → Pending → Pending (user can later use "Check State" button to query mint)
async fn safe_swap(
    wallet: &cdk::wallet::Wallet,
    http_client: &HttpClient,
    swap_request: SwapRequest,
    input_ys: Vec<PublicKey>,
) -> Result<cdk::nuts::SwapResponse, cdk::error::Error> {
    use cdk::wallet::MintConnector;

    // Mark input proofs as Pending before attempting swap
    web_sys::console::log_1(&format!("safe_swap: Marking {} input proofs as Pending", input_ys.len()).into());
    wallet.localstore
        .update_proofs_state(input_ys.clone(), cdk_common::nuts::State::Pending)
        .await?;

    // Execute swap via HTTP client
    match http_client.post_swap(swap_request).await {
        Ok(response) => {
            // Swap succeeded - delete spent proofs
            web_sys::console::log_1(&format!("safe_swap: Swap successful, deleting {} spent proofs", input_ys.len()).into());
            wallet.localstore.update_proofs(vec![], input_ys).await?;
            Ok(response)
        }
        Err(e) => {
            // Swap failed - leave as Pending
            // User can use "Check State" button to query mint for actual state
            web_sys::console::log_1(&format!("safe_swap: Swap failed ({:?}), leaving as Pending (use Check State to verify)", e).into());
            Err(e)
        }
    }
}

/// Generate two keypairs for testing
#[wasm_bindgen]
pub fn generate_test_keypairs() -> Result<JsValue, JsValue> {
    let alice_secret = SecretKey::generate();
    let alice_pubkey = alice_secret.public_key();

    let bob_secret = SecretKey::generate();
    let bob_pubkey = bob_secret.public_key();

    let result = serde_json::json!({
        "alice_secret": alice_secret.to_string(),
        "alice_pubkey": alice_pubkey.to_string(),
        "bob_secret": bob_secret.to_string(),
        "bob_pubkey": bob_pubkey.to_string(),
    });

    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[derive(Serialize, Deserialize)]
pub struct MintQuoteResult {
    pub quote_id: String,
    pub request: String,  // Lightning invoice
    pub paid: bool,
}

/// Create a mint quote (request to mint tokens)
#[wasm_bindgen]
pub async fn create_mint_quote(
    mint_url: String,
    seed_words: String,
    amount_sat: u64,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    // Create unique storage key for this mint
    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url)
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    // Create quote through wallet (saves to localstore)
    let quote = wallet.mint_quote(Amount::from(amount_sat), Some("NUT-10 Checker".to_string())).await
        .map_err(|e| JsValue::from_str(&format!("Failed to create quote: {:?}", e)))?;

    let result = MintQuoteResult {
        quote_id: quote.id.clone(),
        request: quote.request.clone(),
        paid: false,  // Will be checked by polling
    };

    Ok(serde_wasm_bindgen::to_value(&result)?)
}

/// Check if a mint quote has been paid
#[wasm_bindgen]
pub async fn check_mint_quote_status(
    mint_url: String,
    seed_words: String,
    quote_id: String,
) -> Result<bool, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    // Create unique storage key for this mint
    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url)
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    let response = wallet.mint_quote_state(&quote_id).await
        .map_err(|e| JsValue::from_str(&format!("Failed to check quote status: {:?}", e)))?;

    Ok(matches!(response.state, cdk::nuts::MintQuoteState::Paid | cdk::nuts::MintQuoteState::Issued))
}

/// Swap existing tokens for P2PK 2-of-2 multisig tokens (creates 1, 2, 4 sat proofs)
#[wasm_bindgen]
pub async fn swap_to_p2pk_2of2(
    mint_url: String,
    seed_words: String,
    alice_pubkey: String,
    bob_pubkey: String,
    denominations: JsValue,
) -> Result<JsValue, JsValue> {
    // Convert JsValue to Vec<u64>
    let denominations: Vec<u64> = serde_wasm_bindgen::from_value(denominations)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse denominations: {:?}", e)))?;
    // Hardcoded: create 1, 2, 4 sat P2PK proofs (total 7 sat)
    let p2pk_amounts = vec![1u64, 2, 4];
    let total_p2pk: u64 = p2pk_amounts.iter().sum();

    web_sys::console::log_1(&format!("swap_to_p2pk_2of2: creating P2PK proofs for {:?} (total {} sat)",
        p2pk_amounts, total_p2pk).into());
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url.clone())
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    // Parse pubkeys
    let alice_pk = PublicKey::from_str(&alice_pubkey)
        .map_err(|e| JsValue::from_str(&format!("Invalid Alice pubkey: {:?}", e)))?;
    let bob_pk = PublicKey::from_str(&bob_pubkey)
        .map_err(|e| JsValue::from_str(&format!("Invalid Bob pubkey: {:?}", e)))?;

    // Create 2-of-2 multisig spending conditions (no locktime for testing)
    let conditions = Conditions::new(
        None,                           // No locktime
        Some(vec![bob_pk]),            // Additional pubkey (Bob)
        None,                           // No refund keys
        Some(2),                        // Require 2 signatures
        Some(SigFlag::SigAll),         // SigAll flag
        None,                           // No refund num_sigs
    ).map_err(|e| JsValue::from_str(&format!("Failed to create conditions: {:?}", e)))?;

    let spending_conditions = SpendingConditions::new_p2pk(
        alice_pk,           // Primary key (Alice)
        Some(conditions),
    );

    // Get ONLY unspent proofs from wallet
    let all_proofs = wallet.localstore
        .get_proofs(
            Some(wallet.mint_url.clone()),
            Some(wallet.unit.clone()),
            Some(vec![cdk_common::nuts::State::Unspent]), // Only Unspent proofs
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to get proofs: {:?}", e)))?;

    // Select proofs that sum to at least total_p2pk (7 sat)
    let mut input_proofs = Vec::new();
    let mut input_ys = Vec::new(); // Track Y values for state management
    let mut input_total: u64 = 0;

    for proof_info in all_proofs {
        if input_total >= total_p2pk {
            break;
        }
        input_total += u64::from(proof_info.proof.amount);
        input_ys.push(proof_info.y.clone());
        input_proofs.push(proof_info.proof);
    }

    if input_total < total_p2pk {
        return Err(JsValue::from_str(&format!("Insufficient balance: have {} sat, need {} sat", input_total, total_p2pk)));
    }

    web_sys::console::log_1(&format!("Selected {} sat of inputs for {} sat of P2PK outputs (change: {} sat)",
        input_total, total_p2pk, input_total - total_p2pk).into());

    // Get active keyset
    let http_client2 = HttpClient::new(url.clone());
    let keysets = http_client2.get_mint_keysets().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get keysets: {:?}", e)))?;
    let active_keyset_id = keysets.keysets.iter()
        .find(|k| k.active)
        .ok_or_else(|| JsValue::from_str("No active keyset found"))?
        .id;

    // Create outputs
    let mut blinded_outputs = Vec::new();
    let mut secrets_and_factors = Vec::new();

    // 1. Create P2PK outputs (1, 2, 4 sat)
    for &amt in &p2pk_amounts {
        let nut10_secret: Nut10Secret = spending_conditions.clone().into();
        let secret: Secret = nut10_secret.try_into()
            .map_err(|e| JsValue::from_str(&format!("Failed to create secret: {:?}", e)))?;

        let (blinded_point, blinding_factor) = blind_message(&secret.to_bytes(), None)
            .map_err(|e| JsValue::from_str(&format!("Failed to blind message: {:?}", e)))?;

        blinded_outputs.push(BlindedMessage::new(
            Amount::from(amt),
            active_keyset_id,
            blinded_point,
        ));
        secrets_and_factors.push((secret, blinding_factor));
    }

    // 2. Create change outputs (if needed) - no spending conditions
    let change_amount = input_total - total_p2pk;
    if change_amount > 0 {
        let change_denoms = select_denominations(change_amount, &denominations)
            .map_err(|e| JsValue::from_str(&e))?;

        web_sys::console::log_1(&format!("Creating {} change outputs: {:?}",
            change_denoms.len(), change_denoms).into());

        for amt in change_denoms {
            let change_secret = Secret::generate();
            let (change_blinded, change_factor) = blind_message(&change_secret.to_bytes(), None)
                .map_err(|e| JsValue::from_str(&format!("Failed to blind change: {:?}", e)))?;

            blinded_outputs.push(BlindedMessage::new(
                Amount::from(amt),
                active_keyset_id,
                change_blinded,
            ));
            secrets_and_factors.push((change_secret, change_factor));
        }
    }

    // Create swap request
    let swap_request = SwapRequest::new(input_proofs, blinded_outputs);

    // Execute swap using safe_swap helper (handles state transitions)
    let swap_response = safe_swap(&wallet, &http_client2, swap_request, input_ys)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to swap for P2PK tokens: {:?}", e)))?;

    // Get mint keys to unblind
    let all_keys = http_client2.get_mint_keys().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get mint keys: {:?}", e)))?;
    let mint_keys = all_keys.iter()
        .find(|k| k.id == active_keyset_id)
        .ok_or_else(|| JsValue::from_str("Keyset not found"))?;

    // Unblind signatures to create proofs
    let secrets: Vec<Secret> = secrets_and_factors.iter().map(|(s, _)| s.clone()).collect();
    let factors = secrets_and_factors.iter().map(|(_, f)| f.clone()).collect();

    let all_proofs = construct_proofs(
        swap_response.signatures,
        factors,
        secrets,
        &mint_keys.keys,
    )
    .map_err(|e| JsValue::from_str(&format!("Failed to construct proofs: {:?}", e)))?;

    web_sys::console::log_1(&format!("Unblinded {} proofs total", all_proofs.len()).into());

    // Store change proofs back in wallet (everything after the first 3)
    if all_proofs.len() > 3 {
        let change_proofs = &all_proofs[3..];
        web_sys::console::log_1(&format!("Storing {} change proofs back to wallet", change_proofs.len()).into());

        for change_proof in change_proofs {
            let proof_info = cdk_common::common::ProofInfo::new(
                change_proof.clone(),
                wallet.mint_url.clone(),
                cdk_common::nuts::State::Unspent,
                wallet.unit.clone(),
            )
            .map_err(|e| JsValue::from_str(&format!("Failed to create proof info: {:?}", e)))?;

            wallet.localstore.update_proofs(vec![proof_info], vec![]).await
                .map_err(|e| JsValue::from_str(&format!("Failed to store change: {:?}", e)))?;
        }
    }

    // Return only the P2PK proofs (first 3)
    let p2pk_proofs: Vec<_> = all_proofs.into_iter().take(3).collect();
    web_sys::console::log_1(&format!("Returning {} P2PK proofs: {:?}",
        p2pk_proofs.len(),
        p2pk_proofs.iter().map(|p| u64::from(p.amount)).collect::<Vec<_>>()).into());

    Ok(serde_wasm_bindgen::to_value(&p2pk_proofs)?)
}

#[derive(Serialize, Deserialize)]
pub struct SwapResult {
    pub success: bool,
    pub error: Option<String>,
    pub proofs: Option<Proofs>,
}

/// Add proofs to wallet storage as Unspent
#[wasm_bindgen]
pub async fn add_proofs_to_wallet(
    mint_url: String,
    seed_words: String,
    proofs_json: JsValue,
) -> Result<(), JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url.clone())
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client)
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    // Deserialize proofs
    let proofs: Proofs = serde_wasm_bindgen::from_value(proofs_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse proofs: {:?}", e)))?;

    web_sys::console::log_1(&format!("Adding {} proofs to wallet as Unspent", proofs.len()).into());

    // Convert proofs to ProofInfo and store them
    for proof in proofs {
        let proof_info = cdk_common::common::ProofInfo::new(
            proof,
            wallet.mint_url.clone(),
            cdk_common::nuts::State::Unspent,
            wallet.unit.clone(),
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create proof info: {:?}", e)))?;

        wallet.localstore.update_proofs(vec![proof_info], vec![]).await
            .map_err(|e| JsValue::from_str(&format!("Failed to store proof: {:?}", e)))?;
    }

    web_sys::console::log_1(&format!("Successfully added proofs to wallet").into());
    Ok(())
}

/// Check proof states from the mint and update local storage
#[wasm_bindgen]
pub async fn check_proofs_state(
    mint_url: String,
    seed_words: String,
) -> Result<JsValue, JsValue> {
    use cdk::nuts::{CheckStateRequest, CheckStateResponse};
    use cdk::wallet::MintConnector;

    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let mnemonic = Mnemonic::parse(&seed_words)
        .map_err(|e| JsValue::from_str(&format!("Invalid seed: {:?}", e)))?;

    let seed = mnemonic.to_seed("");

    let storage_key = format!("wallet_{}", url.to_string().replace("://", "_").replace("/", "_"));
    let store = Arc::new(LocalStorageWalletDatabase::new(&storage_key).await?);

    let http_client = HttpClient::new(url.clone());

    let wallet = WalletBuilder::new()
        .mint_url(url.clone())
        .unit(CurrencyUnit::Sat)
        .localstore(store)
        .seed(seed)
        .client(http_client.clone())
        .build()
        .map_err(|e| JsValue::from_str(&format!("Failed to build wallet: {:?}", e)))?;

    // Get all proofs from local storage
    let all_proofs = wallet.localstore
        .get_proofs(
            Some(wallet.mint_url.clone()),
            Some(wallet.unit.clone()),
            None,
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to get proofs: {:?}", e)))?;

    if all_proofs.is_empty() {
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "checked": 0,
            "changes": {},
        }))?);
    }

    web_sys::console::log_1(&format!("Checking state of {} proofs with mint", all_proofs.len()).into());

    // Extract Y values to check with mint
    let ys: Vec<_> = all_proofs.iter().map(|p| p.y.clone()).collect();

    // Query mint for proof states using post_check_state
    let request = CheckStateRequest { ys: ys.clone() };
    let response: CheckStateResponse = http_client.post_check_state(request).await
        .map_err(|e| JsValue::from_str(&format!("Failed to check proof states: {:?}", e)))?;

    // Track changes by transition type
    use std::collections::HashMap;
    let mut changes: HashMap<String, u64> = HashMap::new();

    // Update local storage with actual states from mint
    for (proof_info, proof_state) in all_proofs.iter().zip(response.states.iter()) {
        let new_state = proof_state.state;

        // Only update if state changed
        if proof_info.state != new_state {
            let amount = u64::from(proof_info.proof.amount);
            let transition = format!("{:?} => {:?}", proof_info.state, new_state);

            *changes.entry(transition.clone()).or_insert(0) += amount;

            wallet.localstore.update_proofs_state(vec![proof_info.y.clone()], new_state).await
                .map_err(|e| JsValue::from_str(&format!("Failed to update proof state: {:?}", e)))?;

            web_sys::console::log_1(&format!("Updated proof from {:?} to {:?} ({} sat)",
                proof_info.state, new_state, amount).into());
        }
    }

    web_sys::console::log_1(&format!("Checked {} proofs, found {} state transitions",
        all_proofs.len(), changes.len()).into());

    Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
        "checked": all_proofs.len(),
        "changes": changes,
    }))?)
}

/// Attempt to swap (spend) proofs with specific signatures
#[wasm_bindgen]
pub async fn swap_with_signatures(
    mint_url: String,
    proofs_json: JsValue,
    secret_keys: Vec<String>,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    // Deserialize proofs
    let proofs: Proofs = serde_wasm_bindgen::from_value(proofs_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse proofs: {:?}", e)))?;

    // Parse secret keys
    let secrets: Result<Vec<SecretKey>, _> = secret_keys
        .iter()
        .map(|s| SecretKey::from_str(s))
        .collect();
    let secrets = secrets
        .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {:?}", e)))?;

    // Get active keyset
    let http_client = HttpClient::new(url.clone());
    let keysets = http_client.get_mint_keysets().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get keysets: {:?}", e)))?;
    let active_keyset_id = keysets.keysets.iter()
        .find(|k| k.active)
        .ok_or_else(|| JsValue::from_str("No active keyset found"))?
        .id;

    // Create blinded outputs (no spending conditions)
    let mut outputs = Vec::new();
    let mut secrets_and_factors = Vec::new();

    for proof in &proofs {
        let secret = Secret::generate();
        let (blinded_point, blinding_factor) = blind_message(&secret.to_bytes(), None)
            .map_err(|e| JsValue::from_str(&format!("Failed to blind message: {:?}", e)))?;

        outputs.push(BlindedMessage::new(
            proof.amount,
            active_keyset_id,
            blinded_point,
        ));
        secrets_and_factors.push((secret, blinding_factor));
    }

    // Create swap request
    let mut swap_request = SwapRequest::new(proofs, outputs);

    // Sign with each provided secret key
    for secret in secrets {
        swap_request.sign_sig_all(secret)
            .map_err(|e| JsValue::from_str(&format!("Failed to sign: {:?}", e)))?;
    }

    // Submit swap request to mint
    match http_client.post_swap(swap_request).await {
        Ok(swap_response) => {
            // Get mint keys to unblind the response
            let all_keys = http_client.get_mint_keys().await
                .map_err(|e| JsValue::from_str(&format!("Failed to get mint keys: {:?}", e)))?;
            let mint_keys = all_keys.iter()
                .find(|k| k.id == active_keyset_id)
                .ok_or_else(|| JsValue::from_str("Keyset not found"))?;

            // Unblind signatures to create output proofs
            let secrets: Vec<Secret> = secrets_and_factors.iter().map(|(s, _)| s.clone()).collect();
            let factors = secrets_and_factors.iter().map(|(_, f)| f.clone()).collect();

            let output_proofs = construct_proofs(
                swap_response.signatures,
                factors,
                secrets,
                &mint_keys.keys,
            )
            .map_err(|e| JsValue::from_str(&format!("Failed to construct proofs: {:?}", e)))?;

            Ok(serde_wasm_bindgen::to_value(&SwapResult {
                success: true,
                error: None,
                proofs: Some(output_proofs),
            })?)
        }
        Err(e) => {
            Ok(serde_wasm_bindgen::to_value(&SwapResult {
                success: false,
                error: Some(format!("{:?}", e)),
                proofs: None,
            })?)
        }
    }
}
