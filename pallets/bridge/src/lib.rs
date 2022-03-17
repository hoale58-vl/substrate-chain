// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
		ensure,
		pallet_prelude::*,
		traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get},
	};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use pallet_chainbridge::{ChainId, Pallet as ChainBridgePallet, ResourceId};
	use pallet_erc721::Pallet as Erc721Pallet;
	use sp_arithmetic::traits::SaturatedConversion;
	use sp_core::U256;
	use sp_std::prelude::*;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_chainbridge::Config + pallet_erc721::Config
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Specifies the origin check provided by the bridge for calls that can only be called by
		/// the bridge pallet
		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// The currency mechanism.
		type Currency: Currency<Self::AccountId>;

		/// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
		type HashId: Get<ResourceId>;
		type NativeTokenId: Get<ResourceId>;
		type Erc721Id: Get<ResourceId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Remark(T::Hash),
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		InvalidTransfer,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		//
		// Initiation calls. These start a bridge transfer.
		//

		/// Transfers an arbitrary hash to a (whitelisted) destination chain.
		#[pallet::weight(195_000_000)]
		pub fn transfer_hash(
			origin: OriginFor<T>,
			hash: T::Hash,
			dest_id: ChainId,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let resource_id = T::HashId::get();
			let metadata: Vec<u8> = hash.as_ref().to_vec();
			ChainBridgePallet::<T>::transfer_generic(dest_id, resource_id, metadata)
		}

		/// Transfers some amount of the native token to some recipient on a (whitelisted)
		/// destination chain.
		#[pallet::weight(195_000_000)]
		pub fn transfer_native(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			recipient: Vec<u8>,
			dest_id: ChainId,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(
				ChainBridgePallet::<T>::chain_whitelisted(dest_id),
				Error::<T>::InvalidTransfer
			);
			let bridge_id = ChainBridgePallet::<T>::account_id();
			T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

			let resource_id = T::NativeTokenId::get();
			ChainBridgePallet::<T>::transfer_fungible(
				dest_id,
				resource_id,
				recipient,
				U256::from(amount.saturated_into::<u128>()),
			)
		}

		/// Transfer a non-fungible token (erc721) to a (whitelisted) destination chain.
		#[pallet::weight(195_000_000)]
		pub fn transfer_erc721(
			origin: OriginFor<T>,
			recipient: Vec<u8>,
			token_id: U256,
			dest_id: ChainId,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(
				ChainBridgePallet::<T>::chain_whitelisted(dest_id),
				Error::<T>::InvalidTransfer
			);
			match Erc721Pallet::<T>::tokens(&token_id) {
				Some(token) => {
					Erc721Pallet::<T>::burn_token(source, token_id)?;
					let resource_id = T::Erc721Id::get();
					let tid: &mut [u8] = &mut [0; 32];
					token_id.to_big_endian(tid);
					ChainBridgePallet::<T>::transfer_nonfungible(
						dest_id,
						resource_id,
						tid.to_vec(),
						recipient,
						token.metadata,
					)
				},
				None => Err(Error::<T>::InvalidTransfer)?,
			}
		}

		//
		// Executable calls. These can be triggered by a bridge transfer initiated on another chain
		//

		/// Executes a simple currency transfer using the bridge account as the source
		#[pallet::weight(195_000_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>
		) -> DispatchResult {
			let source = T::BridgeOrigin::ensure_origin(origin)?;
			<T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
			Ok(())
		}

		/// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
		#[pallet::weight(195_000_000)]
		pub fn remark(origin: OriginFor<T>, hash: T::Hash) -> DispatchResult {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::deposit_event(Event::Remark(hash));
			Ok(())
		}

		/// Allows the bridge to issue new erc721 tokens
		#[pallet::weight(195_000_000)]
		pub fn mint_erc721(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			id: U256,
			metadata: Vec<u8>,
		) -> DispatchResult {
			T::BridgeOrigin::ensure_origin(origin)?;
			Erc721Pallet::<T>::mint_token(recipient, id, metadata)?;
			Ok(())
		}
	}
}
