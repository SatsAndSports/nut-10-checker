// Test context that encapsulates mint-specific state for testing
use wasm_bindgen::prelude::*;
use std::sync::Arc;
use bip39::Mnemonic;

use cdk::nuts::{Id, CurrencyUnit, Proofs, BlindedMessage, SecretKey as CdkSecretKey, SwapRequest};
use cdk::wallet::{HttpClient, WalletBuilder, Wallet, MintConnector};
use cdk_common::mint_url::MintUrl;
use cdk::Amount;
use cdk::dhke::{blind_message, construct_proofs};
use cdk::secret::Secret;
use cdk::nuts::{KeySet, SpendingConditions, Nut10Secret};

use crate::wallet_db::LocalStorageWalletDatabase;

/// Test context that holds mint connection and cached state
#[wasm_bindgen]
pub struct TestContext {
    wallet: Wallet,
    http_client: HttpClient,
    active_keyset_id: Id,
    mint_keys: KeySet,
}

#[wasm_bindgen]
impl TestContext {
    /// Create a new test context for a mint
    #[wasm_bindgen(constructor)]
    pub async fn new(mint_url: String, seed_words: String) -> Result<TestContext, JsValue> {
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

        // Get active keyset
        let keysets = http_client.get_mint_keysets().await
            .map_err(|e| JsValue::from_str(&format!("Failed to get keysets: {:?}", e)))?;
        let active_keyset_id = keysets.keysets.iter()
            .find(|k| k.active)
            .ok_or_else(|| JsValue::from_str("No active keyset found"))?
            .id;

        // Get mint keys for the active keyset
        let all_keys = http_client.get_mint_keys().await
            .map_err(|e| JsValue::from_str(&format!("Failed to get mint keys: {:?}", e)))?;
        let mint_keys = all_keys.iter()
            .find(|k| k.id == active_keyset_id)
            .ok_or_else(|| JsValue::from_str("Active keyset keys not found"))?
            .clone();

        Ok(TestContext {
            wallet,
            http_client,
            active_keyset_id,
            mint_keys,
        })
    }

    /// Get unspent proofs from wallet that sum to at least the target amount
    /// Returns: { proofs, ys } where ys are needed for state management
    pub async fn get_proofs_to_spend(&self, amount: u64) -> Result<JsValue, JsValue> {
        // Get ONLY unspent proofs from wallet
        let all_proofs = self.wallet.localstore
            .get_proofs(
                Some(self.wallet.mint_url.clone()),
                Some(self.wallet.unit.clone()),
                Some(vec![cdk_common::nuts::State::Unspent]),
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get proofs: {:?}", e)))?;

        // Select proofs that sum to at least the target amount
        let mut input_proofs = Vec::new();
        let mut input_ys = Vec::new();
        let mut input_total: u64 = 0;

        for proof_info in all_proofs {
            if input_total >= amount {
                break;
            }
            input_total += u64::from(proof_info.proof.amount);
            input_ys.push(proof_info.y.clone());
            input_proofs.push(proof_info.proof);
        }

        if input_total < amount {
            return Err(JsValue::from_str(&format!("Insufficient balance: have {} sat, need {} sat", input_total, amount)));
        }

        web_sys::console::log_1(&format!("Selected {} proofs totaling {} sat for {} sat spend",
            input_proofs.len(), input_total, amount).into());

        #[derive(serde::Serialize)]
        struct ProofsResult {
            proofs: Proofs,
            ys: Vec<cdk::nuts::PublicKey>,
            total: u64,
        }

        Ok(serde_wasm_bindgen::to_value(&ProofsResult {
            proofs: input_proofs,
            ys: input_ys,
            total: input_total,
        })?)
    }

    /// Create blinded outputs with no spending conditions
    /// Returns: { outputs, secrets, blinding_factors }
    pub async fn create_blinded_outputs(&self, amounts: Vec<u64>) -> Result<JsValue, JsValue> {
        let mut outputs = Vec::new();
        let mut secrets = Vec::new();
        let mut blinding_factors = Vec::new();

        for amount in amounts {
            let secret = Secret::generate();
            let (blinded_point, blinding_factor) = blind_message(&secret.to_bytes(), None)
                .map_err(|e| JsValue::from_str(&format!("Failed to blind message: {:?}", e)))?;

            outputs.push(BlindedMessage::new(
                Amount::from(amount),
                self.active_keyset_id,
                blinded_point,
            ));
            secrets.push(secret);
            blinding_factors.push(blinding_factor);
        }

        #[derive(serde::Serialize)]
        struct OutputData {
            outputs: Vec<BlindedMessage>,
            secrets: Vec<Secret>,
            blinding_factors: Vec<CdkSecretKey>,
        }

        Ok(serde_wasm_bindgen::to_value(&OutputData {
            outputs,
            secrets,
            blinding_factors,
        })?)
    }

    /// Create blinded outputs with spending conditions
    /// Returns: { outputs, secrets, blinding_factors }
    pub async fn create_blinded_outputs_with_conditions(
        &self,
        amounts: Vec<u64>,
        spending_conditions_json: JsValue,
    ) -> Result<JsValue, JsValue> {
        let spending_conditions: SpendingConditions = serde_wasm_bindgen::from_value(spending_conditions_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse spending conditions: {:?}", e)))?;

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

            outputs.push(BlindedMessage::new(
                Amount::from(amount),
                self.active_keyset_id,
                blinded_point,
            ));
            secrets.push(secret);
            blinding_factors.push(blinding_factor);
        }

        #[derive(serde::Serialize)]
        struct OutputData {
            outputs: Vec<BlindedMessage>,
            secrets: Vec<Secret>,
            blinding_factors: Vec<CdkSecretKey>,
        }

        Ok(serde_wasm_bindgen::to_value(&OutputData {
            outputs,
            secrets,
            blinding_factors,
        })?)
    }

    /// Submit a swap request and update wallet state
    /// Returns: { success, signatures?, error? }
    pub async fn submit_swap(
        &self,
        swap_request_json: JsValue,
        input_ys_json: JsValue,
    ) -> Result<JsValue, JsValue> {
        use cdk::wallet::MintConnector;

        let swap_request: SwapRequest = serde_wasm_bindgen::from_value(swap_request_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse swap request: {:?}", e)))?;

        let input_ys: Vec<cdk::nuts::PublicKey> = serde_wasm_bindgen::from_value(input_ys_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse input ys: {:?}", e)))?;

        // Mark input proofs as Pending before attempting swap
        web_sys::console::log_1(&format!("Marking {} input proofs as Pending", input_ys.len()).into());
        self.wallet.localstore
            .update_proofs_state(input_ys.clone(), cdk_common::nuts::State::Pending)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to update state: {:?}", e)))?;

        // Execute swap via HTTP client
        match self.http_client.post_swap(swap_request).await {
            Ok(response) => {
                // Swap succeeded - delete spent proofs
                web_sys::console::log_1(&format!("Swap successful, deleting {} spent proofs", input_ys.len()).into());
                self.wallet.localstore.update_proofs(vec![], input_ys).await
                    .map_err(|e| JsValue::from_str(&format!("Failed to delete proofs: {:?}", e)))?;

                #[derive(serde::Serialize)]
                struct SwapSuccess {
                    success: bool,
                    signatures: Vec<cdk::nuts::BlindSignature>,
                }

                Ok(serde_wasm_bindgen::to_value(&SwapSuccess {
                    success: true,
                    signatures: response.signatures,
                })?)
            }
            Err(e) => {
                // Swap failed - leave as Pending
                web_sys::console::log_1(&format!("Swap failed ({:?}), leaving as Pending", e).into());

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
    pub async fn unblind_signatures(
        &self,
        signatures_json: JsValue,
        secrets_json: JsValue,
        blinding_factors_json: JsValue,
    ) -> Result<JsValue, JsValue> {
        let signatures: Vec<cdk::nuts::BlindSignature> = serde_wasm_bindgen::from_value(signatures_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse signatures: {:?}", e)))?;
        let secrets: Vec<Secret> = serde_wasm_bindgen::from_value(secrets_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse secrets: {:?}", e)))?;
        let blinding_factors: Vec<CdkSecretKey> = serde_wasm_bindgen::from_value(blinding_factors_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse blinding factors: {:?}", e)))?;

        // Construct proofs using cached mint keys
        let proofs = construct_proofs(signatures, blinding_factors, secrets, &self.mint_keys.keys)
            .map_err(|e| JsValue::from_str(&format!("Failed to construct proofs: {:?}", e)))?;

        Ok(serde_wasm_bindgen::to_value(&proofs)?)
    }

    /// Add proofs to wallet storage as Unspent
    pub async fn add_proofs_to_wallet(&self, proofs_json: JsValue) -> Result<(), JsValue> {
        let proofs: Proofs = serde_wasm_bindgen::from_value(proofs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse proofs: {:?}", e)))?;

        web_sys::console::log_1(&format!("Adding {} proofs to wallet as Unspent", proofs.len()).into());

        for proof in proofs {
            let proof_info = cdk_common::common::ProofInfo::new(
                proof,
                self.wallet.mint_url.clone(),
                cdk_common::nuts::State::Unspent,
                self.wallet.unit.clone(),
            )
            .map_err(|e| JsValue::from_str(&format!("Failed to create proof info: {:?}", e)))?;

            self.wallet.localstore.update_proofs(vec![proof_info], vec![]).await
                .map_err(|e| JsValue::from_str(&format!("Failed to store proof: {:?}", e)))?;
        }

        Ok(())
    }
}
