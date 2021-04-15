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

//! Multicollateral bonding curve pool module benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use codec::Decode;
use frame_benchmarking::benchmarks;
use frame_system::{EventRecord, RawOrigin};
use hex_literal::hex;
use orml_traits::MultiCurrencyExtended;
use sp_std::prelude::*;

use common::{fixed, AssetName, AssetSymbol, USDT, XOR};

use crate::Pallet as MBCPool;
use assets::Pallet as Assets;
use permissions::Pallet as Permissions;
use sp_std::convert::TryFrom;
use tokens::Pallet as Tokens;
use trading_pair::Pallet as TradingPair;

// Support Functions
fn alice<T: Config>() -> T::AccountId {
    let bytes = hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");
    T::AccountId::decode(&mut &bytes[..]).unwrap_or_default()
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Module::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

benchmarks! {
    initialize_pool {
        let caller = alice::<T>();
        let dex_id: T::DEXId = common::DEXId::Polkaswap.into();
        Permissions::<T>::assign_permission(
            caller.clone(),
            &caller,
            permissions::MANAGE_DEX,
            permissions::Scope::Limited(common::hash(&dex_id)),
        ).unwrap();
        let _ = Assets::<T>::register_asset_id(caller.clone(), USDT.into(), AssetSymbol(b"TESTUSD".to_vec()), AssetName(b"USD".to_vec()), 18, Balance::zero(), true);
        TradingPair::<T>::register(RawOrigin::Signed(caller.clone()).into(), common::DEXId::Polkaswap.into(), XOR.into(), USDT.into()).unwrap();
    }: _(
        RawOrigin::Signed(caller.clone()),
        USDT.into()
    )
    verify {
        assert_last_event::<T>(Event::PoolInitialized(common::DEXId::Polkaswap.into(), USDT.into()).into())
    }

    set_reference_asset {
        let caller = alice::<T>();
        let dex_id: T::DEXId = common::DEXId::Polkaswap.into();
        Permissions::<T>::assign_permission(
            caller.clone(),
            &caller,
            permissions::MANAGE_DEX,
            permissions::Scope::Limited(common::hash(&dex_id)),
        ).unwrap();
        let _ = Assets::<T>::register_asset_id(caller.clone(), USDT.into(), AssetSymbol(b"TESTUSD".to_vec()), AssetName(b"USD".to_vec()), 18, Balance::zero(), true);
    }: _(
        RawOrigin::Signed(caller.clone()),
        USDT.into()
    )
    verify {
        assert_last_event::<T>(Event::ReferenceAssetChanged(USDT.into()).into())
    }

    set_optional_reward_multiplier {
        let caller = alice::<T>();
        let dex_id: T::DEXId = common::DEXId::Polkaswap.into();
        Permissions::<T>::assign_permission(
            caller.clone(),
            &caller,
            permissions::MANAGE_DEX,
            permissions::Scope::Limited(common::hash(&dex_id)),
        ).unwrap();
        let _ = Assets::<T>::register_asset_id(caller.clone(), USDT.into(), AssetSymbol(b"TESTUSD".to_vec()), AssetName(b"USD".to_vec()), 18, Balance::zero(), true);
        TradingPair::<T>::register(RawOrigin::Signed(caller.clone()).into(), common::DEXId::Polkaswap.into(), XOR.into(), USDT.into()).unwrap();
        MBCPool::<T>::initialize_pool(RawOrigin::Signed(caller.clone()).into(), USDT.into()).unwrap();
    }: _(
        RawOrigin::Signed(caller.clone()),
        USDT.into(),
        Some(fixed!(123))
    )
    verify {
        assert_last_event::<T>(Event::OptionalRewardMultiplierUpdated(USDT.into(), Some(fixed!(123))).into())
    }

    claim_incentives {
        let caller = alice::<T>();
        Rewards::<T>::insert(caller.clone(), (balance!(100), balance!(200)));
        TotalRewards::<T>::put(balance!(200));
        let pswap_asset_id: T::AssetId = PSWAP.into();
        let pswap_currency = <T::AssetId as Into<<T as tokens::Config>::CurrencyId>>::into(pswap_asset_id);
        let pswap_amount = <T as tokens::Config>::Amount::try_from(balance!(500)).map_err(|_|()).unwrap();
        Tokens::<T>::update_balance(pswap_currency, &IncentivesAccountId::<T>::get(), pswap_amount).unwrap();
    }: _(
        RawOrigin::Signed(caller.clone())
    )
    verify {
        let (limit, owned) = Rewards::<T>::get(caller.clone());
        assert_eq!(limit, balance!(0));
        assert_eq!(owned, balance!(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Runtime};
    use frame_support::assert_ok;

    #[test]
    #[ignore]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_initialize_pool::<Runtime>());
            assert_ok!(test_benchmark_set_reference_asset::<Runtime>());
            assert_ok!(test_benchmark_set_optional_reward_multiplier::<Runtime>());
            assert_ok!(test_benchmark_claim_incentives::<Runtime>());
        });
    }
}
