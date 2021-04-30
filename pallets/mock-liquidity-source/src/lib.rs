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

use common::fixnum::ops::One;
use common::prelude::{FixedWrapper, SwapAmount, SwapOutcome};
use common::{balance, fixed, Balance, Fixed, GetPoolReserves, LiquiditySource};
use core::convert::TryInto;
use frame_support::dispatch::DispatchError;
use frame_support::ensure;
use frame_support::traits::Get;
use frame_system::ensure_signed;
use permissions::{Scope, BURN, MINT, TRANSFER};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[allow(non_snake_case)]
impl<T: Config<I>, I: 'static> Pallet<T, I> {
    fn initialize_reserves(reserves: &[(T::DEXId, T::AssetId, (Fixed, Fixed))]) {
        reserves
            .iter()
            .for_each(|(dex_id, target_asset_id, reserve_pair)| {
                <Reserves<T, I>>::insert(dex_id, target_asset_id, reserve_pair);
            })
    }

    fn get_base_amount_out(
        target_amount_in: Fixed,
        base_reserve: Fixed,
        target_reserve: Fixed,
    ) -> Result<SwapOutcome<Fixed>, DispatchError> {
        let zero = fixed!(0);
        ensure!(
            target_amount_in > zero,
            <Error<T, I>>::InsufficientInputAmount
        );
        ensure!(
            base_reserve > zero && target_reserve > zero,
            <Error<T, I>>::InsufficientLiquidity
        );
        let X: FixedWrapper = base_reserve.into();
        let Y: FixedWrapper = target_reserve.into();
        let d_Y: FixedWrapper = target_amount_in.into();

        let amount_out_without_fee = (d_Y.clone() * X / (Y + d_Y))
            .get()
            .map_err(|_| Error::<T, I>::InsufficientLiquidity)?;

        let fee_fraction: FixedWrapper = T::GetFee::get().into();
        let fee_amount = amount_out_without_fee * fee_fraction;
        Ok(SwapOutcome::new(
            (amount_out_without_fee - fee_amount.clone())
                .get()
                .map_err(|_| Error::<T, I>::CalculationError)?,
            fee_amount
                .get()
                .map_err(|_| Error::<T, I>::CalculationError)?,
        ))
    }

    fn get_target_amount_out(
        base_amount_in: Fixed,
        base_reserve: Fixed,
        target_reserve: Fixed,
    ) -> Result<SwapOutcome<Fixed>, DispatchError> {
        let zero = fixed!(0);
        ensure!(
            base_amount_in > zero,
            <Error<T, I>>::InsufficientInputAmount
        );
        ensure!(
            base_reserve > zero && target_reserve > zero,
            <Error<T, I>>::InsufficientLiquidity
        );
        let fee_fraction: FixedWrapper = T::GetFee::get().into();
        let fee_amount = base_amount_in * fee_fraction;
        let amount_in_with_fee = base_amount_in - fee_amount.clone();
        let X: FixedWrapper = base_reserve.into();
        let Y: FixedWrapper = target_reserve.into();
        let d_X: FixedWrapper = amount_in_with_fee.into();
        let amount_out = (Y * d_X.clone() / (X + d_X))
            .get()
            .map_err(|_| Error::<T, I>::InsufficientLiquidity)?;
        let fee_amount = fee_amount
            .get()
            .map_err(|_| Error::<T, I>::CalculationError)?;

        Ok(SwapOutcome::new(amount_out, fee_amount))
    }

    fn get_base_amount_in(
        target_amount_out: Fixed,
        base_reserve: Fixed,
        target_reserve: Fixed,
    ) -> Result<SwapOutcome<Fixed>, DispatchError> {
        let zero = fixed!(0);
        ensure!(
            target_amount_out > zero,
            <Error<T, I>>::InsufficientOutputAmount
        );
        ensure!(
            base_reserve > zero && target_reserve > zero,
            <Error<T, I>>::InsufficientLiquidity
        );

        let X: FixedWrapper = base_reserve.into();
        let Y: FixedWrapper = target_reserve.into();
        let d_Y: FixedWrapper = target_amount_out.into();

        let base_amount_in_without_fee = (X * d_Y.clone() / (Y - d_Y))
            .get()
            .map_err(|_| Error::<T, I>::InsufficientLiquidity)?;
        let fee_fraction: FixedWrapper = T::GetFee::get().into();
        let base_amount_in_with_fee = FixedWrapper::from(base_amount_in_without_fee)
            / (FixedWrapper::from(Fixed::ONE) - fee_fraction);
        let actual_target_amount_out = Self::get_target_amount_out(
            base_amount_in_with_fee
                .clone()
                .get()
                .map_err(|_| Error::<T, I>::CalculationError)?,
            base_reserve,
            target_reserve,
        )?
        .amount;
        let amount_in = if actual_target_amount_out < target_amount_out {
            base_amount_in_with_fee.clone() + Fixed::from_bits(1)
        } else {
            base_amount_in_with_fee.clone()
        };
        Ok(SwapOutcome::new(
            amount_in
                .get()
                .map_err(|_| Error::<T, I>::CalculationError)?,
            (base_amount_in_with_fee - base_amount_in_without_fee)
                .get()
                .map_err(|_| Error::<T, I>::CalculationError)?,
        ))
    }

    fn get_target_amount_in(
        base_amount_out: Fixed,
        base_reserve: Fixed,
        target_reserve: Fixed,
    ) -> Result<SwapOutcome<Fixed>, DispatchError> {
        let zero = fixed!(0);
        ensure!(
            base_amount_out > zero,
            <Error<T, I>>::InsufficientOutputAmount
        );
        ensure!(
            base_reserve > zero && target_reserve > zero,
            <Error<T, I>>::InsufficientLiquidity
        );

        let one: FixedWrapper = fixed!(1);
        let base_amount_out_wrapper: FixedWrapper = base_amount_out.into();
        let base_amount_out_with_fee = base_amount_out_wrapper / (one - T::GetFee::get());

        let X: FixedWrapper = base_reserve.into();
        let Y: FixedWrapper = target_reserve.into();
        let d_X = base_amount_out_with_fee.clone();

        let target_amount_in: Fixed = (Y * d_X.clone() / (X - d_X))
            .get()
            .map_err(|_| Error::<T, I>::InsufficientLiquidity)?;
        let actual_base_amount_out =
            Self::get_base_amount_out(target_amount_in, base_reserve, target_reserve)?.amount;

        let amount_in = if actual_base_amount_out < base_amount_out {
            target_amount_in + Fixed::from_bits(1).into()
        } else {
            target_amount_in.into()
        };
        let amount_in = amount_in
            .get()
            .map_err(|_| Error::<T, I>::CalculationError)?;
        let fee = (base_amount_out_with_fee - base_amount_out)
            .get()
            .map_err(|_| Error::<T, I>::CalculationError)?;
        Ok(SwapOutcome::new(amount_in, fee))
    }
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    pub fn set_reserves_account_id(account: T::TechAccountId) -> Result<(), DispatchError> {
        let account_id = technical::Pallet::<T>::tech_account_id_to_account_id(&account)?;
        frame_system::Pallet::<T>::inc_consumers(&account_id)
            .map_err(|_| Error::<T, I>::IncRefError)?;
        ReservesAcc::<T, I>::set(account.clone());
        let permissions = [BURN, MINT, TRANSFER];
        for permission in &permissions {
            permissions::Pallet::<T>::assign_permission(
                account_id.clone(),
                &account_id,
                *permission,
                Scope::Unlimited,
            )?;
        }
        Ok(())
    }
}

impl<T: Config<I>, I: 'static>
    LiquiditySource<T::DEXId, T::AccountId, T::AssetId, Balance, DispatchError> for Pallet<T, I>
{
    fn can_exchange(
        dex_id: &T::DEXId,
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
    ) -> bool {
        let base_asset_id = &T::GetBaseAssetId::get();
        if input_asset_id == base_asset_id {
            <Reserves<T, I>>::contains_key(dex_id, output_asset_id)
        } else if output_asset_id == base_asset_id {
            <Reserves<T, I>>::contains_key(dex_id, input_asset_id)
        } else {
            <Reserves<T, I>>::contains_key(dex_id, output_asset_id)
                && <Reserves<T, I>>::contains_key(dex_id, input_asset_id)
        }
    }

    fn quote(
        dex_id: &T::DEXId,
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
        swap_amount: SwapAmount<Balance>,
    ) -> Result<SwapOutcome<Balance>, DispatchError> {
        let swap_amount = match swap_amount {
            SwapAmount::WithDesiredOutput {
                desired_amount_out,
                max_amount_in,
            } => {
                if max_amount_in > Balance::MAX / 2 {
                    SwapAmount::with_desired_output(desired_amount_out, Balance::MAX / 2)
                } else {
                    swap_amount
                }
            }
            _ => swap_amount,
        };
        let swap_amount = swap_amount
            .try_into()
            .map_err(|_| Error::<T, I>::CalculationError)?;
        let base_asset_id = &T::GetBaseAssetId::get();
        if input_asset_id == base_asset_id {
            let (base_reserve, target_reserve) = <Reserves<T, I>>::get(dex_id, output_asset_id);
            Ok(match swap_amount {
                SwapAmount::WithDesiredInput {
                    desired_amount_in: base_amount_in,
                    ..
                } => Self::get_target_amount_out(base_amount_in, base_reserve, target_reserve)?
                    .try_into()
                    .map_err(|_| Error::<T, I>::CalculationError)?,
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: target_amount_out,
                    ..
                } => Self::get_base_amount_in(target_amount_out, base_reserve, target_reserve)?
                    .try_into()
                    .map_err(|_| Error::<T, I>::CalculationError)?,
            })
        } else if output_asset_id == base_asset_id {
            let (base_reserve, target_reserve) = <Reserves<T, I>>::get(dex_id, input_asset_id);
            Ok(match swap_amount {
                SwapAmount::WithDesiredInput {
                    desired_amount_in: target_amount_in,
                    ..
                } => Self::get_base_amount_out(target_amount_in, base_reserve, target_reserve)?
                    .try_into()
                    .map_err(|_| Error::<T, I>::CalculationError)?,
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: base_amount_out,
                    ..
                } => Self::get_target_amount_in(base_amount_out, base_reserve, target_reserve)?
                    .try_into()
                    .map_err(|_| Error::<T, I>::CalculationError)?,
            })
        } else {
            let (base_reserve_a, target_reserve_a) = <Reserves<T, I>>::get(dex_id, input_asset_id);
            let (base_reserve_b, target_reserve_b) = <Reserves<T, I>>::get(dex_id, output_asset_id);
            match swap_amount {
                SwapAmount::WithDesiredInput {
                    desired_amount_in, ..
                } => {
                    let outcome_a: SwapOutcome<Fixed> = Self::get_base_amount_out(
                        desired_amount_in,
                        base_reserve_a,
                        target_reserve_a,
                    )?;
                    let outcome_b: SwapOutcome<Fixed> = Self::get_target_amount_out(
                        outcome_a.amount,
                        base_reserve_b,
                        target_reserve_b,
                    )?;
                    let outcome_a_fee: FixedWrapper = outcome_a.fee.into();
                    let outcome_b_fee: FixedWrapper = outcome_b.fee.into();
                    let amount = outcome_b
                        .amount
                        .into_bits()
                        .try_into()
                        .map_err(|_| Error::<T, I>::CalculationError)?;
                    let fee = (outcome_a_fee + outcome_b_fee)
                        .try_into_balance()
                        .map_err(|_| Error::<T, I>::CalculationError)?;
                    Ok(SwapOutcome::new(amount, fee))
                }
                SwapAmount::WithDesiredOutput {
                    desired_amount_out, ..
                } => {
                    let outcome_b: SwapOutcome<Fixed> = Self::get_base_amount_in(
                        desired_amount_out,
                        base_reserve_b,
                        target_reserve_b,
                    )?;
                    let outcome_a: SwapOutcome<Fixed> = Self::get_target_amount_in(
                        outcome_b.amount,
                        base_reserve_a,
                        target_reserve_a,
                    )?;
                    let outcome_a_fee: FixedWrapper = outcome_a.fee.into();
                    let outcome_b_fee: FixedWrapper = outcome_b.fee.into();
                    let amount = outcome_a
                        .amount
                        .into_bits()
                        .try_into()
                        .map_err(|_| Error::<T, I>::CalculationError)?;
                    let fee = (outcome_b_fee + outcome_a_fee)
                        .try_into_balance()
                        .map_err(|_| Error::<T, I>::CalculationError)?;
                    Ok(SwapOutcome::new(amount, fee))
                }
            }
        }
    }

    fn exchange(
        _sender: &T::AccountId,
        _receiver: &T::AccountId,
        dex_id: &T::DEXId,
        input_asset_id: &T::AssetId,
        output_asset_id: &T::AssetId,
        desired_amount: SwapAmount<Balance>,
    ) -> Result<SwapOutcome<Balance>, DispatchError> {
        // actual exchange does not happen
        Self::quote(dex_id, input_asset_id, output_asset_id, desired_amount)
    }
}

impl<T: Config<I>, I: 'static> GetPoolReserves<T::AssetId> for Pallet<T, I> {
    fn reserves(_base_asset: &T::AssetId, other_asset: &T::AssetId) -> (Balance, Balance) {
        // This will only work for the dex_id being common::DEXId::Polkaswap
        // Letting the dex_id being passed as a parameter by the caller would require changing
        // the trait interface which is not desirable
        let dex_id: T::DEXId = common::DEXId::Polkaswap.into();
        let (base_reserve, target_reserve) = <Reserves<T, I>>::get(dex_id, other_asset);
        (
            base_reserve.into_bits().try_into().unwrap_or(balance!(0)),
            target_reserve.into_bits().try_into().unwrap_or(balance!(0)),
        )
    }
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use common::{EnsureDEXManager, EnsureTradingPairExists, ManagementMode};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config<I: 'static = ()>:
        frame_system::Config + common::Config + assets::Config + technical::Config
    {
        type GetFee: Get<Fixed>;
        type EnsureDEXManager: EnsureDEXManager<Self::DEXId, Self::AccountId, DispatchError>;
        type EnsureTradingPairExists: EnsureTradingPairExists<
            Self::DEXId,
            Self::AssetId,
            DispatchError,
        >;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        // example, this checks should be called at the beginning of management functions of actual liquidity sources, e.g. register, set_fee
        #[pallet::weight(0)]
        pub fn test_access(
            origin: OriginFor<T>,
            dex_id: T::DEXId,
            target_id: T::AssetId,
        ) -> DispatchResultWithPostInfo {
            let _who =
                T::EnsureDEXManager::ensure_can_manage(&dex_id, origin, ManagementMode::Public)?;
            T::EnsureTradingPairExists::ensure_trading_pair_exists(
                &dex_id,
                &T::GetBaseAssetId::get(),
                &target_id,
            )?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn set_reserve(
            origin: OriginFor<T>,
            dex_id: T::DEXId,
            target_id: T::AssetId,
            base_reserve: Fixed,
            target_reserve: Fixed,
        ) -> DispatchResultWithPostInfo {
            let _who = ensure_signed(origin)?;
            <Reserves<T, I>>::insert(dex_id, target_id, (base_reserve, target_reserve));
            Ok(().into())
        }
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
        PairDoesNotExist,
        InsufficientInputAmount,
        InsufficientOutputAmount,
        InsufficientLiquidity,
        /// Specified parameters lead to arithmetic error
        CalculationError,
        /// Increment account reference error.
        IncRefError,
    }

    #[pallet::storage]
    #[pallet::getter(fn reserves)]
    pub type Reserves<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::DEXId,
        Blake2_128Concat,
        T::AssetId,
        (Fixed, Fixed),
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn reserves_account_id)]
    pub type ReservesAcc<T: Config<I>, I: 'static = ()> =
        StorageValue<_, T::TechAccountId, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
        pub phantom: sp_std::marker::PhantomData<I>,
        pub reserves: Vec<(T::DEXId, T::AssetId, (Fixed, Fixed))>,
    }

    #[cfg(feature = "std")]
    impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
        fn default() -> Self {
            Self {
                phantom: Default::default(),
                reserves: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
        fn build(&self) {
            Pallet::<T, I>::initialize_reserves(&self.reserves)
        }
    }
}
