//! A shell pallet built with [`frame`].

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{
	Decode, Encode, alloc::string::ToString,
};
// use frame_support::{ pallet_prelude::*, ensure};
use frame_system::{
	pallet_prelude::*, WeightInfo
};
use scale_info::{prelude::vec::Vec, prelude::{string::String, format}, TypeInfo};
// use sp_runtime::{
// 	RuntimeDebug,
// };
use sp_core::crypto::KeyTypeId;
use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SigningTypes, SubmitTransaction,
	},
	pallet_prelude::BlockNumberFor,
};
use sp_runtime::{
	offchain::{
		http,
		storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
		Duration,
	},
	traits::Zero,
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug, BoundedVec,
};
use sp_io::offchain_index;
use serde::{Deserialize, Deserializer};
use sp_std::str;

pub type ClusterId = u64;

#[derive(Default, PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Ip {
	pub ipv4: Option<Vec<u32>>,
	pub ipv6: Option<Vec<u32>>,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct Worker<AccountId, BlockNumber> {
	pub id: ClusterId,
	pub account: AccountId,
	pub start_block: BlockNumber,
	pub name: Vec<u8>,
	pub ip: Ip,
	pub port: u32,
	pub status: u8,
}

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ping");

pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);
	pub struct ClusterStatusAuthId;

	// used for offchain worker transaction signing
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for ClusterStatusAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for ClusterStatusAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}
const ONCHAIN_TX_KEY: &[u8] = b"cluster::storage::tx";

#[derive(Debug, Deserialize, Encode, Decode, Default)]
struct IndexingData(Vec<u8>, u64);

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// Pallet event
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		// /// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
		/// Authority ID used for offchain worker
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
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

	/// Worker Cluster information
	#[pallet::storage]
	#[pallet::getter(fn get_worker_clusters)]
	pub type WorkerClusters<T: Config> = 
		StorageMap<_, Identity, ClusterId, Worker<T::AccountId, BlockNumberFor<T>>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		WorkerRegistered{ creator: T::AccountId },
		ConnectionEstablished{ cluster_id: ClusterId }
	}

	/// Pallet Errors
	#[pallet::error]
	pub enum Error<T> {
		WorkerRegisterMissingIp,
		WorkerRegisterMissingPort,
		ClusterExists,
		WorkerClusterNotRegistered,
	}

	// The pallet's hooks for offchain worker
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!("Hello from offchain workers!");

			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available");
				return
			}

			// Import `frame_system` and retrieve a block hash of the parent block.
			let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);

			// Reading back the off-chain indexing value. It is exactly the same as reading from
			// ocw local storage.
			let key = Self::derived_key(block_number);
			let oci_mem = StorageValueRef::persistent(&key);

			if let Ok(Some(data)) = oci_mem.get::<IndexingData>() {
				log::info!("off-chain indexing data: {:?}, {:?}",
					str::from_utf8(&data.0).unwrap_or("error"), data.1);
					if let Some(cluster) = Self::get_worker_clusters(data.1){
						log::info!("cluster info: {:?}", cluster);

						let response: String = Self::fetch_cluster_status(cluster.ip, cluster.port).unwrap_or_else(|e| {
							log::error!("fetch_response error: {:?}", e);
							"Failed".into()
						});
						log::info!("Response: {}", response);
						// use response to submit info to blockchain about cluster
			
					} else {
						log::info!("no retrieved.");
				};
			} else {
				log::info!("no off-chain indexing data retrieved.");
			}


			// This will send both signed and unsigned transactions
			// depending on the block number.
			// Usually it's enough to choose one or the other.
			// let should_send = Self::choose_transaction_type(block_number);
			// let res = match should_send {
			// 	TransactionType::Signed => Self::fetch_response_and_send_signed(),
			// 	TransactionType::UnsignedForAny =>
			// 		Self::fetch_response_and_send_unsigned_for_any_account(block_number),
			// 	TransactionType::UnsignedForAll =>
			// 		Self::fetch_response_and_send_unsigned_for_all_accounts(block_number),
			// 	TransactionType::Raw => Self::fetch_response_and_send_raw_unsigned(block_number),
			// 	TransactionType::None => Ok(()),
			// };
			// if let Err(e) = res {
			// 	log::error!("Error: {}", e);
			// }
		}
	}

	#[pallet::call]
	impl<T:Config> Pallet<T> {
		/// Worker cluster registration
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn worker_register(
			origin: OriginFor<T>,
			name: Vec<u8>,
			ip: Ip,
			port: u32,
		) -> DispatchResultWithPostInfo {
			let creator = ensure_signed(origin)?;

			//check ip
			ensure!(ip.ipv4.is_some() || ip.ipv6.is_some(), Error::<T>::WorkerRegisterMissingIp);
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

			// update storage
			WorkerAccounts::<T>::insert(creator.clone(), cid.clone());
			WorkerClusters::<T>::insert(cid.clone(), cluster);
			NextClusterId::<T>::mutate(|id| *id += 1);

			// update data from offchain worker on cluster healthcheck and metadata

			// Off-chain indexing allowing on-chain extrinsics to write to off-chain storage predictably
			// so it can be read in off-chain worker context. As off-chain indexing is called in on-chain
			// context, if it is agreed upon by the blockchain consensus mechanism, then it is expected
			// to run predicably by all nodes in the network.
			//
			// From an on-chain perspective, this is write-only and cannot be read back.
			//
			// The value is written in byte form, so we need to encode/decode it when writting/reading
			// a number to/from this memory space.
			
			let key = Self::derived_key(frame_system::Pallet::<T>::block_number());
			let data: IndexingData = IndexingData(b"submit_number_signed".to_vec(), cid);
			offchain_index::set(&key, &data.encode());

			// Emit an event.
			Self::deposit_event(Event::WorkerRegistered { creator });
	
			Ok(().into())
		}

		#[pallet::call_index(2)]
		#[pallet::weight({0})]
		/// Submit updates worker cluster status and information from successful connection.
		pub fn verify_connection(origin: OriginFor<T>, worker_index: ClusterId, response: String) -> DispatchResult {
			// Retrieve the signer and check it is valid.
			let who = ensure_signed(origin)?;
			// ensure!(Error::<T>::WorkerClusterNotRegistered{ cluster_id: worker_index })



			// WorkerClusters::<T>::try_mutate(worker_index |token_info| -> DispatchResult {
			// 	let cluster_info = token_info.as_mut().ok_or(Error::<T>::WorkerClusterNotRegistered)?;
			// 	// TODO: update this once response format finalizes, then extract value to item.response
			// 	// item = response // formated response about the cluster's config info
			// 	let dummy_value = 1;
			// 	token_info.status = dummy_value;

			// 	// update worker's info
			// 	WorkerClusters::<T>::insert(worker_index, token_info);

			// 	// Emit an event.
			// 	Self::deposit_event(Event::ConnectionEstablished { cluster_id: worker_index });
			// 	Ok(())
			// });


			// let mut item = WorkerClusters::<T>::get(worker_index);

			// Return a successful DispatchResult
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		#[deny(clippy::clone_double_ref)]
		fn derived_key(block_number: BlockNumberFor<T>) -> Vec<u8> {
			block_number.using_encoded(|encoded_bn| {
				ONCHAIN_TX_KEY
					.iter()
					.chain(b"/".iter())
					.chain(encoded_bn)
					.copied()
					.collect::<Vec<u8>>()
			})
		}
		/// Fetches the current cluster status response from remote URL and returns it as a string.
		fn fetch_cluster_status(ip: Ip, port: u32) -> Result<String, http::Error> {
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
			let string_ip = match ip {
				Ip { ipv4: Some(i), ..}  => i.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."),
				Ip { ipv6: Some(i), .. }  => i.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(":"),
				_=> {
					format!("localhost")
				}
			};
			
			let url = format!("http://{}:{}/status", string_ip, port);
			let request = http::Request::get(&url);
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

			log::info!("fetch_response: {}", body_str);

			let response: String = body_str.to_string();

			match response.len() {
				0 => Err(http::Error::Unknown),
				_ => Ok(response),
			}
		}

		// fn confirm_task_completion(ip: Ip, port: u32) -> Result<String, http::Error> {
		// 	// fetch existing tasks
		// 	// if task is pending, call remote http url to fetch job status
		// 	// if complete or error, update job status
		// 	// default: if exceed an interval set job status to failed
		// }
		
	}
	
}
