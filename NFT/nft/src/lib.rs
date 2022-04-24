/*!
Non-Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by u128 (2**128 - 1).
  - JSON calls should pass u128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{ LazyOption, Vector };
use near_sdk::json_types::U128;

use near_sdk::{
    env, near_bindgen, ext_contract, log, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue, Balance, PromiseResult
};


// near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    minted_ids: Vector<AccountId>,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

// define the methods we'll use on the other contract
#[ext_contract(ext_ft)]
pub trait FungibleToken {
    fn ft_transfer_from(&mut self, sender_id: AccountId, amount: u128, memo: Option<String>);
}

// define methods we'll use as callbacks on our contract
#[ext_contract(ext_self)]
pub trait MyContract {
    fn nft_mint_by_ft_callback(&self, to: AccountId) -> String;
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        Self::new(
            owner_id,
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Klyve NEAR non-fungible token".to_string(),
                symbol: "KT-NEAR".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                base_uri: None,
                reference: None,
                reference_hash: None,
            }
        )
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: NFTContractMetadata) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        Self {
            minted_ids: Vector::new(b"".to_vec()), 
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    #[payable]
    pub fn nft_mint_pay(&mut self) {
        let amount: Balance = near_sdk::env::attached_deposit();
        log!("attach money is {}, singer_account_id {}, predecessor_account_id {}", amount, env::signer_account_id(), env::predecessor_account_id());
        self.minted_ids.push(&env::predecessor_account_id());
        let mint_id = self.minted_ids.len().to_string();
        self.tokens.internal_mint_with_refund(mint_id.clone(), env::predecessor_account_id(), 
            Some(TokenMetadata {
                title: Some(format!("klyve hack nft {}", mint_id)), // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
                description: Some(format!("the klyve hack nft, minted by NEAR, id {}", mint_id)), // free-form description
                media: Some("https://near.org/wp-content/uploads/2020/09/cropped-favicon-270x270.png".to_string()), // URL to associated media, preferably to decentralized, content-addressed storage
                media_hash: None, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
                copies: None, // number of copies of this set of metadata in existence when token was minted.
                issued_at: None, // When token was issued or minted, Unix epoch in milliseconds
                expires_at: None, // When token expires, Unix epoch in milliseconds
                starts_at: None, // When token starts being valid, Unix epoch in milliseconds
                updated_at: None, // When token was last updated, Unix epoch in milliseconds
                extra: None, // anything extra the NFT wants to store on-chain. Can be stringified JSON.
                reference: None, // URL to an off-chain JSON file with more info.
                reference_hash: None 
            }),
            None
        );
        if amount < 1000000000000000000000000 {
            panic!("Near deposit amount is not enough, the mint price is 1N.");
        } else if amount > 1000000000000000000000000 {
            let back_amount: Balance = amount - 1000000000000000000000000;
            log!("attach money too much, transfer back {}", back_amount);
            Promise::new(near_sdk::env::signer_account_id()).transfer(back_amount);
        }
    }

    #[payable]
    pub fn nft_mint_by_ft(&mut self, ft_amount: U128) {
        // let gas = env::prepaid_gas();
        assert!(self.minted_ids.len() <= 10000, "Sold out");
        log!("transfered from account {}", env::predecessor_account_id());
        let ft_contract: AccountId = AccountId::new_unchecked("klyve-hack-ft.klyve-hack.testnet".to_string());
        let sender: AccountId = AccountId::new_unchecked("klyve-hack.testnet".to_string());
        log!("prepaid gas {:?}, used {:?}, diff {:?}", env::prepaid_gas(), env::used_gas() * 8u64, env::prepaid_gas() - env::used_gas());
        ext_ft::ft_transfer_from(
            // env::predecessor_account_id(),
            sender,
            ft_amount.into(),
            None,
            ft_contract, // contract account id
            1, // yocto NEAR to attach
            // 5_000_000_000_000
            env::used_gas() * 8u64
        )
        .then(ext_self::nft_mint_by_ft_callback(
            env::predecessor_account_id(), // nft claimer
            env::current_account_id(), // this contract's account id
            env::attached_deposit(), // yocto NEAR to attach to the callback
            env::used_gas() * 16u64// gas to attach
        ));
    }

    #[payable]
    pub fn nft_mint_by_ft_callback(&mut self, to: AccountId) {
        log!("callback {:?}", env::prepaid_gas());
        log!("to id at callback init {}", to);
        assert_eq!(
            env::promise_results_count(),
            1,
            "This is a callback method"
        );
        // handle the result from the cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => "oops! transfer from sender failed".to_string(),
            PromiseResult::Successful(result) => {
                // log!("{}", String::from_utf8(result.clone()).unwrap());
                self.minted_ids.push(&to);
                // self.token_amt_to_mint = self.token_amt_to_mint + 1;
                let mint_id = self.minted_ids.len().to_string();
                log!("{}", mint_id);
                self.tokens.internal_mint_with_refund(mint_id.clone(), to, 
                    Some(TokenMetadata {
                        title: Some(format!("klyve hack nft {}", mint_id)), // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
                        description: Some(format!("the klyve hack nft, minted Big Nana, id {}", mint_id)), // free-form description
                        media: Some("https://near.org/wp-content/uploads/2020/09/cropped-favicon-270x270.png".to_string()), // URL to associated media, preferably to decentralized, content-addressed storage
                        media_hash: None, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
                        copies: None, // number of copies of this set of metadata in existence when token was minted.
                        issued_at: None, // When token was issued or minted, Unix epoch in milliseconds
                        expires_at: None, // When token expires, Unix epoch in milliseconds
                        starts_at: None, // When token starts being valid, Unix epoch in milliseconds
                        updated_at: None, // When token was last updated, Unix epoch in milliseconds
                        extra: None, // anything extra the NFT wants to store on-chain. Can be stringified JSON.
                        reference: None, // URL to an off-chain JSON file with more info.
                        reference_hash: None 
                    }),
                    None,
                );
                "".to_string()
            },
        };
    }


    /// Mint a new token with ID=`token_id` belonging to `receiver_id`.
    ///
    /// Since this example implements metadata, it also requires per-token metadata to be provided
    /// in this call. `self.tokens.mint` will also require it to be Some, since
    /// `StorageKey::TokenMetadata` was provided at initialization.
    ///
    /// `self.tokens.mint` will enforce `predecessor_account_id` to equal the `owner_id` given in
    /// initialization call to `new`.
    
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        receiver_id: AccountId,
        token_metadata: TokenMetadata,
    ) -> Token {
        self.tokens.internal_mint(token_id, receiver_id, Some(token_metadata))
    }
}

near_contract_standards::impl_non_fungible_token_core!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_approval!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(Contract, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;

    const MINT_STORAGE_COST: u128 = 5870000000000000000000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn sample_token_metadata() -> TokenMetadata {
        TokenMetadata {
            title: Some("Olympus Mons".into()),
            description: Some("The tallest mountain in the charted solar system".into()),
            media: None,
            media_hash: None,
            copies: Some(1u64),
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        }
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.nft_token("1".to_string()), None);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_mint() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());

        let token_id = "0".to_string();
        let token = contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());
        assert_eq!(token.token_id, token_id);
        assert_eq!(token.owner_id, accounts(0).to_string());
        assert_eq!(token.metadata.unwrap(), sample_token_metadata());
        assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_transfer(accounts(1), token_id.clone(), None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        if let Some(token) = contract.nft_token(token_id.clone()) {
            assert_eq!(token.token_id, token_id);
            assert_eq!(token.owner_id, accounts(1).to_string());
            assert_eq!(token.metadata.unwrap(), sample_token_metadata());
            assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
        } else {
            panic!("token not correctly created, or not found by nft_token");
        }
    }

    #[test]
    fn test_approve() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        // alice approves bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(150000000000000000000)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_approve(token_id.clone(), accounts(1), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert!(contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
    }

    #[test]
    fn test_revoke() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        // alice approves bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(150000000000000000000)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_approve(token_id.clone(), accounts(1), None);

        // alice revokes bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_revoke(token_id.clone(), accounts(1));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), None));
    }

    #[test]
    fn test_revoke_all() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        // alice approves bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(150000000000000000000)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_approve(token_id.clone(), accounts(1), None);

        // alice revokes bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_revoke_all(token_id.clone());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
    }
}
