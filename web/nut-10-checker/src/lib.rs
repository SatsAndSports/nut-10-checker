use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use bip39::Mnemonic;

// CDK imports
use cdk::nuts::{SecretKey, CurrencyUnit};
use cdk::wallet::{HttpClient, MintConnector, WalletBuilder};
use cdk::Amount;
use cdk_common::mint_url::MintUrl;

mod wallet_db;
use wallet_db::LocalStorageWalletDatabase;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! NUT-10 Checker is ready.", name)
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

/// Get wallet balance
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
