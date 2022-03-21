// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
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

use crate::{
	service::{new_partial, self, frontier_database_dir}, 
	common::{
		authority_keys,
		ChainParams,
		AccountParams,
		open_keystore,
		get_account_id_from_seed,
		p2p_key,
	},
	cli::{Subcommand, Cli},
	chain_spec,
};
use sc_cli::{ChainSpec, RuntimeVersion, SubstrateCli, Result, KeystoreParams};
use node_executor::ExecutorDispatch;
use node_runtime::{Block, RuntimeApi};
use sc_service::PartialComponents;
use std::{io::Write};
use clap::Args;
use sc_service::ChainType;

#[derive(Debug, Args)]
#[allow(missing_docs)]
pub struct BootstrapChainCmd {
    /// Force raw genesis storage output.
    #[clap(long = "raw")]
    pub raw: bool,

    #[clap(flatten)]
	#[allow(missing_docs)]
    pub keystore_params: KeystoreParams,

    #[clap(flatten)]
	#[allow(missing_docs)]
    pub chain_params: ChainParams,

	#[clap(flatten)]
	#[allow(missing_docs)]
    pub account_params: AccountParams,
}

impl BootstrapChainCmd {
	#[allow(missing_docs)]
    pub fn run(&self) -> Result<()> {
        let genesis_authorities = self
            .account_params
            .authority_ids()
            .iter()
            .map(|authority_id| {
				let (account_id, stash_id) = authority_id;
				let keystore = open_keystore(&self.keystore_params, &self.chain_params, &account_id);
				p2p_key(&self.chain_params, &account_id);
                authority_keys(
					Some(*account_id), 
					Some(*stash_id), 
					None,
					&keystore
				)
            })
            .collect();

		let faucet_accounts = self
            .account_params
            .faucet_ids();

        let chain_spec = chain_spec::config(
			self.chain_params.token_symbol(),
			self.chain_params.chain_name(),
            genesis_authorities,
			self.chain_params.chain_id(),
			self.account_params.sudo_account_id(),
			ChainType::Live,
			faucet_accounts
        );

        let json = sc_service::chain_ops::build_spec(&chain_spec, self.raw)?;
        if std::io::stdout().write_all(json.as_bytes()).is_err() {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }
}

/// The `bootstrap-node` command is used to generate key pairs for a single authority
/// private keys are stored in a specified keystore, and the public keys are written to stdout.
#[derive(Debug, Args)]
pub struct BootstrapNodeCmd {
    /// Pass the AccountId of a new node
    ///
    /// Expects a string with an AccountId (hex encoding of an sr2559 public key)
    /// If this argument is not passed a random AccountId will be generated using account-seed argument as a seed
    #[clap(long)]
    account_id: String,

    /// Pass seed used to generate the account pivate key (sr2559) and the corresponding AccountId
    #[clap(long)]
    pub seed_phrase: String,

    #[clap(flatten)]
	#[allow(missing_docs)]
    pub keystore_params: KeystoreParams,

    #[clap(flatten)]
	#[allow(missing_docs)]
    pub chain_params: ChainParams,
}

impl BootstrapNodeCmd {
	#[allow(missing_docs)]
    pub fn run(&self) -> Result<()> {
		let account_id = get_account_id_from_seed(self.seed_phrase.clone().as_str());
		let keystore = open_keystore(&self.keystore_params, &self.chain_params, &account_id);
        let keys = authority_keys(
			None, 
			None, 
			Some(self.seed_phrase.clone()), 
			&keystore
		);
		let p2p_key = p2p_key(&self.chain_params, &account_id);
		
        let keys_json = serde_json::to_string_pretty(&keys)
            .expect("serialization of authority keys should have succeeded");
		let p2p_key_serialized = serde_json::to_string(&p2p_key).unwrap();
        println!("{}\npeer_id: {}", keys_json, p2p_key_serialized);
        Ok(())
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run.base)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full(config, &cli).map_err(sc_cli::Error::Service)
			})
		},
		Some(Subcommand::BootstrapChain(cmd)) => cmd.run(),
        Some(Subcommand::BootstrapNode(cmd)) => cmd.run(),
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run::<Block, RuntimeApi, ExecutorDispatch>(config))
		},
		Some(Subcommand::Benchmark(cmd)) =>
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;

				runner.sync_run(|config| cmd.run::<Block, ExecutorDispatch>(config))
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
					.into())
			},
		Some(Subcommand::BenchmarkStorage(cmd)) => {
			if !cfg!(feature = "runtime-benchmarks") {
				return Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
					.into())
			}

			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } = new_partial(&config, &cli)?;
				let db = backend.expose_db();
				let storage = backend.expose_storage();

				Ok((cmd.run(config, client, db, storage), task_manager))
			})
		},
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					new_partial(&config, &cli)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = new_partial(&config, &cli)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = new_partial(&config, &cli)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					new_partial(&config, &cli)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				// Remove Frontier offchain db
				let frontier_database_config = sc_service::DatabaseSource::RocksDb {
					path: frontier_database_dir(&config),
					cache_size: 0,
				};
				cmd.run(frontier_database_config)?;
				cmd.run(config.database)
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } = new_partial(&config, &cli)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				// we don't need any of the components of new_partial, just a runtime, or a task
				// manager to do `async_run`.
				let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
				let task_manager =
					sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

				Ok((cmd.run::<Block, ExecutorDispatch>(config), task_manager))
			})
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
	}
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Procyon".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/paritytech/substrate/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2022
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let spec = match id {
			"" =>
				return Err(
					"Please specify which chain you want to run, e.g. --dev or --chain=local"
						.into(),
				),
			"dev" => Box::new(chain_spec::dev_config()),
			path =>
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		};
		Ok(spec)
	}

	fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&node_runtime::VERSION
	}
}
