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

use frame_support::{traits::Get, ensure};
use scale_info::prelude::string::String;
use sp_runtime::BoundedVec;

// #[cfg(test)]
// mod tests;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Max number of commands sent per request
		#[pallet::constant]
		type MaxCommand: Get<u32>;

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
	pub type Connection<T> = StorageValue<_, u32, ValueQuery>;

	/// A vector of recently submitted commands.
	#[pallet::storage]
	#[pallet::getter(fn commands)]
	pub type Commands<T: Config> =
		StorageValue<_, BoundedVec<(Option<T::AccountId>, BoundedVec<u8, T::MaxStringLength>), T::MaxCommand>, ValueQuery>;


	/// A vector of recently submitted responses.
	#[pallet::storage]
	#[pallet::getter(fn responses)]
	pub(super) type Responses<T: Config> =
		StorageValue<_, BoundedVec<(Option<T::AccountId>, BoundedVec<u8, T::MaxStringLength>), T::MaxResponses>, ValueQuery>;

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
		NewResponse { response: String, maybe_who: Option<T::AccountId> },
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
		/// Returned if the command is too long.
		CommandTooLong,
		/// Returned if the command are too many.
		TooManyCommands,
	}

	// Public part of the pallet.
	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
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

			// Convert the command to BoundedVec<u8, T::MaxStringLength>
			let command_bounded_vec: BoundedVec<u8, T::MaxStringLength> = 
				command.clone().into_bytes().try_into().map_err(|_| Error::<T>::CommandTooLong)?;

			// Create the command tuple (Option<T::AccountId>, BoundedVec<u8, T::MaxStringLength>)
			let command_tuple = (Some(who.clone()), command_bounded_vec);

			// Try to get the current commands
			let mut current_commands = <Commands<T>>::get();

			// Try to push the new command tuple, if there's room
			if !current_commands.try_push(command_tuple).is_ok() {
				return Err(Error::<T>::TooManyCommands.into());
			}

			// Update the storage.
			<Commands<T>>::put(current_commands);

			// Emit an event.
			Self::deposit_event(Event::CommandSent { command, who });

			// Return a successful DispatchResult
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight({0})]
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


}

impl<T: Config> Pallet<T> {
	/// Add new response to the list.
	fn add_response(maybe_who: Option<T::AccountId>, response: String) {
		log::info!("Adding response to the list: {}", response);

		// Convert the string to a byte vector.
		let response_bytes = response.into_bytes();

		// Ensure the length doesn't exceed the maximum length.
		let bounded_response = match BoundedVec::try_from(response_bytes) {
			Ok(bounded) => bounded,
			Err(_) => {
				log::warn!("Response is too long. It has been ignored.");
				return;
			},
		};
		
		// Get the current list of responses.
		let mut responses = Responses::<T>::get();

		// Attempt to append the new response to the list.
		match responses.try_push((maybe_who.clone(), bounded_response.clone())) {
			Ok(_) => {
				// Update the storage.
				Responses::<T>::put(responses);
	
				// Emit an event that new response has been received.
				Self::deposit_event(Event::NewResponse { maybe_who, response: String::from_utf8(bounded_response.into()).unwrap() });
			},
			Err(_) => {
				log::warn!("Unable to add response. Maximum number of responses reached.");
			},
		}
		
	}
}