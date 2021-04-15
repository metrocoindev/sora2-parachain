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

use crate::mock::*;
use crate::{Error, Module};
use common::{balance, fixed};
use frame_support::assert_noop;
use traits::MultiCurrency;

type PswapDistrModule = Module<Runtime>;

#[test]
fn subscribe_with_default_frequency_should_pass() {
    let mut ext = ExtBuilder::uninitialized().build();
    ext.execute_with(|| {
        PswapDistrModule::subscribe(fees_account_a(), DEX_A_ID, PoolTokenAId::get(), None)
            .expect("Failed to subscribe account.");
        assert_eq!(
            PswapDistrModule::subscribed_accounts(fees_account_a()),
            Some((
                DEX_A_ID,
                PoolTokenAId::get(),
                GetDefaultSubscriptionFrequency::get(),
                0
            ))
        );
    })
}

#[test]
fn subscribe_with_zero_frequency_should_fail() {
    let mut ext = ExtBuilder::uninitialized().build();
    ext.execute_with(|| {
        assert_noop!(
            PswapDistrModule::subscribe(fees_account_a(), DEX_A_ID, PoolTokenAId::get(), Some(0)),
            Error::<Runtime>::InvalidFrequency
        );
    })
}

#[test]
fn subscribe_with_existing_account_should_fail() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        assert_noop!(
            PswapDistrModule::subscribe(fees_account_a(), DEX_A_ID, PoolTokenAId::get(), None),
            Error::<Runtime>::SubscriptionActive
        );
    })
}

#[test]
fn unsubscribe_with_inexistent_account_should_fail() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let result = PswapDistrModule::unsubscribe(alice());
        assert_noop!(result, Error::<Runtime>::UnknownSubscription);
    });
}

#[test]
fn distribute_existing_pswap_should_pass() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Failed to claim.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Failed to claim.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Failed to claim.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        assert_eq!(balance_a, balance!(2.7));
        assert_eq!(balance_b, balance!(1.8));
        assert_eq!(balance_c, balance!(0.9));
    })
}

#[test]
fn distribute_with_zero_balance_should_pass() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();
        PswapDistrModule::distribute_incentive(
            &fees_account_b(),
            &DEX_A_ID,
            &PoolTokenBId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");

        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c())),
            Error::<Runtime>::ZeroClaimableIncentives
        );

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        assert_eq!(balance_a, 0);
        assert_eq!(balance_b, 0);
        assert_eq!(balance_c, 0);
    })
}

#[test]
fn incentive_distribution_routine_should_pass() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let parliament =
            Tokens::free_balance(GetIncentiveAssetId::get(), &GetParliamentAccountId::get());
        assert_eq!(parliament, balance!(0));

        for i in 0u64..5 {
            PswapDistrModule::incentive_distribution_routine(i);
        }

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let parliament =
            Tokens::free_balance(GetIncentiveAssetId::get(), &GetParliamentAccountId::get());
        assert_eq!(balance_a, 0);
        assert_eq!(balance_b, 0);
        assert_eq!(balance_c, 0);
        assert_eq!(parliament, balance!(0.6));

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Failed to claim.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let parliament =
            Tokens::free_balance(GetIncentiveAssetId::get(), &GetParliamentAccountId::get());
        assert_eq!(balance_a, balance!(2.7));
        assert_eq!(balance_b, 0);
        assert_eq!(balance_c, 0);
        assert_eq!(parliament, balance!(0.6));

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Failed to claim.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let parliament =
            Tokens::free_balance(GetIncentiveAssetId::get(), &GetParliamentAccountId::get());
        assert_eq!(balance_a, balance!(2.7));
        assert_eq!(balance_b, balance!(1.8));
        assert_eq!(balance_c, 0);
        assert_eq!(parliament, balance!(0.6));

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Failed to claim.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let parliament =
            Tokens::free_balance(GetIncentiveAssetId::get(), &GetParliamentAccountId::get());
        assert_eq!(balance_a, balance!(2.7));
        assert_eq!(balance_b, balance!(1.8));
        assert_eq!(balance_c, balance!(0.9));
        assert_eq!(parliament, balance!(0.6));

        for i in 5u64..10 {
            PswapDistrModule::incentive_distribution_routine(i);
        }

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let parliament =
            Tokens::free_balance(GetIncentiveAssetId::get(), &GetParliamentAccountId::get());
        assert_eq!(balance_a, balance!(2.7));
        assert_eq!(balance_b, balance!(1.8));
        assert_eq!(balance_c, balance!(0.9));
        assert_eq!(parliament, balance!(0.6));

        let total = balance_a + balance_b + balance_c + parliament;
        assert_eq!(total, balance!(6));
        assert_eq!(total / parliament, 10);
    })
}

#[test]
fn increasing_burn_rate_should_pass() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        assert_eq!(PswapDistrModule::burn_rate(), fixed!(0.1));
        for i in 0u64..3 {
            PswapDistrModule::burn_rate_update_routine(i);
        }
        assert_eq!(PswapDistrModule::burn_rate(), fixed!(0.2));
        for i in 3u64..6 {
            PswapDistrModule::burn_rate_update_routine(i);
        }
        assert_eq!(PswapDistrModule::burn_rate(), fixed!(0.3));
        for i in 6u64..9 {
            PswapDistrModule::burn_rate_update_routine(i);
        }
        assert_eq!(PswapDistrModule::burn_rate(), fixed!(0.4));
        // Observe flatline
        for i in 9u64..12 {
            PswapDistrModule::burn_rate_update_routine(i);
        }
        assert_eq!(PswapDistrModule::burn_rate(), fixed!(0.4));
        for i in 9u64..1000 {
            PswapDistrModule::burn_rate_update_routine(i);
        }
        assert_eq!(PswapDistrModule::burn_rate(), fixed!(0.4));
    })
}

#[test]
fn claim_until_zero_should_pass() {
    let mut ext = ExtBuilder::with_accounts(vec![
        (liquidity_provider_a(), PoolTokenAId::get(), balance!(3)),
        (liquidity_provider_b(), PoolTokenAId::get(), balance!(2)),
        (liquidity_provider_c(), PoolTokenAId::get(), balance!(1)),
    ])
    .build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();

        // start with empty fees account, claiming should fail
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, 0);
        assert_eq!(balance_b, 0);
        assert_eq!(balance_c, 0);
        assert_eq!(balance_d, 0);

        // new pswap was derived from exchange, it should be claimable after distribution
        Assets::mint(
            Origin::signed(tech_account_id.clone()),
            GetIncentiveAssetId::get(),
            fees_account_a(),
            balance!(60),
        )
        .expect("Minting tokens is not expected to fail.");
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Claiming is not expected to fail.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(27));
        assert_eq!(balance_b, balance!(18));
        assert_eq!(balance_c, balance!(9));
        assert_eq!(balance_d, 0);

        // again period of no incentives, should return error for non claimable
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(27));
        assert_eq!(balance_b, balance!(18));
        assert_eq!(balance_c, balance!(9));
        assert_eq!(balance_d, 0);

        // new pswap was derived from exchange, it should be claimable after distribution, now only one account claims it
        Assets::mint(
            Origin::signed(tech_account_id.clone()),
            GetIncentiveAssetId::get(),
            fees_account_a(),
            balance!(600),
        )
        .expect("Minting tokens is not expected to fail.");
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Claiming is not expected to fail.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(27));
        assert_eq!(balance_b, balance!(198));
        assert_eq!(balance_c, balance!(9));
        assert_eq!(balance_d, balance!(360));

        // final pswap arrival, should be consistent for previously claimed and unclaimed
        Assets::mint(
            Origin::signed(tech_account_id.clone()),
            GetIncentiveAssetId::get(),
            fees_account_a(),
            balance!(6000),
        )
        .expect("Minting tokens is not expected to fail.");
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Claiming is not expected to fail.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(2996.999999999999990010));
        assert_eq!(balance_b, balance!(1998.000000000000008999));
        assert_eq!(balance_c, balance!(999.000000000000000991));
        assert_eq!(balance_d, 0);
        assert_eq!(
            balance_a + balance_b + balance_c + balance_d,
            balance!(5994)
        );
    })
}

#[test]
fn external_transfer_to_tech_account_after_distribution() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();

        // initial distribution happens normally
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");

        let balance_tech = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_tech, balance!(5.4));

        // before clre claimable value will be increased
        Assets::mint(
            Origin::signed(tech_account_id.clone()),
            GetIncentiveAssetId::get(),
            tech_account_id.clone(),
            balance!(11111.111111111111111111),
        )
        .expect("Minting tokens is not expected to fail.");

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Failed to claim.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Failed to claim.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Failed to claim.");

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_tech = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        // externally added incentive is evenly distributed amoung current unclaimed balances
        assert_eq!(balance_a, balance!(5558.255555555561844801));
        assert_eq!(balance_b, balance!(3705.503703703700268335));
        assert_eq!(balance_c, balance!(1852.751851851848997975));
        assert_eq!(
            balance_a + balance_b + balance_c,
            balance!(11116.511111111111111111)
        );
        assert_eq!(balance_tech, 0);
    })
}

#[test]
fn jump_start_with_unowned_incentive_should_pass() {
    let mut ext = ExtBuilder::with_accounts(vec![
        (fees_account_a(), common::PSWAP.into(), balance!(6)),
        (liquidity_provider_a(), PoolTokenAId::get(), balance!(3)),
        (liquidity_provider_b(), PoolTokenAId::get(), balance!(2)),
        (liquidity_provider_c(), PoolTokenAId::get(), balance!(1)),
    ])
    .build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();

        // initially no liquidity providers have received incentives yet, thus shares are not calculated for them yet,
        // however some incentive is transferred to claimable reserve
        Assets::mint(
            Origin::signed(tech_account_id.clone()),
            GetIncentiveAssetId::get(),
            tech_account_id.clone(),
            balance!(11111.111111111111111111),
        )
        .expect("Minting tokens is not expected to fail.");

        // no one can claim it as shares are not calculated for this transfer
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b())),
            Error::<Runtime>::ZeroClaimableIncentives
        );
        assert_noop!(
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c())),
            Error::<Runtime>::ZeroClaimableIncentives
        );

        // now liquidity providers receive their incentive, and claim it
        PswapDistrModule::distribute_incentive(
            &fees_account_a(),
            &DEX_A_ID,
            &PoolTokenAId::get(),
            &tech_account_id,
        )
        .expect("Error is not expected during distribution");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Failed to claim.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Failed to claim.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Failed to claim.");
        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);

        // one of claimers collects unowned incentive, special correction is applied so precision loss is avoided on following claims
        assert_eq!(balance_a, balance!(2.70000000000000000));
        assert_eq!(balance_b, balance!(1.800000000000000000));
        assert_eq!(balance_c, balance!(11112.011111111111111111));

        assert_eq!(balance_d, balance!(0.000000000000000000));
    })
}

#[test]
fn increasing_volumes_should_pass() {
    let mut ext = ExtBuilder::with_accounts(vec![
        (liquidity_provider_a(), PoolTokenAId::get(), balance!(3)),
        (liquidity_provider_b(), PoolTokenAId::get(), balance!(2)),
        (liquidity_provider_c(), PoolTokenAId::get(), balance!(1)),
    ])
    .build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();

        let mut decimals_factor = 1;

        for _ in 0..=27u32 {
            Assets::mint(
                Origin::signed(tech_account_id.clone()),
                GetIncentiveAssetId::get(),
                fees_account_a(),
                10 * decimals_factor,
            )
            .expect("Minting tokens is not expected to fail.");
            PswapDistrModule::distribute_incentive(
                &fees_account_a(),
                &DEX_A_ID,
                &PoolTokenAId::get(),
                &tech_account_id,
            )
            .expect("Error is not expected during distribution");
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
                .expect("Claiming is not expected to fail.");
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
                .expect("Claiming is not expected to fail.");
            PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
                .expect("Claiming is not expected to fail.");
            decimals_factor *= 10;
        }

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(5000000000.000000000004504507));
        assert_eq!(balance_b, balance!(3333333333.333333333336336335));
        assert_eq!(balance_c, balance!(1666666666.666666666659159157));
        assert_eq!(
            balance_a + balance_b + balance_c,
            balance!(9999999999.999999999999999999)
        );
        assert_eq!(balance_d, 0);
    })
}

#[test]
fn multiple_pools_should_pass() {
    let mut ext = ExtBuilder::with_accounts(vec![
        (fees_account_a(), common::PSWAP.into(), balance!(20)),
        (fees_account_b(), common::PSWAP.into(), balance!(2)),
        (liquidity_provider_a(), PoolTokenAId::get(), balance!(1)),
        (liquidity_provider_b(), PoolTokenBId::get(), balance!(5)),
        (liquidity_provider_c(), PoolTokenBId::get(), balance!(5)),
    ])
    .build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();

        for i in 0u64..5 {
            PswapDistrModule::incentive_distribution_routine(i);
        }

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Claiming is not expected to fail.");

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(18.000000000000000017));
        assert_eq!(balance_b, balance!(0.899999999999999991));
        assert_eq!(balance_c, balance!(0.899999999999999992));
        assert_eq!(balance_d, 0);
        assert_eq!(
            balance_a + balance_b + balance_c + balance_c,
            balance!(20.699999999999999992)
        )
    })
}

#[test]
fn mixed_multiple_pools_should_pass() {
    let mut ext = ExtBuilder::with_accounts(vec![
        (fees_account_a(), common::PSWAP.into(), balance!(20)),
        (fees_account_b(), common::PSWAP.into(), balance!(4)),
        (liquidity_provider_a(), PoolTokenAId::get(), balance!(1)),
        (liquidity_provider_a(), PoolTokenBId::get(), balance!(5)),
        (liquidity_provider_b(), PoolTokenBId::get(), balance!(5)),
        (liquidity_provider_c(), PoolTokenAId::get(), balance!(1)),
        (liquidity_provider_c(), PoolTokenBId::get(), balance!(10)),
    ])
    .build();
    ext.execute_with(|| {
        let tech_account_id = GetPswapDistributionAccountId::get();

        for i in 0u64..5 {
            PswapDistrModule::incentive_distribution_routine(i);
        }

        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_a()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_b()))
            .expect("Claiming is not expected to fail.");
        PswapDistrModule::claim_incentive(Origin::signed(liquidity_provider_c()))
            .expect("Claiming is not expected to fail.");

        let balance_a = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_a());
        let balance_b = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_b());
        let balance_c = Tokens::free_balance(GetIncentiveAssetId::get(), &liquidity_provider_c());
        let balance_d = Tokens::free_balance(GetIncentiveAssetId::get(), &tech_account_id);
        assert_eq!(balance_a, balance!(9.900000000000000009)); // 9 from A, 0.9 from B
        assert_eq!(balance_b, balance!(0.899999999999999999)); // 0.9 from B
        assert_eq!(balance_c, balance!(10.799999999999999992)); // 9 from A, 1.8 from B
        assert_eq!(balance_d, 0);
        assert_eq!(
            balance_a + balance_b + balance_c + balance_d,
            balance!(21.6) // (initial) 24 - (parliament) 10%
        );
    })
}

#[test]
fn calculating_distribution_should_pass() {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| {
        // zero amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(balance!(0)).unwrap();
        assert_eq!(distribution.liquidity_providers, balance!(0));
        assert_eq!(distribution.vesting, balance!(0));
        assert_eq!(distribution.parliament, balance!(0));

        // indivisible small amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(1u128).unwrap();
        assert_eq!(
            distribution.liquidity_providers + distribution.vesting + distribution.parliament,
            1u128
        );

        // divisible small amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(100u128).unwrap();
        assert_eq!(distribution.liquidity_providers, 90u128);
        assert_eq!(distribution.vesting, 0u128);
        assert_eq!(distribution.parliament, 10u128);

        // regular amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(balance!(100)).unwrap();
        assert_eq!(distribution.liquidity_providers, balance!(90));
        assert_eq!(distribution.vesting, balance!(0));
        assert_eq!(distribution.parliament, balance!(10));

        for i in 0u64..6 {
            PswapDistrModule::burn_rate_update_routine(i);
        }
        // burn rate should increase to 0.3 after this

        // zero amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(balance!(0)).unwrap();
        assert_eq!(distribution.liquidity_providers, balance!(0));
        assert_eq!(distribution.vesting, balance!(0));
        assert_eq!(distribution.parliament, balance!(0));

        // indivisible small amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(1u128).unwrap();
        assert_eq!(
            distribution.liquidity_providers + distribution.vesting + distribution.parliament,
            1u128
        );

        // divisible small amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(100u128).unwrap();
        assert_eq!(distribution.liquidity_providers, 70u128);
        assert_eq!(distribution.vesting, 20u128);
        assert_eq!(distribution.parliament, 10u128);

        // regular amount
        let distribution = PswapDistrModule::calculate_pswap_distribution(balance!(100)).unwrap();
        assert_eq!(distribution.liquidity_providers, balance!(70));
        assert_eq!(distribution.vesting, balance!(20));
        assert_eq!(distribution.parliament, balance!(10));

        // large value, balance is limited to i128 max because of Fixed type calculation
        let balance_max = 170141183460469231731687303715884105727u128;
        let distribution = PswapDistrModule::calculate_pswap_distribution(balance_max).unwrap();
        assert_eq!(
            distribution.liquidity_providers,
            119098828422328462212181112601118874008u128
        );
        assert_eq!(
            distribution.vesting,
            34028236692093846346337460743176821147u128
        );
        assert_eq!(
            distribution.parliament,
            17014118346046923173168730371588410572u128
        );
        assert_eq!(
            distribution.liquidity_providers + distribution.parliament + distribution.vesting,
            balance_max
        );
    })
}
