use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::window;

use cdk_common::database::Error as DbError;
use cdk_common::database::WalletDatabase;
use cdk_common::common::ProofInfo;
use cdk_common::mint_url::MintUrl;
use cdk_common::nuts::{
    CurrencyUnit, Id, KeySetInfo, Keys, MintInfo, PublicKey, SpendingConditions, State,
};
use cdk_common::wallet::{
    MintQuote, MeltQuote, Transaction, TransactionDirection, TransactionId,
};
use cashu::KeySet;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WalletState {
    mints: HashMap<MintUrl, Option<MintInfo>>,
    keysets: HashMap<MintUrl, Vec<KeySetInfo>>,
    keyset_map: HashMap<Id, KeySetInfo>,
    mint_quotes: HashMap<String, MintQuote>,
    melt_quotes: HashMap<String, MeltQuote>,
    keys: HashMap<Id, Keys>,
    proofs: Vec<ProofInfo>,
    keyset_counters: HashMap<Id, u32>,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Clone)]
pub struct LocalStorageWalletDatabase {
    state: Arc<Mutex<WalletState>>,
    storage_key: String,
}

impl LocalStorageWalletDatabase {
    pub async fn new(storage_key: &str) -> Result<Self, JsValue> {
        // Try to load from localStorage
        let state = match Self::load_from_localstorage(storage_key).await {
            Ok(state) => state,
            Err(_) => WalletState::default(),
        };

        let db = Self {
            state: Arc::new(Mutex::new(state)),
            storage_key: storage_key.to_string(),
        };

        // Save immediately
        db.save_snapshot().await?;

        Ok(db)
    }

    async fn save_snapshot(&self) -> Result<(), JsValue> {
        let state = self.state.lock().unwrap().clone();
        let json = serde_json::to_string(&state)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        let storage = window()
            .ok_or_else(|| JsValue::from_str("No window"))?
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("No localStorage"))?;

        storage.set_item(&self.storage_key, &json)?;
        Ok(())
    }

    async fn load_from_localstorage(storage_key: &str) -> Result<WalletState, JsValue> {
        let storage = window()
            .ok_or_else(|| JsValue::from_str("No window"))?
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("No localStorage"))?;

        let json = storage
            .get_item(storage_key)?
            .ok_or_else(|| JsValue::from_str("No wallet state found"))?;

        let state: WalletState = serde_json::from_str(&json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;

        Ok(state)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct StorageError(String);

fn to_db_error(e: JsValue) -> DbError {
    DbError::Database(Box::new(StorageError(format!("{:?}", e))))
}

#[async_trait(?Send)]
impl WalletDatabase for LocalStorageWalletDatabase {
    type Err = DbError;

    async fn add_mint(
        &self,
        mint_url: MintUrl,
        mint_info: Option<MintInfo>,
    ) -> Result<(), Self::Err> {
        self.state.lock().unwrap().mints.insert(mint_url, mint_info);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn remove_mint(&self, mint_url: MintUrl) -> Result<(), Self::Err> {
        self.state.lock().unwrap().mints.remove(&mint_url);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_mint(&self, mint_url: MintUrl) -> Result<Option<MintInfo>, Self::Err> {
        Ok(self.state.lock().unwrap().mints.get(&mint_url).cloned().flatten())
    }

    async fn get_mints(&self) -> Result<HashMap<MintUrl, Option<MintInfo>>, Self::Err> {
        Ok(self.state.lock().unwrap().mints.clone())
    }

    async fn update_mint_url(
        &self,
        old_mint_url: MintUrl,
        new_mint_url: MintUrl,
    ) -> Result<(), Self::Err> {
        {
            let mut state = self.state.lock().unwrap();
            if let Some(info) = state.mints.remove(&old_mint_url) {
                state.mints.insert(new_mint_url, info);
            }
        }
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn add_mint_keysets(
        &self,
        mint_url: MintUrl,
        keysets: Vec<KeySetInfo>,
    ) -> Result<(), Self::Err> {
        {
            let mut state = self.state.lock().unwrap();
            state.keysets.insert(mint_url, keysets.clone());
            for keyset in keysets {
                state.keyset_map.insert(keyset.id, keyset);
            }
        }
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_mint_keysets(
        &self,
        mint_url: MintUrl,
    ) -> Result<Option<Vec<KeySetInfo>>, Self::Err> {
        Ok(self.state.lock().unwrap().keysets.get(&mint_url).cloned())
    }

    async fn get_keyset_by_id(&self, keyset_id: &Id) -> Result<Option<KeySetInfo>, Self::Err> {
        Ok(self.state.lock().unwrap().keyset_map.get(keyset_id).cloned())
    }

    async fn add_mint_quote(&self, quote: MintQuote) -> Result<(), Self::Err> {
        self.state.lock().unwrap().mint_quotes.insert(quote.id.clone(), quote);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_mint_quote(&self, quote_id: &str) -> Result<Option<MintQuote>, Self::Err> {
        Ok(self.state.lock().unwrap().mint_quotes.get(quote_id).cloned())
    }

    async fn get_mint_quotes(&self) -> Result<Vec<MintQuote>, Self::Err> {
        Ok(self.state.lock().unwrap().mint_quotes.values().cloned().collect())
    }

    async fn remove_mint_quote(&self, quote_id: &str) -> Result<(), Self::Err> {
        self.state.lock().unwrap().mint_quotes.remove(quote_id);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn add_melt_quote(&self, quote: MeltQuote) -> Result<(), Self::Err> {
        self.state.lock().unwrap().melt_quotes.insert(quote.id.clone(), quote);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_melt_quote(&self, quote_id: &str) -> Result<Option<MeltQuote>, Self::Err> {
        Ok(self.state.lock().unwrap().melt_quotes.get(quote_id).cloned())
    }

    async fn get_melt_quotes(&self) -> Result<Vec<MeltQuote>, Self::Err> {
        Ok(self.state.lock().unwrap().melt_quotes.values().cloned().collect())
    }

    async fn remove_melt_quote(&self, quote_id: &str) -> Result<(), Self::Err> {
        self.state.lock().unwrap().melt_quotes.remove(quote_id);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn add_keys(&self, keyset: KeySet) -> Result<(), Self::Err> {
        let id = keyset.id;
        let keys = keyset.keys;
        self.state.lock().unwrap().keys.insert(id, keys);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_keys(&self, id: &Id) -> Result<Option<Keys>, Self::Err> {
        Ok(self.state.lock().unwrap().keys.get(id).cloned())
    }

    async fn remove_keys(&self, id: &Id) -> Result<(), Self::Err> {
        self.state.lock().unwrap().keys.remove(id);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn update_proofs(
        &self,
        added: Vec<ProofInfo>,
        removed_ys: Vec<PublicKey>,
    ) -> Result<(), Self::Err> {
        {
            let mut state = self.state.lock().unwrap();
            state.proofs.retain(|p| !removed_ys.contains(&p.y));
            state.proofs.extend(added);
        }
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_proofs(
        &self,
        mint_url: Option<MintUrl>,
        unit: Option<CurrencyUnit>,
        state: Option<Vec<State>>,
        spending_conditions: Option<Vec<SpendingConditions>>,
    ) -> Result<Vec<ProofInfo>, Self::Err> {
        let proofs = self.state.lock().unwrap().proofs.clone();

        let filtered: Vec<ProofInfo> = proofs
            .into_iter()
            .filter(|p| {
                mint_url.as_ref().map_or(true, |url| &p.mint_url == url)
                    && unit.as_ref().map_or(true, |u| &p.unit == u)
                    && state.as_ref().map_or(true, |states| states.contains(&p.state))
                    && spending_conditions.as_ref().map_or(true, |conds| {
                        p.spending_condition.as_ref().map_or(false, |pc| conds.contains(pc))
                            || p.spending_condition.is_none()
                    })
            })
            .collect();

        Ok(filtered)
    }

    async fn update_proofs_state(&self, ys: Vec<PublicKey>, new_state: State) -> Result<(), Self::Err> {
        {
            let mut state = self.state.lock().unwrap();
            for proof in &mut state.proofs {
                if ys.contains(&proof.y) {
                    proof.state = new_state;
                }
            }
        }
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn increment_keyset_counter(&self, keyset_id: &Id, count: u32) -> Result<u32, Self::Err> {
        let new_value = {
            let mut state = self.state.lock().unwrap();
            let counter = state.keyset_counters.entry(*keyset_id).or_insert(0);
            *counter += count;
            *counter
        };
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(new_value)
    }

    async fn add_transaction(&self, transaction: Transaction) -> Result<(), Self::Err> {
        self.state.lock().unwrap().transactions.push(transaction);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<Option<Transaction>, Self::Err> {
        Ok(self.state.lock().unwrap()
            .transactions
            .iter()
            .find(|t| TransactionId::new(t.ys.clone()) == transaction_id)
            .cloned())
    }

    async fn list_transactions(
        &self,
        mint_url: Option<MintUrl>,
        direction: Option<TransactionDirection>,
        unit: Option<CurrencyUnit>,
    ) -> Result<Vec<Transaction>, Self::Err> {
        let transactions = self.state.lock().unwrap().transactions.clone();

        let filtered: Vec<Transaction> = transactions
            .into_iter()
            .filter(|t| {
                mint_url.as_ref().map_or(true, |url| &t.mint_url == url)
                    && direction.as_ref().map_or(true, |dir| &t.direction == dir)
                    && unit.as_ref().map_or(true, |u| &t.unit == u)
            })
            .collect();

        Ok(filtered)
    }

    async fn remove_transaction(&self, transaction_id: TransactionId) -> Result<(), Self::Err> {
        self.state.lock().unwrap().transactions.retain(|t| TransactionId::new(t.ys.clone()) != transaction_id);
        self.save_snapshot().await.map_err(to_db_error)?;
        Ok(())
    }

    async fn get_balance(
        &self,
        mint_url: Option<MintUrl>,
        unit: Option<CurrencyUnit>,
        states: Option<Vec<State>>,
    ) -> Result<u64, Self::Err> {
        let proofs = self.get_proofs(mint_url, unit, states, None).await?;
        let balance: u64 = proofs.iter().map(|p| u64::from(p.proof.amount)).sum();
        Ok(balance)
    }
}
