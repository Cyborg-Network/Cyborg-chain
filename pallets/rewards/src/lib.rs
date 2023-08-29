//! Pallet Rewards v1
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
//! Pallet Rewards is a pallet that offers foundational incentives to providers for maintaining 
//! consistent network connectivity.
//! Rewards are recalculated hourly in the database based on random connectivity assessments by 
//! the edge connect pallet,
//! with daily payout distribution.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * - `check_provider_eligibility` - Check if a provider is eligible to receive a reward.
//! * - `reward_provider` - Reward a provider for providing a service or completing a task.
 
#![cfg_attr(not(feature = "std"), no_std)]

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
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

    #[pallet::storage]
	#[pallet::getter(fn providers)]
	pub type Providers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn rewards)]
    pub type Rewards<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A provider has been rewarded.
        ProviderRewarded(T::AccountId, u32),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The provider is not connected.
        NotConnected,
        /// The provider is not eligible to receive a reward.
        NotEligible,
        /// The provider has already been rewarded for this period.
        AlreadyRewarded,
        /// The reward amount is too low.
        RewardTooLow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Check if a provider is eligible to receive a reward.
        #[pallet::call_index(0)]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
        pub fn check_provider_eligibility(
            origin: OriginFor<T>,
            provider: T::AccountId,
            connection: bool,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Check if the provider is connected in the last hour.
            ensure!(connection, Error::<T>::NotConnected);

            // Check if the provider is already eligible to receive a reward.
            ensure!(!Providers::<T>::contains_key(&provider), Error::<T>::AlreadyRewarded);

            // Update storage.
            Providers::<T>::insert(&provider, who);

            Ok(())
        }

        /// Reward a provider for providing a service or completing a task.
        #[pallet::call_index(1)]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
        pub fn reward_provider(
            origin: OriginFor<T>,
            provider: T::AccountId,
            reward: u32,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Check if the provider is eligible to receive a reward.
            if Providers::<T>::contains_key(&provider) {
                return Err(Error::<T>::NotEligible.into());
            }

            // Check if the reward amount is too low.
            if reward < 1 {
                return Err(Error::<T>::RewardTooLow.into());
            }

            // Reward the provider.
            Rewards::<T>::insert(&provider, reward);

            // Emit an event.
            Self::deposit_event(Event::ProviderRewarded(provider, reward));

            Ok(())
        }
    }
}