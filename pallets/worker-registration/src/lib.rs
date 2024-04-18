//! A shell pallet built with [`frame`].

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{
	Decode, Encode,
};
use frame_support::{ pallet_prelude::*, ensure};
use frame_system::{
	pallet_prelude::*, WeightInfo
};
use scale_info::{prelude::vec::Vec, TypeInfo};
use sp_runtime::{
	RuntimeDebug,
};

pub type ClusterId = u64;

#[derive(Default, PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Ip {
	pub ipv4: Option<u32>,
	pub ipv6: Option<u32>,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Worker<AccountId, BlockNumber> {
	pub id: ClusterId,
	pub account: AccountId,
	pub start_block: BlockNumber,
	pub name: Vec<u8>,
	pub ip: Vec<Ip>,
	pub port: u32,
	pub status: u8,
}

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Pallet event
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		// /// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::type_value]
    pub fn DefaultForm1() -> ClusterId {
        1
    }

	// /// Id of the next cluster of worker to be registered
    #[pallet::storage]
    #[pallet::getter(fn get_next_cluster_id)]
    pub type NextClusterId<T: Config> = StorageValue<_, ClusterId, ValueQuery, DefaultForm1>;


	/// user's Worker information
	#[pallet::storage]
	#[pallet::getter(fn get_worker_accounts)]
	pub type WorkerAccounts<T: Config> = 
		StorageMap<_, Identity, T::AccountId, ClusterId, OptionQuery>;

	// /// Worker Cluster information
	// #[pallet::storage]
	// #[pallet::getter(fn get_worker_clusters)]
	// pub type WorkerClusters<T: Config> = 
	// 	StorageMap<_, Identity, ClusterId, Worker<T::AccountId, BlockNumberFor<T>>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		WorkerRegistered{ creator: T::AccountId },
	}

	/// Pallet Errors
	#[pallet::error]
	pub enum Error<T> {
		WorkerRegisterMissingIp,
		WorkerRegisterMissingPort,
		ClusterExists,
	}

	#[pallet::call]
	impl<T:Config> Pallet<T> {
		/// Worker cluster registration
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn worker_register(
			origin: OriginFor<T>,
			name: Vec<u8>,
			ip: Vec<Ip>,
			port: u32,
		) -> DispatchResultWithPostInfo {
			let creator = ensure_signed(origin)?;

			//check ip
			ensure!(ip.len() > 0, Error::<T>::WorkerRegisterMissingIp);
			ensure!(port > 0, Error::<T>::WorkerRegisterMissingPort);
			
			//check cluster
			ensure!(WorkerAccounts::<T>::contains_key(creator.clone()) == false, 
			Error::<T>::ClusterExists);

			let cid = NextClusterId::<T>::get();

			let cluster = Worker {
				id: cid.clone(),
				account: creator.clone(),
				start_block: <frame_system::Pallet<T>>::block_number(),
				name: name.clone(),
				ip: ip.clone(),
				port: port.clone(),
				status: 1,
			};

			//update storage
			WorkerAccounts::<T>::insert(creator.clone(), cid.clone());
			// WorkerClusters::<T>::insert(cid.clone(), cluster);
			NextClusterId::<T>::mutate(|id| *id += 1);
			//update data from offchain worker on cluster healthcheck and metadata

			// Emit an event.
			Self::deposit_event(Event::WorkerRegistered { creator });
	
			Ok(().into())
		}
	}
}
