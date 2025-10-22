// Low-level helper functions for testing P2PK and SigAll functionality

use wasm_bindgen::prelude::*;
use std::str::FromStr;
use std::sync::Arc;

// CDK imports
use cdk::nuts::{
    SecretKey, PublicKey, SpendingConditions, Conditions, SigFlag,
    SwapRequest, Proofs, BlindedMessage, Nut10Secret, BlindSignature,
};
use cdk::wallet::{HttpClient, MintConnector};
use cdk_common::mint_url::MintUrl;
use cdk::dhke::{blind_message, construct_proofs};
use cdk::secret::Secret;
use cdk::Amount;

/// Create blinded outputs with no spending conditions
/// Returns: { outputs, secrets, blinding_factors }
#[wasm_bindgen]
pub async fn create_blinded_outputs(
    mint_url: String,
    amounts: Vec<u64>,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let http_client = HttpClient::new(url);
    let keysets = http_client.get_mint_keysets().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get keysets: {:?}", e)))?;
    let active_keyset_id = keysets.keysets.iter()
        .find(|k| k.active)
        .ok_or_else(|| JsValue::from_str("No active keyset found"))?
        .id;

    let mut outputs = Vec::new();
    let mut secrets = Vec::new();
    let mut blinding_factors = Vec::new();

    for amount in amounts {
        let secret = Secret::generate();
        let (blinded_point, blinding_factor) = blind_message(&secret.to_bytes(), None)
            .map_err(|e| JsValue::from_str(&format!("Failed to blind message: {:?}", e)))?;

        outputs.push(BlindedMessage::new(Amount::from(amount), active_keyset_id, blinded_point));
        secrets.push(secret);
        blinding_factors.push(blinding_factor);
    }

    #[derive(serde::Serialize)]
    struct OutputData {
        outputs: Vec<BlindedMessage>,
        secrets: Vec<Secret>,
        blinding_factors: Vec<SecretKey>,
    }

    Ok(serde_wasm_bindgen::to_value(&OutputData {
        outputs,
        secrets,
        blinding_factors,
    })?)
}

/// Create blinded outputs with P2PK spending conditions
/// Returns: { outputs, secrets, blinding_factors }
#[wasm_bindgen]
pub async fn create_blinded_outputs_with_p2pk(
    mint_url: String,
    amounts: Vec<u64>,
    spending_conditions_json: JsValue,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let spending_conditions: SpendingConditions = serde_wasm_bindgen::from_value(spending_conditions_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse spending conditions: {:?}", e)))?;

    let http_client = HttpClient::new(url);
    let keysets = http_client.get_mint_keysets().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get keysets: {:?}", e)))?;
    let active_keyset_id = keysets.keysets.iter()
        .find(|k| k.active)
        .ok_or_else(|| JsValue::from_str("No active keyset found"))?
        .id;

    let mut outputs = Vec::new();
    let mut secrets = Vec::new();
    let mut blinding_factors = Vec::new();

    for amount in amounts {
        // Each proof needs a unique secret even with same spending conditions
        let nut10_secret: Nut10Secret = spending_conditions.clone().into();
        let secret: Secret = nut10_secret.try_into()
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to secret: {:?}", e)))?;

        let (blinded_point, blinding_factor) = blind_message(&secret.to_bytes(), None)
            .map_err(|e| JsValue::from_str(&format!("Failed to blind message: {:?}", e)))?;

        outputs.push(BlindedMessage::new(Amount::from(amount), active_keyset_id, blinded_point));
        secrets.push(secret);
        blinding_factors.push(blinding_factor);
    }

    #[derive(serde::Serialize)]
    struct OutputData {
        outputs: Vec<BlindedMessage>,
        secrets: Vec<Secret>,
        blinding_factors: Vec<SecretKey>,
    }

    Ok(serde_wasm_bindgen::to_value(&OutputData {
        outputs,
        secrets,
        blinding_factors,
    })?)
}

/// Create spending conditions for P2PK 2-of-2 multisig with SigAll
#[wasm_bindgen]
pub fn create_spending_conditions_p2pk_2of2(
    pubkey1: String,
    pubkey2: String,
    locktime: Option<u64>,
) -> Result<JsValue, JsValue> {
    let pk1 = PublicKey::from_str(&pubkey1)
        .map_err(|e| JsValue::from_str(&format!("Invalid pubkey1: {:?}", e)))?;
    let pk2 = PublicKey::from_str(&pubkey2)
        .map_err(|e| JsValue::from_str(&format!("Invalid pubkey2: {:?}", e)))?;

    let conditions = Conditions::new(
        locktime,
        Some(vec![pk2]),  // Additional pubkey (second signer)
        Some(vec![pk1]),  // Refund keys (first signer can refund after locktime)
        Some(2),          // num_sigs: require 2 signatures before locktime
        Some(SigFlag::SigAll),  // SigAll flag
        Some(1),          // num_sigs_refund: 1 signature after locktime
    ).map_err(|e| JsValue::from_str(&format!("Failed to create conditions: {:?}", e)))?;

    let spending_conditions = SpendingConditions::new_p2pk(pk1, Some(conditions));

    Ok(serde_wasm_bindgen::to_value(&spending_conditions)?)
}

/// Create spending conditions with locktime and refund key
/// Before locktime: requires primary_pubkey signature
/// After locktime: requires refund_pubkey signature
#[wasm_bindgen]
pub fn create_spending_conditions_with_locktime_refund(
    primary_pubkey: String,
    refund_pubkey: String,
    locktime: u64,
) -> Result<JsValue, JsValue> {
    let primary_pk = PublicKey::from_str(&primary_pubkey)
        .map_err(|e| JsValue::from_str(&format!("Invalid primary pubkey: {:?}", e)))?;
    let refund_pk = PublicKey::from_str(&refund_pubkey)
        .map_err(|e| JsValue::from_str(&format!("Invalid refund pubkey: {:?}", e)))?;

    let conditions = Conditions::new(
        Some(locktime),           // Locktime
        None,                      // No additional pubkeys (only primary before locktime)
        Some(vec![refund_pk]),    // Refund keys (can spend after locktime)
        Some(1),                   // num_sigs: require 1 signature before locktime
        None,                      // No SigFlag (default SigInputs)
        Some(1),                   // num_sigs_refund: 1 signature after locktime
    ).map_err(|e| JsValue::from_str(&format!("Failed to create conditions: {:?}", e)))?;

    let spending_conditions = SpendingConditions::new_p2pk(primary_pk, Some(conditions));

    Ok(serde_wasm_bindgen::to_value(&spending_conditions)?)
}

/// Create a SwapRequest from input proofs and blinded outputs (does not sign)
#[wasm_bindgen]
pub fn create_swap_request(
    inputs_json: JsValue,
    outputs_json: JsValue,
) -> Result<JsValue, JsValue> {
    let inputs: Proofs = serde_wasm_bindgen::from_value(inputs_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse inputs: {:?}", e)))?;
    let outputs: Vec<BlindedMessage> = serde_wasm_bindgen::from_value(outputs_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse outputs: {:?}", e)))?;

    let swap_request = SwapRequest::new(inputs, outputs);

    Ok(serde_wasm_bindgen::to_value(&swap_request)?)
}

/// Sign a SwapRequest with SigAll flag
#[wasm_bindgen]
pub fn sign_swap_request_sigall(
    swap_request_json: JsValue,
    secret_keys: Vec<String>,
) -> Result<JsValue, JsValue> {
    let mut swap_request: SwapRequest = serde_wasm_bindgen::from_value(swap_request_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse swap request: {:?}", e)))?;

    let secrets: Result<Vec<SecretKey>, _> = secret_keys
        .iter()
        .map(|s| SecretKey::from_str(s))
        .collect();
    let secrets = secrets
        .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {:?}", e)))?;

    for secret in secrets {
        swap_request.sign_sig_all(secret)
            .map_err(|e| JsValue::from_str(&format!("Failed to sign: {:?}", e)))?;
    }

    Ok(serde_wasm_bindgen::to_value(&swap_request)?)
}

/// Sign a SwapRequest with individual P2PK signatures (not SigAll)
#[wasm_bindgen]
pub fn sign_swap_request_p2pk(
    swap_request_json: JsValue,
    secret_keys: Vec<String>,
) -> Result<JsValue, JsValue> {
    let mut swap_request: SwapRequest = serde_wasm_bindgen::from_value(swap_request_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse swap request: {:?}", e)))?;

    let secrets: Result<Vec<SecretKey>, _> = secret_keys
        .iter()
        .map(|s| SecretKey::from_str(s))
        .collect();
    let secrets = secrets
        .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {:?}", e)))?;

    // Sign each input proof individually
    for secret in &secrets {
        for proof in swap_request.inputs_mut() {
            proof.sign_p2pk(secret.clone())
                .map_err(|e| JsValue::from_str(&format!("Failed to sign: {:?}", e)))?;
        }
    }

    Ok(serde_wasm_bindgen::to_value(&swap_request)?)
}

/// Submit a SwapRequest to the mint
#[wasm_bindgen]
pub async fn submit_swap_request(
    mint_url: String,
    swap_request_json: JsValue,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let swap_request: SwapRequest = serde_wasm_bindgen::from_value(swap_request_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse swap request: {:?}", e)))?;

    let http_client = HttpClient::new(url);

    match http_client.post_swap(swap_request).await {
        Ok(response) => {
            #[derive(serde::Serialize)]
            struct SwapSuccess {
                success: bool,
                signatures: Vec<BlindSignature>,
            }

            Ok(serde_wasm_bindgen::to_value(&SwapSuccess {
                success: true,
                signatures: response.signatures,
            })?)
        }
        Err(e) => {
            #[derive(serde::Serialize)]
            struct SwapError {
                success: bool,
                error: String,
            }

            Ok(serde_wasm_bindgen::to_value(&SwapError {
                success: false,
                error: format!("{:?}", e),
            })?)
        }
    }
}

/// Unblind signatures to create proofs
#[wasm_bindgen]
pub async fn unblind_signatures(
    mint_url: String,
    signatures_json: JsValue,
    secrets_json: JsValue,
    blinding_factors_json: JsValue,
) -> Result<JsValue, JsValue> {
    let url: MintUrl = mint_url.parse()
        .map_err(|e| JsValue::from_str(&format!("Invalid mint URL: {:?}", e)))?;

    let signatures: Vec<BlindSignature> = serde_wasm_bindgen::from_value(signatures_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse signatures: {:?}", e)))?;
    let secrets: Vec<Secret> = serde_wasm_bindgen::from_value(secrets_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse secrets: {:?}", e)))?;
    let blinding_factors: Vec<SecretKey> = serde_wasm_bindgen::from_value(blinding_factors_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse blinding factors: {:?}", e)))?;

    // Get mint keys
    let http_client = HttpClient::new(url);
    let all_keys = http_client.get_mint_keys().await
        .map_err(|e| JsValue::from_str(&format!("Failed to get mint keys: {:?}", e)))?;

    // Find the keyset for the first signature
    let keyset_id = signatures[0].keyset_id;
    let mint_keys = all_keys.iter()
        .find(|k| k.id == keyset_id)
        .ok_or_else(|| JsValue::from_str("Keyset not found"))?;

    // Construct proofs
    let proofs = construct_proofs(signatures, blinding_factors, secrets, &mint_keys.keys)
        .map_err(|e| JsValue::from_str(&format!("Failed to construct proofs: {:?}", e)))?;

    Ok(serde_wasm_bindgen::to_value(&proofs)?)
}
