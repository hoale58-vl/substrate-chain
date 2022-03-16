use bip39::{Language, Mnemonic, MnemonicType};
use node_runtime::{AccountId, Signature};
use sc_service::config::{BasePath};
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use grandpa_primitives::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use std::{path::PathBuf, str::FromStr};
use sp_core::{sr25519, Pair, Public};
use clap::Args;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
};

/// Generate AccountId based on string command line argument.
fn parse_account_id(s: &str) -> AccountId {
	AccountId::from_str(s).expect("Passed string is not a hex encoding of a public key")
}

/// Helper function generate random accounts
pub fn random_accounts(n_accounts: u32) -> Vec<AccountId> {
	let mut accounts: Vec<AccountId> = Vec::with_capacity(n_accounts as usize);
	for _index in 0..(n_accounts-1) {
		let (pair, _) = sr25519::Pair::generate();
		accounts.push(
			AccountPublic::from(
				pair.public()
			).into_account()
		);
	}
	accounts
}

#[derive(Debug, Args, Clone)]
pub struct ChainParams {
	/// Pass the chain id.
	///
	/// It can be a predefined one (dev) or an arbitrary chain id passed to the genesis block
	/// `dev` chain id means that a set of known accounts will be used to form a comittee
	#[clap(long, value_name = "CHAIN_SPEC", default_value = "49")]
	pub chain_id: String,

	/// Specify custom base path.
	#[clap(long, short = 'd', value_name = "PATH", parse(from_os_str))]
	pub base_path: PathBuf,

	#[clap(long, default_value = "Procyon")]
	pub chain_name: String,

	#[clap(long, default_value = "SIP")]
	pub token_symbol: String,
}

#[derive(Debug, Args, Clone)]
pub struct AccountParams {
	/// Pass the AccountIds of authorities forming the committe at the genesis
	///
	/// Expects a delimited collection of AccountIds
	#[clap(long, require_value_delimiter = true, parse(from_str = parse_account_id))]
	authority_ids: Option<Vec<AccountId>>,

	/// Pass the faucets AccountIds
	///
	/// Expects a delimited collection of AccountIds
	#[clap(long, require_value_delimiter = true, parse(from_str = parse_account_id))]
	faucet_ids: Option<Vec<AccountId>>,

	/// Pass the AccountId of the sudo account
	///
	/// If the chain-id is "dev" it will default to the first generated account (Alice)
	/// and use a default pre-defined id in any other case
	#[clap(long, parse(from_str = parse_account_id))]
	sudo_account_id: Option<AccountId>,

	/// Pass the AccountIds of authorities forming the committe at the genesis
	///
	/// Expects a delimited collection of AccountIds
	#[clap(long, default_value = "10")]
	n_dev_accounts: u32,
}

impl AccountParams {
	pub fn authority_ids(&self) -> Vec<AccountId> {
		match &self.authority_ids {
			Some(authority_ids) => authority_ids.to_vec(),
			None => random_accounts(self.n_dev_accounts)
		}
	}

	pub fn faucet_ids(&self) -> Vec<AccountId> {
		match &self.faucet_ids {
			Some(faucet_ids) => faucet_ids.to_vec(),
			None => random_accounts(self.n_dev_accounts)
		}
	}

	pub fn sudo_account_id(&self) -> AccountId {
		match &self.sudo_account_id {
			Some(sudo_account_id) => sudo_account_id.clone(),
			None => random_accounts(self.n_dev_accounts).into_iter().nth(0).unwrap()
		}
	}
}

impl ChainParams {
	pub fn chain_id(&self) -> &str {
		&self.chain_id
	}

	pub fn base_path(&self) -> BasePath {
		self.base_path.clone().into()
	}

	pub fn chain_name(&self) -> &str {
		&self.chain_name
	}

	pub fn token_symbol(&self) -> &str {
		&self.token_symbol
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorityKeys {
	pub account_id: AccountId,
	pub stash_account_id: AccountId,
	pub babe_key: BabeId,
	pub grandpa_key: GrandpaId,
	pub im_online_key: ImOnlineId,
	pub authority_discovery_key: AuthorityDiscoveryId,
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn authority_keys(
	account_id: Option<&AccountId>,
	seed_phrase: Option<String>
) -> AuthorityKeys {
	let seed_phrase: String = seed_phrase.unwrap_or(
		Mnemonic::new(MnemonicType::Words12, Language::English).into_phrase()
	);
	let seed_phrase = seed_phrase.as_str();
	let account_from_seed = get_account_id_from_seed::<sr25519::Public>(seed_phrase);
	let account_id = account_id.unwrap_or(&account_from_seed);

	AuthorityKeys { 
		account_id: account_id.clone(), 
		stash_account_id: get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed_phrase)),
		babe_key: get_from_seed::<BabeId>(seed_phrase), 
		grandpa_key: get_from_seed::<GrandpaId>(seed_phrase), 
		im_online_key: get_from_seed::<ImOnlineId>(seed_phrase), 
		authority_discovery_key: get_from_seed::<AuthorityDiscoveryId>(seed_phrase)
	}
}