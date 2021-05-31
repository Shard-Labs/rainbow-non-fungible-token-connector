use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{Base64VecU8, ValidAccountId, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, CryptoHash, PanicOnDefault, Promise, StorageUsage, Gas
};

use admin_controlled::{AdminControlled, Mask};

use crate::internal::*;
pub use crate::metadata::*;
pub use crate::mint::*;
pub use crate::nft_core::*;
pub use crate::token::*;

mod internal;
mod metadata;
mod mint;
mod nft_core;
mod token;

near_sdk::setup_alloc!();

/// Gas to call finish withdraw method on factory.
const FINISH_WITHDRAW_GAS: Gas = 50_000_000_000_000; // todo check if this value is valid

const NO_DEPOSIT: Balance = 0;

const PAUSE_WITHDRAW: Mask = 1 << 0;

#[ext_contract(ext_bridge_nft_factory)]
pub trait ExtBridgeNFTFactory {
    #[result_serializer(borsh)]
    fn finish_withdraw_to_eth(
        &self,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] recipient: AccountId,
    ) -> Promise;
}


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,

    pub tokens_by_id: LookupMap<TokenId, Token>,

    pub token_metadata_by_id: UnorderedMap<TokenId, TokenMetadata>,

    pub owner_id: AccountId,

    /// The storage size in bytes for one account.
    pub extra_storage_in_bytes_per_token: StorageUsage,

    pub metadata: LazyOption<NFTMetadata>,

    paused: Mask,
}

/// Helper structure to for keys of the persistent collections.
#[derive(BorshSerialize)]
pub enum StorageKey {
    TokensPerOwner,
    TokenPerOwnerInner { account_id_hash: CryptoHash },
    TokensById,
    TokenMetadataById,
    NftMetadata,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId, metadata: NFTMetadata) -> Self {
        let mut this = Self {
            tokens_per_owner: LookupMap::new(StorageKey::TokensPerOwner.try_to_vec().unwrap()),
            tokens_by_id: LookupMap::new(StorageKey::TokensById.try_to_vec().unwrap()),
            token_metadata_by_id: UnorderedMap::new(
                StorageKey::TokenMetadataById.try_to_vec().unwrap(),
            ),
            owner_id: owner_id.into(),
            extra_storage_in_bytes_per_token: 0,
            metadata: LazyOption::new(
                StorageKey::NftMetadata.try_to_vec().unwrap(),
                Some(&metadata),
            ),
            paused: Mask::default(),
        };

        this.measure_min_token_storage_cost();

        this
    }

    fn measure_min_token_storage_cost(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let tmp_account_id = "a".repeat(64);
        let u = UnorderedSet::new(
            StorageKey::TokenPerOwnerInner {
                account_id_hash: hash_account_id(&tmp_account_id),
            }
                .try_to_vec()
                .unwrap(),
        );
        self.tokens_per_owner.insert(&tmp_account_id, &u);

        let tokens_per_owner_entry_in_bytes = env::storage_usage() - initial_storage_usage;
        let owner_id_extra_cost_in_bytes = (tmp_account_id.len() - self.owner_id.len()) as u64;

        self.extra_storage_in_bytes_per_token =
            tokens_per_owner_entry_in_bytes + owner_id_extra_cost_in_bytes;

        self.tokens_per_owner.remove(&tmp_account_id);
    }

    #[payable]
    pub fn withdraw(&mut self, token_id: String, recipient: String) -> Promise {
        self.check_not_paused(PAUSE_WITHDRAW);

        assert_one_yocto();
        Promise::new(env::predecessor_account_id()).transfer(1);

        // check the token exists and that the caller is the owner
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");

        let predecessor_account_id = env::predecessor_account_id();
        if &predecessor_account_id != &token.owner_id && !token.approved_account_ids.contains_key(&predecessor_account_id) {
            env::panic(b"Unauthorized");
        }

        // burn the token
        self.tokens_by_id.remove(&token_id);
        self.token_metadata_by_id.remove(&token_id);

        let mut tokens_set = self.tokens_per_owner.get(&predecessor_account_id).unwrap();
        tokens_set.remove(&token_id);
        self.tokens_per_owner.insert(&predecessor_account_id, &tokens_set);

        // call the nft factory to finish the withdrawal to eth
        ext_bridge_nft_factory::finish_withdraw_to_eth(
            token_id,
            recipient,
            &self.owner_id,
            NO_DEPOSIT,
            FINISH_WITHDRAW_GAS,
        )
    }
}

admin_controlled::impl_admin_controlled!(Contract, paused);

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::serde::export::TryFrom;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn alice() -> AccountId {
        String::from("alice.near")
    }
    fn bob() -> AccountId {
        String::from("bob.near")
    }
    fn nft() -> AccountId {
        String::from("nft.near")
    }

    fn get_context(predecessor_account_id: AccountId, attached_deposit: Balance) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 1000 * 10u128.pow(24),
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit,
            prepaid_gas: 2 * 10u64.pow(14),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }

    fn helper_contract_metadata() -> NFTMetadata {
        NFTMetadata {
            spec: "".to_string(),
            name: "".to_string(),
            symbol: "".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        }
    }

    fn helper_token_metadata() -> TokenMetadata {
        TokenMetadata {
            title: Some("Mochi Rising".to_string()),
            description: Some("Limited edition canvas".to_string()),
            media: None,
            media_hash: None,
            copies: None,
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        }
    }

    fn helper_mint() -> (Contract, VMContext) {
        let context = get_context(nft(), 10u128.pow(24));
        testing_env!(context.clone());
        let mut contract = Contract::new(
            ValidAccountId::try_from(nft()).unwrap(),
            helper_contract_metadata(),
        );
        contract.nft_mint("0".to_string(), helper_token_metadata());
        (contract, context)
    }

    #[test]
    fn basic_mint_from_owner() {
        helper_mint();
    }

    #[test]
    #[should_panic(expected = "Owner's method")]
    fn failed_mint_from_non_owner() {
        let context = get_context(alice(), 7660000000000000000000);
        testing_env!(context);
        let mut contract = Contract::new(
            ValidAccountId::try_from(nft()).unwrap(),
            helper_contract_metadata(),
        );
        contract.nft_mint("0".to_string(), helper_token_metadata());
    }

    #[test]
    fn simple_transfer() {
        let (mut contract, mut context) = helper_mint();
        let token_info = contract.nft_token("0".to_string());
        assert!(
            token_info.is_some(),
            "Expected to find newly minted token, got None."
        );
        let token_info_obj = token_info.unwrap();
        // Add one yoctoⓃ
        context.attached_deposit = 1;
        testing_env!(context.clone());
        contract.nft_transfer(
            ValidAccountId::try_from(bob()).unwrap(),
            "0".to_string(),
            None,
            Some("my memo".to_string()),
        );
        assert_eq!(
            token_info_obj.approved_account_ids.len(),
            0,
            "Expected number of initial approvers to be 0"
        );
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn failed_simple_transfer_needs_one_yocto() {
        let (mut contract, _) = helper_mint();
        contract.nft_transfer(
            ValidAccountId::try_from(bob()).unwrap(),
            "0".to_string(),
            Some(U64::from(0u64)),
            Some("my memo".to_string()),
        );
    }

    #[test]
    fn transfer_using_approver() {
        let (mut contract, mut context) = helper_mint();
        let mut token_info = contract.nft_token("0".to_string());
        assert!(
            token_info.is_some(),
            "Expected to find newly minted token, got None."
        );
        let mut token_info_obj = token_info.unwrap();
        assert_eq!(
            token_info_obj.approved_account_ids.len(),
            0,
            "Expected no initial approvers."
        );
        contract.nft_approve(
            "0".to_string(),
            ValidAccountId::try_from(alice()).unwrap(),
            None,
        );
        token_info = contract.nft_token("0".to_string());
        assert!(
            token_info.is_some(),
            "Expected to find token after approval, got None."
        );
        token_info_obj = token_info.unwrap();
        assert_eq!(
            token_info_obj.approved_account_ids.len(),
            1,
            "Expected one approver."
        );
        assert_eq!(
            token_info_obj.owner_id,
            nft(),
            "Expected nft.near to own token."
        );
        // Call from alice
        context.predecessor_account_id = alice();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        contract.nft_transfer(
            ValidAccountId::try_from(alice()).unwrap(),
            "0".to_string(),
            Some(U64::from(0u64)),
            Some("thanks for allowing me to take it".to_string()),
        );
        token_info = contract.nft_token("0".to_string());
        assert!(
            token_info.is_some(),
            "Expected to find token after transfer, got None."
        );
        token_info_obj = token_info.unwrap();
        assert_eq!(
            token_info_obj.approved_account_ids.len(),
            0,
            "Expected approvers to reset to zero after transfer."
        );
        assert_eq!(
            token_info_obj.owner_id,
            alice(),
            "Expected alice.near to own token after transferring using approvals."
        );
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn failed_transfer_using_unauthorized_approver() {
        let (mut contract, mut context) = helper_mint();
        contract.nft_approve(
            "0".to_string(),
            ValidAccountId::try_from(alice()).unwrap(),
            None,
        );
        // Bob tries to transfer when only alice should be allowed to
        context.predecessor_account_id = bob();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        contract.nft_transfer(
            ValidAccountId::try_from(bob()).unwrap(),
            "0".to_string(),
            Some(U64::from(1u64)),
            Some("I am trying to hack you.".to_string()),
        );
    }
}
