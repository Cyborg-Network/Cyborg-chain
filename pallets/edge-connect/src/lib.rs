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

#[frame_support::pallet]
pub mod pallet {

	use codec::{Decode, Encode};
	use frame_support::{ensure, traits::Get};
	use frame_system::{
		self as system,
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			SignedPayload, Signer, SigningTypes, SubmitTransaction,
		},
	};
	use scale_info::prelude::string::String;
	use sp_core::crypto::KeyTypeId;
	use sp_runtime::{
		offchain::{
			http,
			storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
			storage_lock::{BlockAndTime, StorageLock},
			Duration,
		},
		traits::Zero,
		transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
		BoundedVec, RuntimeDebug,
	};
	// use sp_std::vec::Vec;
	use serde::{Deserialize, Deserializer};
	use sp_std::prelude::*;

	// #[cfg(test)]
	// mod mock;

	// #[cfg(test)]
	// mod tests;

	// #[cfg(feature = "runtime-benchmarks")]
	// mod benchmarking;

	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"edge");

	const HTTP_REMOTE_REQUEST: &str = "http://127.0.0.1:9000/block";

	const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds

	const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds

	const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

	/// The type to sign and send transactions.
	const UNSIGNED_TXS_PRIORITY: u64 = 100;

	pub mod crypto {
		use super::*;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_runtime::{
			app_crypto::{app_crypto, sr25519},
			traits::Verify,
			MultiSignature, MultiSigner,
		};
		use sp_std::prelude::*;
		app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;

		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		// implemented for mock runtime in test
		impl
			frame_system::offchain::AppCrypto<
				<Sr25519Signature as Verify>::Signer,
				Sr25519Signature,
			> for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	#[derive(Deserialize, Encode, Decode, Default, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ResponseData {
		// Specify deserializing function to convert JSON string to vector of bytes
		#[serde(deserialize_with = "de_string_to_bytes")]
		pub response_type: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		pub id: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		pub response_ref: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		pub timestamp: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		pub status_code: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		pub node_id: Vec<u8>,
		pub args: Vec<Vec<u8>>,
		pub data: Vec<Vec<u8>>,
	}

	pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
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

		/// Number of blocks of cooldown after unsigned transaction is included.
		///
		/// This ensures that we only accept unsigned transactions once, every `UnsignedInterval`
		/// blocks.
		#[pallet::constant]
		type UnsignedInterval: Get<Self::BlockNumber>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		/// Maximum number of responses received per request
		#[pallet::constant]
		type MaxResponses: Get<u32>;

		/// The maximum length of a response string.
		#[pallet::constant]
		type MaxStringLength: Get<u32>;
	}

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn connection)]
	pub type Connection<T> = StorageValue<_, u32>; // TODO: change to the proper data structure

	/// A vector of recently submitted responses.
	#[pallet::storage]
	#[pallet::getter(fn responses)]
	pub(super) type Responses<T: Config> = StorageValue<
		_,
		BoundedVec<(Option<T::AccountId>, BoundedVec<u8, T::MaxStringLength>), T::MaxResponses>,
		ValueQuery,
	>;

	/// Defines the block when next unsigned transaction will be accepted.
	///
	/// To prevent spam of unsigned (and unpayed!) transactions on the network,
	/// we only allow one transaction every `T::UnsignedInterval` blocks.
	/// This storage entry defines when new transaction is going to be accepted.
	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

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
		/// Event generated when a new command is sent to CyberHub.
		/// [command, who]
		CommandSent { command: String, who: T::AccountId },
		/// Event generated when a response is received from CyberHub.
		/// [response, maybe_who]
		NewResponse { maybe_who: Option<T::AccountId>, response: String },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Returned if the connection already exists.
		ConnectionAlreadyExists,
		/// Returned if the connection does not exist.
		ConnectionDoesNotExist,
		/// Returned if the response is too large.
		ResponseTooLarge,
		/// Return error if the command is not valid.
		InvalidCommand,

		/// Returned if ocw function executed is not supported.
		UnknownOffchainTx,

		// Error returned when making signed transactions in off-chain worker
		NoLocalAcctForSigning,
		OffchainSignedTxError,

		// Error returned when making unsigned transactions in off-chain worker
		OffchainUnsignedTxError,

		// Error returned when making unsigned transactions with signed payloads in off-chain
		// worker
		OffchainUnsignedTxSignedPayloadError,

		// Error returned when fetching github info
		HttpFetchingError,
		DeserializeToObjError,
		DeserializeToStrError,
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

			// Here we are showcasing various techniques used when running off-chain workers (ocw)
			// 1. Sending signed transaction from ocw
			// 2. Sending unsigned transaction from ocw
			// 3. Sending unsigned transactions with signed payloads from ocw
			// 4. Fetching JSON via http requests in ocw
			const TX_TYPES: u32 = 4;
			let modu = block_number.try_into().map_or(TX_TYPES, |bn: usize| (bn as u32) % TX_TYPES);
			let result = match modu {
				0 => Self::offchain_signed_tx(block_number),
				1 => Self::offchain_unsigned_tx(block_number),
				2 => Self::offchain_unsigned_tx_signed_payload(block_number),
				3 => Self::fetch_remote_response(),
				_ => Err(Error::<T>::UnknownOffchainTx),
			};

			if let Err(e) = result {
				log::error!("Error: {}", e);
			}
		}
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
			let valid_tx = |provide| {
				ValidTransaction::with_tag_prefix("ocw-edge")
					.priority(UNSIGNED_TXS_PRIORITY)
					.and_provides([&provide])
					.longevity(3)
					.propagate(true)
					.build()
			};

			match call {
				Call::submit_response_unsigned { response: _response } =>
					valid_tx(b"submit_response_unsigned".to_vec()),
				Call::submit_response_unsigned_with_signed_payload {
					ref payload,
					ref signature,
				} => {
					if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
						return InvalidTransaction::BadProof.into()
					}
					valid_tx(b"submit_response_unsigned_with_signed_payload".to_vec())
				},
				_ => InvalidTransaction::Call.into(),
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

			// Make sure the `command` == `ping`
			ensure!(command == "ping", Error::<T>::InvalidCommand);

			// Send a `ping` command to CyberHub
			let _request = Self::send_ping();

			// TODO [DISCUSSION]: At this point, it would typically pass the request to an offchain
			// worker to send it. In the current form, the `send_ping` function only builds the
			// request, it does not send it.

			// Emit an event.
			Self::deposit_event(Event::CommandSent { command, who });

			// Return a successful DispatchResult
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		/// Submit new response to the list.
		///
		/// This method is a public function of the module and can be called from within
		/// a transaction. It appends given `response` to current list of responses.
		/// The `offchain worker` will create, sign & submit a transaction that
		/// calls this function passing the response.
		///
		/// The transaction needs to be signed (see `ensure_signed`) check, so that the caller
		/// pays a fee to execute it.
		/// This makes sure that it's not easy (or rather cheap) to attack the chain by submitting
		/// excessive transactions.
		pub fn submit_response_signed(origin: OriginFor<T>, response: String) -> DispatchResult {
			// Retrieve the signer and check it is valid.
			let who = ensure_signed(origin)?;

			// Check that the connection exists.
			ensure!(<Connection<T>>::exists(), Error::<T>::ConnectionDoesNotExist);

			log::info!("submit_response_signed: ({}, {:?})", response, who);

			// Submit response received from CyberHub
			Self::add_response(response);

			// Self::deposit_event(Event::NewResponse { Some(who), response });

			// Return a successful DispatchResult
			Ok(())
		}

		/// Submit new response to the list via unsigned transaction.
		///
		/// Works exactly like the `submit_response_signed` function, but since we allow sending the
		/// transaction without a signature, and hence without paying any fees,
		/// we need a way to make sure that only some transactions are accepted.
		/// This function can be called only once every `T::UnsignedInterval` blocks.
		/// Transactions that call that function are de-duplicated on the pool level
		/// via `validate_unsigned` implementation and also are rendered invalid if
		/// the function has already been called in current "session".
		///
		/// It's important to specify `weight` for unsigned calls as well, because even though
		/// they don't charge fees, we still don't want a single block to contain unlimited
		/// number of such transactions.
		///
		/// This example is not focused on correctness of the oracle itself, but rather its
		/// purpose is to showcase offchain worker capabilities.
		#[pallet::call_index(3)]
		#[pallet::weight(10_000)]
		pub fn submit_response_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			response: String,
		) -> DispatchResult {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			log::info!("submit_response_unsigned: {}", response);
			// Add the response to the on-chain list, but mark it as coming from an empty address.
			Self::add_response(response);
			// now increment the block number at which we expect next unsigned transaction.
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());

			Self::deposit_event(Event::NewResponse { None, response });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10_000)]
		pub fn submit_response_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			response_payload: ResponsePayload<T::Public, T::BlockNumber>,
			signature: T::Signature,
		) -> DispatchResult {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			// now increment the block number at which we expect next unsigned transaction.
			log::info!(
				"submit_response_unsigned_with_signed_payload: ({}, {:?})",
				response,
				public
			);
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());

			// Add the response to the on-chain list, but mark it as coming from an empty address.
			Self::add_response(None, response_payload.response);

			Self::deposit_event(Event::NewResponse { None, response });

			Ok(())
		}

		// 3. remove_connection
		#[pallet::call_index(5)]
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
			Ok(())
		}
	}

	/// Payload used by this crate to hold response
	/// data required to submit a transaction.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ResponsePayload<Public, BlockNumber> {
		block_number: BlockNumber,
		response: String,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for ResponsePayload<T::Public, T::BlockNumber> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	impl<T: Config> Pallet<T> {
		// Add new response to the list.
		fn add_response(response: String) {
			// Get the current list of responses.
			let mut responses = Responses::<T>::get();

			// Attempt to append the new response to the list.
			match responses.try_push(response.clone()) {
				// The new response has been added to the list.
				Ok(_) => {
					// Write the updated list of responses to storage.
					Responses::<T>::put(responses);
				},
				// The offchain worker is already running, so we don't need to do anything.
				Err(_) => {
					log::warn!("Unable to add response. Maximum number of responses reached.");
				},
			}
		}

		// Create a reference to Local Storage value.
		// Since the local storage is common for all offchain workers, it's a good practice
		// to prepend our entry with the pallet name.
		// TODO: change http to websocket
		fn fetch_remote_response() -> Result<(), Error<T>> {
			// Create a reference to Local Storage value.
			// Since the local storage is common for all offchain workers, it's a good practice
			// to prepend our entry with the pallet name.
			let s_info = StorageValueRef::persistent(b"offchain-edge::ch-info");

			// Local storage is persisted and shared between runs of the offchain workers,
			// offchain workers may run concurrently. We can use the `mutate` function to
			// write a storage entry in an atomic fashion.
			//
			// With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
			// We will likely want to use `mutate` to access
			// the storage comprehensively.
			//
			if let Ok(Some(info)) = s_info.get::<ResponseData>() {
				// ch-info has already been fetched. Return early.
				log::info!("cached ch-info: {:?}", info);
				return Ok(())
			}

			// Since off-chain storage can be accessed by off-chain workers from multiple runs,
			// it is important to lock   it before doing heavy computations or write operations.
			//
			// There are four ways of defining a lock:
			//   1) `new` - lock with default time and block exipration
			//   2) `with_deadline` - lock with default block but custom time expiration
			//   3) `with_block_deadline` - lock with default time but custom block expiration
			//   4) `with_block_and_time_deadline` - lock with custom time and block expiration
			// Here we choose the most custom one for demonstration purpose.
			let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
				b"offchain-demo::lock",
				LOCK_BLOCK_EXPIRATION,
				Duration::from_millis(LOCK_TIMEOUT_EXPIRATION),
			);

			// We try to acquire the lock here. If failed, we know the `fetch_n_parse` part
			// inside is being   executed by previous run of ocw, so the function just returns.
			if let Ok(_guard) = lock.try_lock() {
				match Self::fetch_n_parse() {
					Ok(info) => {
						s_info.set(&info);
					},
					Err(err) => return Err(err),
				}
			}

			Ok(())
		}

		/// Fetch from remote and deserialize the JSON to a struct using `serde-json`.
		fn fetch_n_parse() -> Result<ResponseData, Error<T>> {
			let resp_bytes = Self::fetch_from_remote().map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;

			let resp_str =
				str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::DeserializeToStrError)?;
			// Print out our fetched JSON string
			log::info!("fetch_n_parse: {}", resp_str);

			// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
			let info: ResponseData =
				serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::DeserializeToObjError)?;
			Ok(info)
		}

		/// This function uses the `offchain::http` API to query the remote endpoint
		/// information,   and returns the JSON response as vector of bytes.
		fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
			// Initiate an external HTTP GET request. This is using high-level wrappers from
			// `sp_runtime`.
			let request = http::Request::get(HTTP_REMOTE_REQUEST);

			// Keeping the offchain worker execution time reasonable, so limiting the call to be
			// within 3s.
			let timeout =
				sp_io::offchain::timestamp().add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));

			let pending = request
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|e| {
					log::error!("{:?}", e);
					<Error<T>>::HttpFetchingError
				})?;

			// By default, the http request is async from the runtime perspective. So we are
			// asking the   runtime to wait here
			// The returning value here is a `Result` of `Result`, so we are unwrapping it twice
			// by two `?`   ref: https://docs.substrate.io/rustdocs/latest/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
			let http_response = pending
				.try_wait(timeout)
				.map_err(|e| {
					log::error!("{:?}", e);
					<Error<T>>::HttpFetchingError
				})?
				.map_err(|e| {
					log::error!("{:?}", e);
					<Error<T>>::HttpFetchingError
				})?;

			if http_response.code != 200 {
				log::error!("Unexpected http request status code: {}", http_response.code);
				return Err(<Error<T>>::HttpFetchingError)
			}

			// Next we fully read the response body and collect it to a vector of bytes.
			Ok(http_response.body().collect::<Vec<u8>>())
		}

		fn offchain_signed_tx(block_number: T::BlockNumber) -> Result<(), Error<T>> {
			// We retrieve a signer and check if it is valid.
			//   Since this pallet only has one key in the keystore. We use `any_account()1 to
			//   retrieve it. If there are multiple keys and we want to pinpoint it,
			// `with_filter()` can be chained,
			let signer = Signer::<T, T::AuthorityId>::any_account();

			// Translating the current block number to number and submit it on-chain
			let number: u64 = block_number.try_into().unwrap_or(0);

			// `result` is in the type of `Option<(Account<T>, Result<(), ()>)>`. It is:
			//   - `None`: no account is available for sending transaction
			//   - `Some((account, Ok(())))`: transaction is successfully sent
			//   - `Some((account, Err(())))`: error occured when sending the transaction
			let result = signer.send_signed_transaction(|_acct|
			// This is the on-chain function
			Call::submit_response_signed { response });

			// Display error if the signed tx fails.
			if let Some((acc, res)) = result {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(<Error<T>>::OffchainSignedTxError)
				}
				// Transaction is sent successfully
				return Ok(())
			}

			// The case of `None`: no account is available for sending
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}

		fn offchain_unsigned_tx(block_number: T::BlockNumber) -> Result<(), Error<T>> {
			let number: u64 = block_number.try_into().unwrap_or(0);
			let call = Call::submit_response_unsigned { response };

			// `submit_unsigned_transaction` returns a type of `Result<(), ()>`
			//   ref: https://substrate.dev/rustdocs/v2.0.0/frame_system/offchain/struct.SubmitTransaction.html#method.submit_unsigned_transaction
			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).map_err(
				|_| {
					log::error!("Failed in offchain_unsigned_tx");
					<Error<T>>::OffchainUnsignedTxError
				},
			)
		}

		fn offchain_unsigned_tx_signed_payload(
			block_number: T::BlockNumber,
		) -> Result<(), Error<T>> {
			// Retrieve the signer to sign the payload
			let signer = Signer::<T, T::AuthorityId>::any_account();

			let number: u64 = block_number.try_into().unwrap_or(0);

			// `send_unsigned_transaction` is returning a type of `Option<(Account<T>,
			// Result<(), ()>)>`.   Similar to `send_signed_transaction`, they account for:
			//   - `None`: no account is available for sending transaction
			//   - `Some((account, Ok(())))`: transaction is successfully sent
			//   - `Some((account, Err(())))`: error occured when sending the transaction
			if let Some((_, res)) = signer.send_unsigned_transaction(
				|acct| ResponsePayload { number, public: acct.public.clone() },
				|payload, signature| Call::submit_response_unsigned_with_signed_payload {
					payload,
					signature,
				},
			) {
				return res.map_err(|_| {
					log::error!("Failed in offchain_unsigned_tx_signed_payload");
					<Error<T>>::OffchainUnsignedTxSignedPayloadError
				})
			}

			// The case of `None`: no account is available for sending
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}
	}

	// fn send_ping() -> http::Request<'static, &'static str> {
	// 	// Send the `ping` command to CyberHub
	// 	let post_request = http::Request::post(
	// 		"http://127.0.0.1:9000/ws",
	// 		r#"{"command": "ping"}"#,
	// 	);

	// 	post_request
	// }
}
