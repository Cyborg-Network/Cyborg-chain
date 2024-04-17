//! A shell pallet built with [`frame`].

#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;

// Re-export all pallet parts, this is needed to properly import the pallet into the runtime.
pub use pallet::*;

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Worker<AccountId, BlockNumber> {
	pub account_id: AccountId,
	pub start_block: BlockNumber,
	pub name: Vec<u8>,
	pub ip: Vec<u8>,
	pub port: u32,
	pub status: u8,
}

#[frame::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	// #[pallet::storage]
	// #[pallet::getter(fn get_worker_accounts)]
	// pub type WorkerAccounts<T: Config> = 
	// 	StorageMap<_, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		WorkerRegisterMissingIp,
		WorkerRegisterMissingPort,
	}

	#[pallet::call]
	impl<T:Config> Pallet<T> {
		/// Worker cluster registration
		#[pallet::call_index(001)]
		pub fn worker_register(
			origin: OriginFor<T>,
			name: Vec<u8>,
			ip: Vec<u8>,
			port: u32,
			level: u8,
		) -> DispatchResultWithPostInfo {
			let creator = ensure_signed(origin)?;

			ensure!(ip.len() > 0, Error<T>::WorkerRegisterMissingIp);
			ensure!(port > 0, Error<T>::WorkerRegisterMissingPort);
		}
	}
}