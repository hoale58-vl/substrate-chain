use bip39::{Language, Mnemonic, Seed, MnemonicType};
use libp2p::{
	identity::{ed25519 as libp2p_ed25519, PublicKey},
	PeerId,
};
use node_runtime::AccountId;
use sc_cli::KeystoreParams;
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use sha3::{Digest, Keccak256};
use sp_application_crypto::key_types;
use sp_consensus_babe::AuthorityId as BabeId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_keystore::SyncCryptoStore;
use std::{fs, path::PathBuf, str::FromStr};
use sp_core::{ecdsa, Pair, Public, H160, H256, ed25519};
use structopt::StructOpt;
use tiny_hderive::bip32::ExtendedPrivKey;

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

pub fn derive_bip44_pairs_from_mnemonic<TPublic: Public>(
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

pub fn generate_accounts(mnemonic: String, num_accounts: u32) -> Vec<AccountId> {
	let childs = derive_bip44_pairs_from_mnemonic::<ecdsa::Public>(&mnemonic, num_accounts);
	log::debug!("Account Generation");
	childs
		.iter()
		.map(|par| {
			let account = get_account_id_from_pair(par.clone());
			log::debug!(
				"private_key {} --------> Account {:x?}",
				sp_core::hexdisplay::HexDisplay::from(&par.clone().seed()),
				account
			);
			account
		})
		.flatten()
		.collect()
}

pub const DEVNET_ID: &str = "dev";

/// Generate AccountId based on string command line argument.
fn parse_account_id(s: &str) -> AccountId {
	AccountId::from_str(s).expect("Passed string is not a hex encoding of a public key")
}

pub fn well_knowns_accounts(n_accounts: Option<u32>) -> Vec<AccountId> {
	let parent_mnemonic =
		"bottom drive obey lake curtain smoke basket hold race lonely fit walk".to_string();
	generate_accounts(parent_mnemonic, n_accounts.unwrap_or(10))
}

#[derive(Debug, StructOpt, Clone)]
pub struct ChainParams {
	/// Pass the chain id.
	///
	/// It can be a predefined one (dev) or an arbitrary chain id passed to the genesis block
	/// `dev` chain id means that a set of known accounts will be used to form a comittee
	#[structopt(long, value_name = "CHAIN_SPEC", default_value = "42")]
	pub chain_id: String,

	#[structopt(long, default_value_if("chain-id", Some(DEVNET_ID), "true"), conflicts_with_all(&["account-ids", "sudo-account-id"]))]
	pub dev_chain: bool,

	/// Specify custom base path.
	#[structopt(long, short = "d", value_name = "PATH", parse(from_os_str))]
	pub base_path: PathBuf,

	#[structopt(long)]
	pub session_period: Option<u32>,

	#[structopt(long)]
	pub millisecs_per_block: Option<u64>,

	#[structopt(long, default_value = "Procyon")]
	pub chain_name: String,

	#[structopt(long, default_value = "SIP")]
	pub token_symbol: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct AccountParams {
	/// Pass the AccountIds of authorities forming the committe at the genesis
	///
	/// Expects a delimited collection of AccountIds
	#[structopt(long, require_delimiter = true, parse(from_str = parse_account_id), required_unless("dev-chain"))]
	account_ids: Option<Vec<AccountId>>,

	/// Pass the AccountId of the sudo account
	///
	/// If the chain-id is "dev" it will default to the first generated account (Alice)
	/// and use a default pre-defined id in any other case
	#[structopt(long, parse(from_str = parse_account_id), required_unless("dev-chain"))]
	sudo_account_id: Option<AccountId>,

	/// Pass the AccountIds of authorities forming the committe at the genesis
	///
	/// Expects a delimited collection of AccountIds
	#[structopt(long, requires_if("dev-chain", "true"))]
	n_dev_accounts: Option<u32>,
}

impl AccountParams {
	pub fn account_ids(&self) -> Vec<AccountId> {
		match &self.account_ids {
			Some(account_ids) => account_ids.to_vec(),
			None => well_knowns_accounts(self.n_dev_accounts)
		}
	}

	pub fn sudo_account_id(&self) -> AccountId {
		match &self.sudo_account_id {
			Some(sudo_account_id) => *sudo_account_id,
			None => well_knowns_accounts(self.n_dev_accounts).into_iter().nth(0).unwrap()
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

#[derive(Clone)]
pub struct SerializablePeerId {
	inner: PeerId,
}

impl SerializablePeerId {
	pub fn new(inner: PeerId) -> SerializablePeerId {
		SerializablePeerId { inner }
	}
}

impl Serialize for SerializablePeerId {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s: String = format!("{}", self.inner);
		serializer.serialize_str(&s)
	}
}

impl<'de> Deserialize<'de> for SerializablePeerId {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let inner = PeerId::from_str(&s)
			.map_err(|_| D::Error::custom(format!("Could not deserialize as PeerId: {}", s)))?;
		Ok(SerializablePeerId { inner })
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorityKeys {
	pub account_id: AccountId,
	pub babe_key: BabeId,
	pub im_online_key: ImOnlineId,
	pub authority_discovery_key: AuthorityDiscoveryId,
	pub peer_id: SerializablePeerId,
}

/// returns Babe key, if absent a new key is generated from the seed phrase
fn babe_key(keystore: &impl SyncCryptoStore, seed_phrase: &str) -> BabeId
{
	SyncCryptoStore::sr25519_public_keys(&*keystore, key_types::BABE)
		.pop()
		.unwrap_or_else(|| {
			// Fix error not create file if seed_phrase is not None
			let public = SyncCryptoStore::sr25519_generate_new(&*keystore, key_types::BABE, Some(seed_phrase))
				.expect("Could not create babe key");
			SyncCryptoStore::insert_unknown(&*keystore, key_types::BABE, seed_phrase, public.as_ref())
				.expect("Could not create babe key");
			public
		})
		.into()
}

/// returns im online key, if absent a new key is generated from the seed phrase
fn im_online_key(keystore: &impl SyncCryptoStore, seed_phrase: &str) -> ImOnlineId
{
	SyncCryptoStore::sr25519_public_keys(&*keystore, key_types::IM_ONLINE)
		.pop()
		.unwrap_or_else(|| {
			// Fix error not create file if seed_phrase is not None
			let public = SyncCryptoStore::sr25519_generate_new(&*keystore, key_types::IM_ONLINE, Some(seed_phrase))
				.expect("Could not create im online key");
			SyncCryptoStore::insert_unknown(&*keystore, key_types::IM_ONLINE, seed_phrase, public.as_ref())
				.expect("Could not create im online key");
			public
		})
		.into()
}

/// returns discovery key, if absent a new key is generated from the seed phrase
fn discovery_key(keystore: &impl SyncCryptoStore, seed_phrase: &str) -> AuthorityDiscoveryId
{
	SyncCryptoStore::sr25519_public_keys(&*keystore, key_types::AUTHORITY_DISCOVERY)
		.pop()
		.unwrap_or_else(|| {
			// Fix error not create file if seed_phrase is not None
			let public = SyncCryptoStore::sr25519_generate_new(&*keystore, key_types::AUTHORITY_DISCOVERY, Some(seed_phrase))
				.expect("Could not create discovery key");
			SyncCryptoStore::insert_unknown(&*keystore, key_types::AUTHORITY_DISCOVERY, seed_phrase, public.as_ref())
				.expect("Could not create discovery key");
			public
		})
		.into()
}

// Return secretkey - generate secret file
fn p2p_key(
	chain_params: &ChainParams,
	account_id: &AccountId, 
	seed_phrase: &str) -> SerializablePeerId 
{
	let (pair, seed) = ed25519::Pair::from_phrase(seed_phrase, None).expect("seed phrase isn invalid");

	let file = chain_params
		.base_path()
		.path()
		.join(account_id.to_string())
		.join("p2p_key");
	let secret_hex = hex::encode(seed.as_ref());
	fs::write(file, secret_hex).expect("Could not write p2p secret");

	let mut public: ed25519::Public = pair.into();
	let public = libp2p_ed25519::PublicKey::decode(public.as_mut()).expect("Cannot decode ed25519 public");
	SerializablePeerId::new(PublicKey::Ed25519(public).into_peer_id())
}

pub fn open_keystore(
	keystore_params: &KeystoreParams,
	chain_params: &ChainParams,
	account_id: &AccountId,
) -> impl SyncCryptoStore {
	let chain_id = chain_params.chain_id();
	let base_path: BasePath = chain_params.base_path().path().join(account_id.to_string()).into();

	let config_dir = base_path.config_dir(chain_id);
	match keystore_params
		.keystore_config(&config_dir)
		.expect("keystore configuration should be available")
	{
		(_, KeystoreConfig::Path { path, password }) =>
			LocalKeystore::open(path, password).expect("Keystore open should succeed"),
		_ => unreachable!("keystore_config always returns path and password; qed"),
	}
}

pub fn authority_keys(
	keystore: &impl SyncCryptoStore,
	chain_params: &ChainParams,
	account_id: &AccountId,
	seed_phrase: Option<String>,
) -> AuthorityKeys {
	let seed_phrase: String = seed_phrase.unwrap_or(
		Mnemonic::new(MnemonicType::Words12, Language::English).into_phrase()
	);
	let seed_phrase = seed_phrase.as_str();

	let peer_id 					= p2p_key(chain_params, &account_id, seed_phrase.clone());
	let babe_key 					= babe_key(keystore, seed_phrase.clone());
	let im_online_key 				= im_online_key(keystore, seed_phrase.clone());
	let authority_discovery_key 	= discovery_key(keystore, seed_phrase);

	let account_id = account_id.clone();
	AuthorityKeys { account_id, babe_key, im_online_key, authority_discovery_key, peer_id }
}
