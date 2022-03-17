// This file is part of Substrate.

// Copyright (C) 2018-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use node_runtime::{
	constants::currency::*, wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig,
	BalancesConfig, Block, CouncilConfig, DemocracyConfig, ElectionsConfig, GrandpaConfig,
	ImOnlineConfig, IndicesConfig, MaxNominations, SessionConfig, SessionKeys, SocietyConfig,
	StakerStatus, StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
	EVMConfig, EthereumConfig,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_runtime::{
	Perbill,
};
use std::collections::BTreeMap;

pub use node_primitives::{AccountId, Balance, Signature};
pub use node_runtime::GenesisConfig;
use crate::common::{
	get_account_id_from_seed, 
	AuthorityKeys,
	authority_keys
};

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// Helper function to create GenesisConfig for testing
fn genesis(
	authorities: Vec<AuthorityKeys>,
	root_key: Option<AccountId>,
	faucet_accounts: Vec<AccountId>,
	initial_nominators: Vec<AccountId>,
	_enable_println: bool
) -> GenesisConfig {
	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	let num_faucet_accounts = faucet_accounts.len();

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = authorities
		.iter()
		.map(|auth| (auth.account_id.clone(), auth.stash_account_id.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|nominator| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MaxNominations::get() as usize).min(authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.account_id.clone())
				.collect::<Vec<_>>();
			(nominator.clone(), nominator.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();
		

	GenesisConfig {
		system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
		balances: BalancesConfig {
			balances: faucet_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
		},
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(node_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		grandpa: GrandpaConfig { authorities: vec![] },
		session: SessionConfig {
			keys: authorities
				.clone()
				.into_iter()
				.map(|auth| {
					(
						auth.account_id.clone(),
						auth.account_id.clone(),
						SessionKeys {
							babe: auth.babe_key.clone(),
							grandpa: auth.grandpa_key,
							authority_discovery: auth.authority_discovery_key,
							im_online: auth.im_online_key,
						},
					)
				})
				.collect(),
		},
		sudo: SudoConfig { key: root_key },
		evm: EVMConfig {
			accounts: BTreeMap::new(),
		},
		ethereum: EthereumConfig {},
		staking: StakingConfig {
			validator_count: authorities.len() as u32,
			minimum_validator_count: authorities.len() as u32,
			invulnerables: authorities.iter().map(|x| x.account_id.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections: ElectionsConfig {
			members: faucet_accounts
				.iter()
				.take((num_faucet_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		council: CouncilConfig::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: faucet_accounts
				.iter()
				.take((num_faucet_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		technical_membership: Default::default(),
		treasury: Default::default(),
		society: SocietyConfig {
			members: faucet_accounts
				.iter()
				.take((num_faucet_accounts + 1) / 2)
				.cloned()
				.collect(),
			pot: 0,
			max_members: 999,
		},
		vesting: Default::default(),
		assets: Default::default(),
		indices: IndicesConfig { indices: vec![] },
		gilt: Default::default(),
		transaction_storage: Default::default(),
		transaction_payment: Default::default(),
		base_fee: Default::default(),
	}
}

#[allow(missing_docs)]
pub fn dev_config() -> ChainSpec {
	let chain_type = ChainType::Development;
	let token_symbol = "SIP";
	let chain_name = "Procyon";
	let chain_id = "43";

	// Single node => Development (Require only one validator)
	let alith_account = get_account_id_from_seed("bottom drive obey lake curtain smoke basket hold race lonely fit walk");
	let alith_stash_account = get_account_id_from_seed("bottom drive obey mountain curtain smoke basket hold race lonely fit walk");
	let accounts : Vec<AccountId> = vec![
		alith_account.clone()
	];

	// Development mode - Alice will automatically create and finalize block
	let authority_keys : Vec<AuthorityKeys> = vec![
		authority_keys(Some(alith_account), Some(alith_stash_account), Some(String::from("Alice")))
	];

	config(
		&token_symbol,
		&chain_name,
		authority_keys,
		&chain_id,
		accounts.clone().into_iter().nth(0),
		chain_type,
		accounts
	)
}

#[allow(missing_docs)]
pub fn config(
	token_symbol:  &str,
	chain_name:  &str,
	authorities: Vec<AuthorityKeys>,
	chain_id: &str,
	sudo_account: Option<AccountId>,
	chain_type: ChainType,
	faucet_accounts: Vec<AccountId>
) -> ChainSpec {
	ChainSpec::from_genesis(
		&chain_name, // Name
		chain_id, // ID
		chain_type,
		move || {
			genesis(
				// Initial PoA authorities
				authorities.clone(),
				// Sudo account
				sudo_account.clone(),
				// Pre-funded accounts
				faucet_accounts.clone(),
				vec![],
				true
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Fork ID
		None,
		// Properties
		Some({
			let json_str =
				format!("{{\"tokenDecimals\": 18, \"tokenSymbol\": \"{}\", \"ss58Format\": 43}}", token_symbol)
					.to_string();
			serde_json::from_str(&json_str).expect("Provided valid json map")
		}),
		// Extensions
		Default::default(),
	)
}
