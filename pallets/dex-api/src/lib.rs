// This file is part of the SORA network and Polkaswap app.

// Copyright (c) 2020, 2021, Polka Biome Ltd. All rights reserved.
// SPDX-License-Identifier: BSD-4-Clause

// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:

// Redistributions of source code must retain the above copyright notice, this list
// of conditions and the following disclaimer.
// Redistributions in binary form must reproduce the above copyright notice, this
// list of conditions and the following disclaimer in the documentation and/or other
// materials provided with the distribution.
//
// All advertising materials mentioning features or use of this software must display
// the following acknowledgement: This product includes software developed by Polka Biome
// Ltd., SORA, and Polkaswap.
//
// Neither the name of the Polka Biome Ltd. nor the names of its contributors may be used
// to endorse or promote products derived from this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY Polka Biome Ltd. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL Polka Biome Ltd. BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
// STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

#![cfg_attr(not(feature = "std"), no_std)]

use common::prelude::{Balance, SwapAmount, SwapOutcome, SwapVariant};
use common::{
    LiquidityRegistry, LiquiditySource, LiquiditySourceFilter, LiquiditySourceId,
    LiquiditySourceType,
};
use frame_support::sp_runtime::DispatchError;
use frame_support::weights::Weight;
use frame_system::ensure_signed;
use sp_std::vec::Vec;

pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait WeightInfo {
    fn swap() -> Weight;
}

type DEXManager<T> = dex_manager::Pallet<T>;

impl<T: Config>
    LiquiditySource<
        LiquiditySourceId<T::DEXId, LiquiditySourceType>,
        T::AccountId,
        T::AssetId,
        Balance,
        DispatchError,
    > for Pallet<T>
{
    fn can_exchange(
        liquidity_source_id: &LiquiditySourceId<T::DEXId, LiquiditySourceType>,
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
    ) -> bool {
        use LiquiditySourceType::*;
        macro_rules! can_exchange {
            ($source_type:ident) => {
                T::$source_type::can_exchange(
                    &liquidity_source_id.dex_id,
                    input_asset_id,
                    output_asset_id,
                )
            };
        }
        match liquidity_source_id.liquidity_source_index {
            XYKPool => can_exchange!(XYKPool),
            BondingCurvePool => can_exchange!(BondingCurvePool),
            MulticollateralBondingCurvePool => can_exchange!(MulticollateralBondingCurvePool),
            MockPool => can_exchange!(MockLiquiditySource),
            MockPool2 => can_exchange!(MockLiquiditySource2),
            MockPool3 => can_exchange!(MockLiquiditySource3),
            MockPool4 => can_exchange!(MockLiquiditySource4),
        }
    }

    fn quote(
        liquidity_source_id: &LiquiditySourceId<T::DEXId, LiquiditySourceType>,
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
        swap_amount: SwapAmount<Balance>,
    ) -> Result<SwapOutcome<Balance>, DispatchError> {
        use LiquiditySourceType::*;
        macro_rules! quote {
            ($source_type:ident) => {
                T::$source_type::quote(
                    &liquidity_source_id.dex_id,
                    input_asset_id,
                    output_asset_id,
                    swap_amount,
                )
            };
        }
        match liquidity_source_id.liquidity_source_index {
            LiquiditySourceType::XYKPool => quote!(XYKPool),
            BondingCurvePool => quote!(BondingCurvePool),
            MulticollateralBondingCurvePool => quote!(MulticollateralBondingCurvePool),
            MockPool => quote!(MockLiquiditySource),
            MockPool2 => quote!(MockLiquiditySource2),
            MockPool3 => quote!(MockLiquiditySource3),
            MockPool4 => quote!(MockLiquiditySource4),
        }
    }

    fn exchange(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        liquidity_source_id: &LiquiditySourceId<T::DEXId, LiquiditySourceType>,
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
        swap_amount: SwapAmount<Balance>,
    ) -> Result<SwapOutcome<Balance>, DispatchError> {
        use LiquiditySourceType::*;
        macro_rules! exchange {
            ($source_type:ident) => {
                T::$source_type::exchange(
                    sender,
                    receiver,
                    &liquidity_source_id.dex_id,
                    input_asset_id,
                    output_asset_id,
                    swap_amount,
                )
            };
        }
        match liquidity_source_id.liquidity_source_index {
            XYKPool => exchange!(XYKPool),
            BondingCurvePool => exchange!(BondingCurvePool),
            MulticollateralBondingCurvePool => exchange!(MulticollateralBondingCurvePool),
            MockPool => exchange!(MockLiquiditySource),
            MockPool2 => exchange!(MockLiquiditySource2),
            MockPool3 => exchange!(MockLiquiditySource3),
            MockPool4 => exchange!(MockLiquiditySource4),
        }
    }
}

impl<T: Config> Pallet<T> {
    /// List liquidity source types which are enabled on chain, this applies to all DEX'es.
    /// Used in aggregation pallets, such as liquidity-proxy.
    pub fn get_supported_types() -> Vec<LiquiditySourceType> {
        EnabledSourceTypes::<T>::get()
    }
}

impl<T: Config>
    LiquidityRegistry<
        T::DEXId,
        T::AccountId,
        T::AssetId,
        LiquiditySourceType,
        Balance,
        DispatchError,
    > for Pallet<T>
{
    fn list_liquidity_sources(
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
        filter: LiquiditySourceFilter<T::DEXId, LiquiditySourceType>,
    ) -> Result<Vec<LiquiditySourceId<T::DEXId, LiquiditySourceType>>, DispatchError> {
        let supported_types = Self::get_supported_types();
        DEXManager::<T>::ensure_dex_exists(&filter.dex_id)?;
        Ok(supported_types
            .iter()
            .filter_map(|source_type| {
                if filter.matches_index(*source_type)
                    && Self::can_exchange(
                        &LiquiditySourceId::new(filter.dex_id, *source_type),
                        input_asset_id,
                        output_asset_id,
                    )
                {
                    Some(LiquiditySourceId::new(
                        filter.dex_id.clone(),
                        source_type.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect())
    }
}
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use assets::AssetIdOf;
    use common::{AccountIdOf, DexIdOf};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + common::Config + dex_manager::Config + trading_pair::Config
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type MockLiquiditySource: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        type MockLiquiditySource2: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        type MockLiquiditySource3: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        type MockLiquiditySource4: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        type BondingCurvePool: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        type MulticollateralBondingCurvePool: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        type XYKPool: LiquiditySource<
            Self::DEXId,
            Self::AccountId,
            Self::AssetId,
            Balance,
            DispatchError,
        >;
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Perform swap with specified parameters. Gateway for invoking liquidity source exchanges.
        ///
        /// - `dex_id`: ID of the exchange.
        /// - `liquidity_source_type`: Type of liquidity source to perform swap on.
        /// - `input_asset_id`: ID of Asset to be deposited from sender account into pool reserves.
        /// - `output_asset_id`: ID of Asset t0 be withdrawn from pool reserves into receiver account.
        /// - `amount`: Either amount of desired input or output tokens, determined by `swap_variant` parameter.
        /// - `limit`: Either maximum input amount or minimum output amount tolerated for successful swap,
        ///            determined by `swap_variant` parameter.
        /// - `swap_variant`: Either 'WithDesiredInput' or 'WithDesiredOutput', indicates amounts purpose.
        /// - `receiver`: Optional value, indicates AccountId for swap receiver. If not set, default is `sender`.
        #[pallet::weight(<T as Config>::WeightInfo::swap())]
        pub fn swap(
            origin: OriginFor<T>,
            dex_id: T::DEXId,
            liquidity_source_type: LiquiditySourceType,
            input_asset_id: T::AssetId,
            output_asset_id: T::AssetId,
            amount: Balance,
            limit: Balance,
            swap_variant: SwapVariant,
            receiver: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let receiver = receiver.unwrap_or(sender.clone());
            let outcome = Self::exchange(
                &sender,
                &receiver,
                &LiquiditySourceId::<T::DEXId, LiquiditySourceType>::new(
                    dex_id.clone(),
                    liquidity_source_type.clone(),
                ),
                &input_asset_id,
                &output_asset_id,
                SwapAmount::with_variant(swap_variant, amount.clone(), limit.clone()),
            )?;
            let (input_amount, output_amount) = match swap_variant {
                SwapVariant::WithDesiredInput => (amount, outcome.amount.clone()),
                SwapVariant::WithDesiredOutput => (outcome.amount.clone(), amount),
            };
            Self::deposit_event(Event::DirectExchange(
                sender,
                receiver,
                dex_id,
                liquidity_source_type,
                input_asset_id,
                output_asset_id,
                input_amount,
                output_amount,
                outcome.fee.clone(),
            ));
            Ok(().into())
        }
    }

    #[pallet::event]
    #[pallet::metadata(AccountIdOf<T> = "AccountId", AssetIdOf<T> = "AssetId", DexIdOf<T> = "DEXId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Exchange of tokens has been performed
        /// [Sender Account, Receiver Account, DEX Id, LiquiditySourceType, Input Asset Id, Output Asset Id, Input Amount, Output Amount, Fee Amount]
        DirectExchange(
            AccountIdOf<T>,
            AccountIdOf<T>,
            DexIdOf<T>,
            LiquiditySourceType,
            AssetIdOf<T>,
            AssetIdOf<T>,
            Balance,
            Balance,
            Balance,
        ),
    }

    #[pallet::storage]
    pub type EnabledSourceTypes<T: Config> = StorageValue<_, Vec<LiquiditySourceType>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub source_types: Vec<LiquiditySourceType>,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {
                source_types: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            EnabledSourceTypes::<T>::put(&self.source_types);
        }
    }
}
