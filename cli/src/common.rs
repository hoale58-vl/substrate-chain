use bip39::{Language, Mnemonic, Seed, MnemonicType};
use node_runtime::{AccountId};
use sc_service::config::{BasePath};
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use grandpa_primitives::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use std::{path::PathBuf, str::FromStr};
use sp_core::{ecdsa, Pair, Public, H160, H256};
use clap::Args;
use tiny_hderive::bip32::ExtendedPrivKey;
use sha3::{Digest, Keccak256};

/// Generate AccountId based on string command line argument.
fn parse_account_id(s: &str) -> AccountId {
	AccountId::from_str(s).expect("Passed string is not a hex encoding of a public key")
}

/// Generate (AccountId, AccountId) based on string command line argument.
fn parse_account_stash_id(s: &str) -> (AccountId, AccountId) {
	let mut v = s.split(",");
	(parse_account_id(v.next().unwrap()), parse_account_id(v.next().unwrap()))
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
	/// Pass the AccountIds of authorities, stash account forming the committe at the genesis
	///
	/// Expects a delimited collection of AccountIds
	#[clap(long, require_value_delimiter = true, parse(from_str = parse_account_stash_id))]
	authority_ids: Option<Vec<(AccountId, AccountId)>>,

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
	pub fn authority_ids(&self) -> Vec<(AccountId, AccountId)> {
		match &self.authority_ids {
			Some(authority_ids) => authority_ids.to_vec(),
			None => vec![]
		}
	}

	pub fn faucet_ids(&self) -> Vec<AccountId> {
		match &self.faucet_ids {
			Some(faucet_ids) => faucet_ids.to_vec(),
			None => vec![]
		}
	}

	pub fn sudo_account_id(&self) -> Option<AccountId> {
		self.sudo_account_id
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

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

// Get accountId BIP44 from ecdsa key pair
pub fn get_account_id_from_pair(pair: ecdsa::Pair) -> Option<AccountId> {
	let decompressed = libsecp256k1::PublicKey::parse_slice(
		&pair.public().0,
		Some(libsecp256k1::PublicKeyFormat::Compressed),
	)
	.ok()?
	.serialize();

	let mut m = [0u8; 64];
	m.copy_from_slice(&decompressed[1..65]);

	Some(H160::from(H256::from_slice(Keccak256::digest(&m).as_slice())).into())
}

/// Helper function generate list of ecdsa BIP44 Key Pairs from seed phrase
fn derive_bip44_pairs_from_mnemonic<TPublic: Public>(
	mnemonic: &str,
	num_accounts: u32,
) -> Vec<TPublic::Pair> {
	let seed = Mnemonic::from_phrase(mnemonic, Language::English)
		.map(|x| Seed::new(&x, ""))
		.expect("Wrong mnemonic provided");

	let mut childs = Vec::new();
	for i in 0..num_accounts {
		if let Some(child_pair) =
			ExtendedPrivKey::derive(seed.as_bytes(), format!("m/44'/60'/0'/0/{}", i).as_ref())
				.ok()
				.map(|account| TPublic::Pair::from_seed_slice(&account.secret()).ok())
				.flatten()
		{
			childs.push(child_pair);
		} else {
			log::error!("An error ocurred while deriving key {} from parent", i)
		}
	}
	childs
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed(seed: &str) -> AccountId
{
	let childs = derive_bip44_pairs_from_mnemonic::<ecdsa::Public>(seed, 1);
	get_account_id_from_pair(childs.into_iter().nth(0).unwrap()).unwrap()
}

/// Helper function to generate an account ID from seed
fn get_stash_account_id_from_seed(seed: &str) -> AccountId
{
	let childs = derive_bip44_pairs_from_mnemonic::<ecdsa::Public>(seed, 2);
	get_account_id_from_pair(childs.into_iter().nth(1).unwrap()).unwrap()
}

pub fn authority_keys(
	account_id: Option<AccountId>,
	stash_account_id: Option<AccountId>,
	seed_phrase: Option<String>
) -> AuthorityKeys {
	let seed_phrase: String = seed_phrase.unwrap_or(
		Mnemonic::new(MnemonicType::Words12, Language::English).into_phrase()
	);
	let seed_phrase = seed_phrase.as_str();
	let account_id = match account_id {
		Some(account) => account,
		None => {
			get_account_id_from_seed(seed_phrase)
		}
	};
	let stash_account_id = match stash_account_id {
		Some(account) => account,
		None => {
			get_stash_account_id_from_seed(seed_phrase)
		}
	};

	AuthorityKeys { 
		account_id: account_id, 
		stash_account_id: stash_account_id,
		babe_key: get_from_seed::<BabeId>(seed_phrase), 
		grandpa_key: get_from_seed::<GrandpaId>(seed_phrase), 
		im_online_key: get_from_seed::<ImOnlineId>(seed_phrase), 
		authority_discovery_key: get_from_seed::<AuthorityDiscoveryId>(seed_phrase)
	}
}