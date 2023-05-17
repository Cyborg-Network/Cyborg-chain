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
//! Edge Connect is a pallet that allows users to create and remove connections between Cyborg
//! blockchain and external edge servers.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `create_connection` - Creates a connection between Cyborg blockchain and an external edge
//!   server.
//! * `send_command` - Sends a command to CyberHub.
//! * `receive_response` - Receives a response from CyberHub.
//! * `remove_connection` - Removes a connection between Cyborg blockchain and an external edge
//!   server.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SigningTypes, SubmitTransaction,
	},
};
use scale_info::prelude::string::String;
use lite_json::json::JsonValue;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	offchain::{
		http,
		storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
		Duration,
	},
	traits::Zero,
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug,
};
use sp_std::vec::Vec;

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

pub use pallet::*;

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

		// Configuration parameters

		/// A grace period after we send transaction.
		///
		/// To avoid sending too many transactions, we only attempt to send one
		/// every `GRACE_PERIOD` blocks. We use Local Storage to coordinate
		/// sending between distinct runs of this offchain worker.
		#[pallet::constant]
		type GracePeriod: Get<Self::BlockNumber>;
	}

	// The pallet's hooks for offchain worker
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello from offchain workers!");

			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available");
				return
			}

			// Import `frame_system` and retrieve a block hash of the parent block.
			let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);

			let response: String = Self::fetch_response().unwrap_or_else(|e| {
				log::error!("fetch_response error: {:?}", e);
				"Failed".into()
			});
			log::info!("Response: {}", response);

			// This will send both signed and unsigned transactions
			// depending on the block number.
			// Usually it's enough to choose one or the other.
			let should_send = Self::choose_transaction_type(block_number);
			let res = match should_send {
				TransactionType::Signed => Self::fetch_response_and_send_signed(),
				TransactionType::UnsignedForAny =>
					Self::fetch_response_and_send_unsigned_for_any_account(block_number),
				TransactionType::UnsignedForAll =>
					Self::fetch_response_and_send_unsigned_for_all_accounts(block_number),
				TransactionType::Raw => Self::fetch_response_and_send_raw_unsigned(block_number),
				TransactionType::None => Ok(()),
			};
			if let Err(e) = res {
				log::error!("Error: {}", e);
			}
		}
	}

	// Public part of the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create connection
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

		// TODO:
		// Create functions for:
		// 1. send command
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn send_command(origin: OriginFor<T>, command: String) -> DispatchResult {
			// Retrieve the signer and check it is valid.
			let who = ensure_signed(origin)?;

			// Check that the connection exists.
			ensure!(<Connection<T>>::exists(), Error::<T>::ConnectionDoesNotExist);

			// TODO: send command to ocw

			// Return a successful DispatchResult
			Ok(());
		}

		// 2. submit_response (ocw * 3)
		#[pallet::call_index(2)]
		#[pallet::weight({0})]
		pub fn submit_response(origin: OriginFor<T>, response: String) -> DispatchResult {
			// Retrieve the signer and check it is valid.
			let who = ensure_signed(origin)?;

			// Check that the connection exists.
			ensure!(<Connection<T>>::exists(), Error::<T>::ConnectionDoesNotExist);

			// Submit response received from CyberHub
			Self::add_response(Some(who), response);

			// Return a successful DispatchResult
			Ok(().into())
		}

		// 3. remove_connection
		#[pallet::call_index(3)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn remove_connection(origin: OriginFor<T>, connection: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			// Check that the connection exists.
			ensure!(<Connection<T>>::exists(), Error::<T>::ConnectionDoesNotExist);

			// Update storage.
			<Connection<T>>::kill();

			// Emit an event.
			Self::deposit_event(Event::ConnectionRemoved { connection, who });

			// Return a successful DispatchResult
			Ok(());
		}
	}

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn connection)]
	pub type Connection<T> = StorageValue<_, u32>; // TODO: change to the proper data structure

	/// A vector of recently submitted responses.
	#[pallet::storage]
	#[pallet::getter(fn responses)]
	pub(super) type Responses<T: Config> =
		StorageMap<_, Blake2_128Concat, Option<T::AccountId>, Vec<String>>;

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
		/// Event generated when a response is received from CyberHub.
		/// [response, maybe_who]
		NewResponse { response: String, maybe_who: Option<T::AccountId> },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Returned if the connection already exists.
		ConnectionAlreadyExists,
		/// Returned if the connection does not exist.
		ConnectionDoesNotExist,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			if let Call::submit_response_unsigned_with_signed_payload {
				response_payload: ref payload,
				ref signature,
			} = call
			{
				let signature_valid =
					SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
				if !signature_valid {
					return InvalidTransaction::BadProof.into()
				}
				Self::validate_transaction_parameters(&payload.block_number, &payload.response)
			} else if let Call::submit_response_unsigned { block_number, response: new_response } =
				call
			{
				Self::validate_transaction_parameters(block_number, new_response)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}
}

enum TransactionType {
	Signed,
	UnsignedForAny,
	UnsignedForAll,
	Raw,
	None,
}

impl<T: Config> Pallet<T> {
	/// Chooses which transaction type to send.
	///
	/// This function serves mostly to showcase `StorageValue` helper
	/// and local storage usage.
	///
	/// Returns a type of transaction that should be produced in current run.
	fn choose_transaction_type(block_number: T::BlockNumber) -> TransactionType {
		/// A friendlier name for the error that is going to be returned in case we are in the grace
		/// period.
		const RECENTLY_SENT: () = ();

		// Start off by creating a reference to Local Storage value.
		// Since the local storage is common for all offchain workers, it's a good practice
		// to prepend your entry with the module name.
		let val = StorageValueRef::persistent(b"example_ocw::last_send");
		// The Local Storage is persisted and shared between runs of the offchain workers,
		// and offchain workers may run concurrently. We can use the `mutate` function, to
		// write a storage entry in an atomic fashion. Under the hood it uses `compare_and_set`
		// low-level method of local storage API, which means that only one worker
		// will be able to "acquire a lock" and send a transaction if multiple workers
		// happen to be executed concurrently.
		let res = val.mutate(|last_send: Result<Option<T::BlockNumber>, StorageRetrievalError>| {
			match last_send {
				// If we already have a value in storage and the block number is recent enough
				// we avoid sending another transaction at this time.
				Ok(Some(block)) if block_number < block + T::GracePeriod::get() =>
					Err(RECENTLY_SENT),
				// In every other case we attempt to acquire the lock and send a transaction.
				_ => Ok(block_number),
			}
		});

		// The result of `mutate` call will give us a nested `Result` type.
		// The first one matches the return of the closure passed to `mutate`, i.e.
		// if we return `Err` from the closure, we get an `Err` here.
		// In case we return `Ok`, here we will have another (inner) `Result` that indicates
		// if the value has been set to the storage correctly - i.e. if it wasn't
		// written to in the meantime.
		match res {
			// The value has been set correctly, which means we can safely send a transaction now.
			Ok(block_number) => {
				// We will send different transactions based on a random number.
				// Note that this logic doesn't really guarantee that the transactions will be sent
				// in an alternating fashion (i.e. fairly distributed). Depending on the execution
				// order and lock acquisition, we may end up for instance sending two `Signed`
				// transactions in a row. If a strict order is desired, it's better to use
				// the storage entry for that. (for instance store both block number and a flag
				// indicating the type of next transaction to send).
				let transaction_type = block_number % 4u32.into();
				if transaction_type == Zero::zero() {
					TransactionType::Signed
				} else if transaction_type == T::BlockNumber::from(1u32) {
					TransactionType::UnsignedForAny
				} else if transaction_type == T::BlockNumber::from(2u32) {
					TransactionType::UnsignedForAll
				} else {
					TransactionType::Raw
				}
			},
			// We are in the grace period, we should not send a transaction this time.
			Err(MutateStorageError::ValueFunctionFailed(RECENTLY_SENT)) => TransactionType::None,
			// We wanted to send a transaction, but failed to write the block number (acquire a
			// lock). This indicates that another offchain worker that was running concurrently
			// most likely executed the same logic and succeeded at writing to storage.
			// Thus we don't really want to send the transaction, knowing that the other run
			// already did.
			Err(MutateStorageError::ConcurrentModification(_)) => TransactionType::None,
		}
	}

	/// A helper function to fetch the response and send signed transaction.
	fn fetch_response_and_send_signed() -> Result<(), &'static str> {
		let signer = Signer::<T, T::AuthorityId>::all_accounts();
		if !signer.can_sign() {
			return Err(
				"No local accounts available. Consider adding one via `author_insertKey` RPC.",
			)
		}
		// Make an external HTTP request to fetch the current response.
		// Note this call will block until response is received.
		let response = Self::fetch_response().map_err(|_| "Failed to fetch response")?;

		// Using `send_signed_transaction` associated type we create and submit a transaction
		// representing the call, we've just created.
		// Submit signed will return a vector of results for all accounts that were found in the
		// local keystore with expected `KEY_TYPE`.
		let results = signer.send_signed_transaction(|_account| {
			// Received response is wrapped into a call to `submit_response` public function of this
			// pallet. This means that the transaction, when executed, will simply call that
			// function passing `response` as an argument.
			Call::submit_response { response }
		});

		for (acc, res) in &results {
			match res {
				Ok(()) => log::info!("[{:?}] Submitted response: {}", acc.id, response),
				Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
			}
		}

		Ok(())
	}

	/// Fetches the current response from remote URL and returns it as a string.
	// TODO: change http to websocket
	fn fetch_response() -> Result<String, http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 3s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));
		// Initiate an external HTTP GET request.
		// This is using high-level wrappers from `sp_runtime`, for the low-level calls that
		// you can find in `sp_io`. The API is trying to be similar to `reqwest`, but
		// since we are running in a custom WASM execution environment we can't simply
		// import the library here.
		let request = http::Request::get("https://<cyberhub-url>");
		// We set the deadline for sending of the request, note that awaiting response can
		// have a separate deadline. Next we send the request, before that it's also possible
		// to alter request headers or stream body content in case of non-GET requests.
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

		// The request is already being processed by the host, we are free to do anything
		// else in the worker (we can send multiple concurrent requests too).
		// At some point however we probably want to check the response though,
		// so we can block current thread and wait for it to finish.
		// Note that since the request is being driven by the host, we don't have to wait
		// for the request to have it complete, we will just not read the response.
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		// Let's check the status code before we proceed to reading the response.
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}

		// Next we want to fully read the response body and collect it to a vector of bytes.
		// Note that the return object allows you to read the body in chunks as well
		// with a way to control the deadline.
		let body = response.body().collect::<Vec<u8>>();

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;

		let response = if body_str == "pong" {
			Ok(String::from(body_str))
		} else {
			log::warn!("Unable to extract response from the CyberHub: {:?}", body_str);
			Err(http::Error::Unknown)
		};		

		response
	}

	/// Add new response to the list.
	fn add_response(maybe_who: Option<T::AccountId>, response: String) {
		log::info!("Adding response to the list: {}", response);

		// Add the new response to the list.
		if let Some(who) = maybe_who.clone() {
			Responses::<T>::mutate(&who, |responses: &mut Vec<(String, Option<T::AccountId>)>| {
				responses.push((response.clone(), maybe_who.clone()));
			});
			// here we are raising the NewResponse event
			Self::deposit_event(Event::NewResponse { response, maybe_who });
		}
	}
}
