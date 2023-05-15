//! Pallet Edge Connect
//! 
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//! - [`Error`]
//! - [`Event`]
//! - [`Storage`]
//! 
//! ## Overview
//! 
//! Edge Connect is a pallet that allows users to create and remove connections between Cyborg blockchain and external edge servers.
//! 
//! ## Interface
//! 
//! ### Dispatchable Functions
//! 
//! * `create_connection` - Creates a connection between Cyborg blockchain and an external edge server.
//! * `remove_connection` - Removes a connection between Cyborg blockchain and an external edge server.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use sp_core::crypto::KeyTypeId;
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
	SignedPayload, Signer, SigningTypes, SubmitTransaction,
};

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"edge");

pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Authority ID used for offchain worker
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn connection)]
	pub type Connection<T> = StorageValue<_, u32>; // TODO: change to the proper data structure

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [connection, who]
		ConnectionCreated { connection: u32, who: T::AccountId },
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [connection, who]
		ConnectionRemoved { connection: u32, who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Returned if the connection already exists.
        ConnectionAlreadyExists,
        /// Returned if the connection does not exist.
        ConnectionDoesNotExist,
	}

	// The pallet's hooks for offchain worker
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: T::BlockNumber) {
			log::info!("Hello from offchain workers!");

			let signer = Signer::<T, T::AuthorityId>::all_accounts();

			// Using `send_signed_transaction` associated type we create and submit a transaction
			// representing the call we've just created.
			// `send_signed_transaction()` return type is `Option<(Account<T>, Result<(), ()>)>`. It is:
			//	 - `None`: no account is available for sending transaction
			//	 - `Some((account, Ok(())))`: transaction is successfully sent
			//	 - `Some((account, Err(())))`: error occurred when sending the transaction
			let results = signer.send_signed_transaction(|_account| {
				// This is the on-chain function
				Call::create_connection {
					connection: 0,
				}
			});

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!("[{:?}]: submit transaction success.", acc.id),
					Err(e) => log::error!("[{:?}]: submit transaction failure. Reason: {:?}", acc.id, e),
				}
			}
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn create_connection(origin: OriginFor<T>, connection: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			// Check that the connection does not already exist.
			ensure!(!<Connection<T>>::exists(), Error::<T>::ConnectionAlreadyExists);

			// Update storage.
			<Connection<T>>::put(connection);

			// Emit an event.
			Self::deposit_event(Event::ConnectionCreated { connection, who });

			// Return a successful DispatchResult
			Ok(())
		}

		// TODO: create remove_connection fn
	}
}
