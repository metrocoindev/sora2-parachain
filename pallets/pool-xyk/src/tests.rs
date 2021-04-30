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

use common::prelude::{SwapAmount, SwapOutcome};
use common::{
    balance, AssetName, AssetSymbol, Balance, LiquiditySource, LiquiditySourceType, ToFeeAccount,
};
use frame_support::{assert_noop, assert_ok};

use crate::mock::*;

use sp_std::rc::Rc;

type PresetFunction<'a> = Rc<
    dyn Fn(
            crate::mock::DEXId,
            AssetId,
            AssetId,
            common::TradingPair<crate::mock::TechAssetId>,
            crate::mock::TechAccountId,
            crate::mock::TechAccountId,
            AccountId,
            AccountId,
        ) -> ()
        + 'a,
>;

#[derive(Clone)]
struct RunTestsWithSlippageBehaviors<'a> {
    initial_deposit: (Balance, Balance),
    desired_amount: Balance,
    tests: Vec<PresetFunction<'a>>,
}

impl<'a> crate::Module<Runtime> {
    fn preset_initial(tests: Vec<PresetFunction<'a>>) {
        let mut ext = ExtBuilder::default().build();
        let dex_id = DEX_A_ID;
        let gt: crate::mock::AssetId = GoldenTicket.into();
        let bp: crate::mock::AssetId = BlackPepper.into();

        ext.execute_with(|| {
            assert_ok!(assets::Pallet::<Runtime>::register_asset_id(
                ALICE(),
                GoldenTicket.into(),
                AssetSymbol(b"GT".to_vec()),
                AssetName(b"Golden Ticket".to_vec()),
                18,
                Balance::from(0u32),
                true,
            ));

            assert_ok!(assets::Pallet::<Runtime>::register_asset_id(
                ALICE(),
                BlackPepper.into(),
                AssetSymbol(b"BP".to_vec()),
                AssetName(b"Black Pepper".to_vec()),
                18,
                Balance::from(0u32),
                true,
            ));

            assert_ok!(trading_pair::Pallet::<Runtime>::register(
                Origin::signed(BOB()),
                dex_id.clone(),
                GoldenTicket.into(),
                BlackPepper.into()
            ));

            assert_ok!(crate::Module::<Runtime>::initialize_pool(
                Origin::signed(BOB()),
                dex_id.clone(),
                GoldenTicket.into(),
                BlackPepper.into(),
            ));

            assert!(
                trading_pair::Pallet::<Runtime>::is_source_enabled_for_trading_pair(
                    &dex_id,
                    &GoldenTicket.into(),
                    &BlackPepper.into(),
                    LiquiditySourceType::XYKPool,
                )
                .expect("Failed to query trading pair status.")
            );

            let (tpair, tech_acc_id) =
                crate::Module::<Runtime>::tech_account_from_dex_and_asset_pair(
                    dex_id.clone(),
                    GoldenTicket.into(),
                    BlackPepper.into(),
                )
                .unwrap();

            let fee_acc = tech_acc_id.clone().to_fee_account().unwrap();
            let repr: AccountId =
                technical::Pallet::<Runtime>::tech_account_id_to_account_id(&tech_acc_id).unwrap();
            let fee_repr: AccountId =
                technical::Pallet::<Runtime>::tech_account_id_to_account_id(&fee_acc).unwrap();

            assert_ok!(assets::Pallet::<Runtime>::mint_to(
                &gt,
                &ALICE(),
                &ALICE(),
                balance!(900000)
            ));

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                balance!(900000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                balance!(2000000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &repr.clone()).unwrap(),
                0
            );

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &repr.clone()).unwrap(),
                0
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &fee_repr.clone()).unwrap(),
                0
            );

            let base_asset: AssetId = GoldenTicket.into();
            let target_asset: AssetId = BlackPepper.into();
            let tech_asset: AssetId = crate::Module::<Runtime>::get_marking_asset(&tech_acc_id)
                .expect("Failed to get marking asset")
                .into();
            assert_eq!(
                crate::Module::<Runtime>::properties(base_asset, target_asset),
                Some((repr.clone(), fee_repr.clone(), tech_asset))
            );
            /*
            assert_eq!(
                pswap_distribution::Module::<Runtime>::subscribed_accounts(&fee_repr),
                Some((
                    dex_id.clone(),
                    tech_asset,
                    GetDefaultSubscriptionFrequency::get(),
                    0
                ))
            );
            */

            for test in &tests {
                test(
                    dex_id.clone(),
                    gt.clone(),
                    bp.clone(),
                    tpair.clone(),
                    tech_acc_id.clone(),
                    fee_acc.clone(),
                    repr.clone(),
                    fee_repr.clone(),
                );
            }
        });
    }

    fn preset_deposited_pool(tests: Vec<PresetFunction<'a>>) {
        let mut new_tests: Vec<PresetFunction<'a>> = vec![Rc::new(
            |dex_id, _, _, _, tech_acc_id: crate::mock::TechAccountId, _, _, _| {
                assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
                    Origin::signed(ALICE()),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    balance!(360000),
                    balance!(144000),
                    balance!(360000),
                    balance!(144000),
                ));

                let tech_asset: AssetId = crate::Module::<Runtime>::get_marking_asset(&tech_acc_id)
                    .expect("Failed to get marking asset")
                    .into();
                assert_eq!(
                    assets::Pallet::<Runtime>::free_balance(&tech_asset, &ALICE()).unwrap(),
                    balance!(227683.9915321233119024),
                );
                //TODO: total supply check
            },
        )];
        let mut tests_to_add = tests.clone();
        new_tests.append(&mut tests_to_add);
        crate::Module::<Runtime>::preset_initial(new_tests);
    }

    fn preset_deposited_small(tests: Vec<PresetFunction<'a>>) {
        let mut new_tests: Vec<PresetFunction<'a>> =
            vec![Rc::new(|dex_id, _, _, _, _, _, _, _| {
                assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
                    Origin::signed(ALICE()),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    balance!(0.01),
                    balance!(0.01),
                    balance!(0),
                    balance!(0),
                ));
            })];
        let mut tests_to_add = tests.clone();
        new_tests.append(&mut tests_to_add);
        crate::Module::<Runtime>::preset_initial(new_tests);
    }

    fn run_tests_with_different_slippage_behavior(descriptor: RunTestsWithSlippageBehaviors<'a>) {
        let initial_deposit = descriptor.initial_deposit;
        let desired_amount = descriptor.desired_amount;
        let prepare: PresetFunction<'a> = Rc::new({
            move |dex_id, _, _, _, _, _, _, _| {
                assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
                    Origin::signed(ALICE()),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    initial_deposit.0,
                    initial_deposit.1,
                    initial_deposit.0,
                    initial_deposit.1,
                ));
            }
        });

        // List of cases for different slippage behavior.
        let cases: Vec<PresetFunction<'a>> = vec![
            Rc::new(move |dex_id, _, _, _, _, _, _, _| {
                assert_ok!(crate::Module::<Runtime>::swap_pair(
                    Origin::signed(ALICE()),
                    ALICE(),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    SwapAmount::WithDesiredOutput {
                        desired_amount_out: desired_amount,
                        max_amount_in: balance!(99999999),
                    }
                ));
            }),
            Rc::new(move |dex_id, _, _, _, _, _, _, _| {
                assert_ok!(crate::Module::<Runtime>::swap_pair(
                    Origin::signed(ALICE()),
                    ALICE(),
                    dex_id,
                    BlackPepper.into(),
                    GoldenTicket.into(),
                    SwapAmount::WithDesiredInput {
                        desired_amount_in: desired_amount,
                        min_amount_out: balance!(0),
                    }
                ));
            }),
        ];

        // Run tests inside each behavior.
        for case in &cases {
            let mut new_tests = vec![prepare.clone(), case.clone()];
            new_tests.append(&mut descriptor.tests.clone());
            crate::Module::<Runtime>::preset_initial(new_tests);
        }

        // Case with original pool state, behavior is not prepended.
        let mut new_tests = vec![prepare.clone()];
        new_tests.append(&mut descriptor.tests.clone());
        crate::Module::<Runtime>::preset_initial(new_tests);
    }
}

macro_rules! simplify_swap_outcome(
 ($a: expr) => ({
     match $a {
         SwapOutcome { amount, fee } => (amount.into(), fee.into())
     }
 })
);

#[test]
fn can_exchange_all_directions() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, gt, bp, _, _, _, _, _| {
        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
            Origin::signed(ALICE()),
            dex_id,
            GoldenTicket.into(),
            BlackPepper.into(),
            balance!(100000),
            balance!(200000),
            0,
            0,
        ));
        assert!(crate::Module::<Runtime>::can_exchange(&dex_id, &gt, &bp));
        assert!(crate::Module::<Runtime>::can_exchange(&dex_id, &bp, &gt));
        // TODO: add tests for indirect exchange, i.e. both input and output are not base asset
    })]);
}

#[test]
fn quote_case_exact_input_for_output_base_first() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, gt, bp, _, _, _, _, _| {
        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
            Origin::signed(ALICE()),
            dex_id,
            GoldenTicket.into(),
            BlackPepper.into(),
            balance!(100000),
            balance!(200000),
            0,
            0,
        ));
        assert_eq!(
            simplify_swap_outcome!(crate::Module::<Runtime>::quote(
                &dex_id,
                &gt,
                &bp,
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(100000),
                    min_amount_out: balance!(50000),
                }
            )
            .unwrap()),
            (99849774661992989484226, balance!(300))
        );
    })]);
}

#[test]
fn quote_case_exact_input_for_output_base_second() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, gt, bp, _, _, _, _, _| {
        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
            Origin::signed(ALICE()),
            dex_id,
            GoldenTicket.into(),
            BlackPepper.into(),
            balance!(100000),
            balance!(200000),
            0,
            0,
        ));
        assert_eq!(
            simplify_swap_outcome!(crate::Module::<Runtime>::quote(
                &dex_id,
                &bp,
                &gt,
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(100000),
                    min_amount_out: 0,
                }
            )
            .unwrap()),
            (33233333333333333333334, 99999999999999999999)
        );
    })]);
}

#[test]
fn quote_case_exact_output_for_input_base_first() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, gt, bp, _, _, _, _, _| {
        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
            Origin::signed(ALICE()),
            dex_id,
            GoldenTicket.into(),
            BlackPepper.into(),
            balance!(100000),
            balance!(200000),
            0,
            0,
        ));
        assert_eq!(
            simplify_swap_outcome!(crate::Module::<Runtime>::quote(
                &dex_id,
                &gt,
                &bp,
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(100000),
                    max_amount_in: balance!(150000),
                }
            )
            .unwrap()),
            (100300902708124373119358, 300902708124373119358)
        );
    })]);
}

#[test]
fn quote_case_exact_output_for_input_base_second() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, gt, bp, _, _, _, _, _| {
        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
            Origin::signed(ALICE()),
            dex_id,
            GoldenTicket.into(),
            BlackPepper.into(),
            balance!(100000),
            balance!(200000),
            0,
            0,
        ));
        assert_eq!(
            simplify_swap_outcome!(crate::Module::<Runtime>::quote(
                &dex_id,
                &bp,
                &gt,
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(50000),
                    max_amount_in: balance!(999000),
                }
            )
            .unwrap()),
            (201207243460764587525150, 150451354062186559679)
        );
    })]);
}

#[test]
fn quote_case_exact_output_for_input_base_second_fail_with_out_of_bounds() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, gt, bp, _, _, _, _, _| {
        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
            Origin::signed(ALICE()),
            dex_id,
            GoldenTicket.into(),
            BlackPepper.into(),
            balance!(100000),
            balance!(200000),
            0,
            0,
        ));
        assert_noop!(
            crate::Module::<Runtime>::quote(
                &dex_id,
                &bp,
                &gt,
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(50000),
                    max_amount_in: balance!(90000),
                }
            ),
            crate::Error::<Runtime>::CalculatedValueIsOutOfDesiredBounds
        );
    })]);
}

#[test]
fn depositliq_large_values() {
    crate::Module::<Runtime>::preset_initial(vec![Rc::new(|dex_id, _, _, _, _, _, _, _| {
        assert_noop!(
            crate::Module::<Runtime>::deposit_liquidity(
                Origin::signed(ALICE()),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                balance!(999360000),
                balance!(999144000),
                balance!(360000),
                balance!(144000),
            ),
            crate::Error::<Runtime>::SourceBaseAmountIsNotLargeEnough
        );
    })]);
}

#[test]
fn depositliq_valid_range_but_desired_is_corrected() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, _, _, _, _, _, _, _| {
            assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
                Origin::signed(ALICE()),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                balance!(360000),
                balance!(999000),
                balance!(350000),
                balance!(143000),
            ));
        },
    )]);
}

#[test]
fn pool_is_already_initialized_and_other_after_depositliq() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, repr: AccountId, fee_repr: AccountId| {
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &repr.clone()).unwrap(),
                balance!(144000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &repr.clone()).unwrap(),
                balance!(360000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &fee_repr.clone()).unwrap(),
                0
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &fee_repr.clone()).unwrap(),
                0
            );

            assert_noop!(
                crate::Module::<Runtime>::initialize_pool(
                    Origin::signed(BOB()),
                    dex_id.clone(),
                    GoldenTicket.into(),
                    BlackPepper.into(),
                ),
                crate::Error::<Runtime>::PoolIsAlreadyInitialized
            );
        },
    )]);
}

#[test]
fn swap_pair_desired_output_and_withdraw_cascade() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, repr: AccountId, fee_repr: AccountId| {
            assert_ok!(crate::Module::<Runtime>::swap_pair(
                Origin::signed(ALICE()),
                ALICE(),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(33000),
                    max_amount_in: balance!(99999999),
                }
            ));
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                432650925750223643890137
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                balance!(1889000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &repr.clone()).unwrap(),
                467027027027027027041534
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &repr.clone()).unwrap(),
                balance!(111000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &fee_repr.clone()).unwrap(),
                322047222749329068329
            );

            // a = sqrt ( 467027 * 111000 ) / 8784 = 25.92001146000573
            // b = 467_027 / a = 18018.00900900901
            // c = 111_000 / a = 4282.405514028097
            // Testing this line with noop
            // fail for each asset min, after this success.

            // First minimum is above boundaries.
            assert_noop!(
                crate::Module::<Runtime>::withdraw_liquidity(
                    Origin::signed(ALICE()),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    balance!(8784),
                    balance!(18100),
                    balance!(4100)
                ),
                crate::Error::<Runtime>::CalculatedValueIsNotMeetsRequiredBoundaries
            );

            // Second minimum is above boundaries.
            assert_noop!(
                crate::Module::<Runtime>::withdraw_liquidity(
                    Origin::signed(ALICE()),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    balance!(8784),
                    balance!(18000),
                    balance!(4300)
                ),
                crate::Error::<Runtime>::CalculatedValueIsNotMeetsRequiredBoundaries
            );

            // Both minimums is below.
            assert_ok!(crate::Module::<Runtime>::withdraw_liquidity(
                Origin::signed(ALICE()),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                balance!(8784),
                balance!(18000),
                balance!(4200),
            ));

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                450668729188225185978702
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                1893282356407400019291402
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &repr.clone()).unwrap(),
                449009223589025484952969
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &repr.clone()).unwrap(),
                106717643592599980708598
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &fee_repr.clone()).unwrap(),
                322047222749329068329
            );

            assert_ok!(crate::Module::<Runtime>::swap_pair(
                Origin::signed(ALICE()),
                ALICE(),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(33000),
                    max_amount_in: balance!(99999999),
                }
            ));

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                249063125369447164991908
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                1926282356407400019291402
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &repr.clone()).unwrap(),
                650010010596347171876803
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &repr.clone()).unwrap(),
                73717643592599980708598
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &fee_repr.clone()).unwrap(),
                926864034205663131289
            );
        },
    )]);
}

#[test]
fn swap_pair_desired_input() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, repr: AccountId, fee_repr: AccountId| {
            assert_ok!(crate::Module::<Runtime>::swap_pair(
                Origin::signed(ALICE()),
                ALICE(),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(33000),
                    min_amount_out: 0,
                }
            ));
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                balance!(507000)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                1868058365847885345166231
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &repr.clone()).unwrap(),
                balance!(392901)
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &repr.clone()).unwrap(),
                131941634152114654833769
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &fee_repr.clone()).unwrap(),
                balance!(99)
            );
        },
    )]);
}

#[test]
fn swap_pair_invalid_dex_id() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(|_, _, _, _, _, _, _, _| {
        assert_noop!(
            crate::Module::<Runtime>::swap_pair(
                Origin::signed(ALICE()),
                ALICE(),
                380,
                GoldenTicket.into(),
                BlackPepper.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(33000),
                    max_amount_in: balance!(99999999),
                }
            ),
            dex_manager::Error::<Runtime>::DEXDoesNotExist
        );
    })]);
}

#[test]
fn swap_pair_different_asset_pair() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, _, _, _, _, _, _, _| {
            assert_noop!(
                crate::Module::<Runtime>::swap_pair(
                    Origin::signed(ALICE()),
                    ALICE(),
                    dex_id,
                    GoldenTicket.into(),
                    RedPepper.into(),
                    SwapAmount::WithDesiredOutput {
                        desired_amount_out: balance!(33000),
                        max_amount_in: balance!(99999999),
                    }
                ),
                technical::Error::<Runtime>::TechAccountIdIsNotRegistered
            );
        },
    )]);
}

#[test]
fn swap_pair_swap_fail_with_invalid_balance() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, _, _, _, _, _, _, _| {
            assert_noop!(
                crate::Module::<Runtime>::swap_pair(
                    Origin::signed(BOB()),
                    BOB(),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    SwapAmount::WithDesiredOutput {
                        desired_amount_out: balance!(33000),
                        max_amount_in: balance!(999999999),
                    }
                ),
                crate::Error::<Runtime>::AccountBalanceIsInvalid
            );
        },
    )]);
}

#[test]
fn swap_pair_outcome_should_match_actual_desired_amount_in_with_basic_asset() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, _repr: AccountId, _fee_repr: AccountId| {
            use sp_core::crypto::AccountId32;
            let new_account = AccountId32::from([33; 32]);
            assets::Pallet::<Runtime>::transfer(
                Origin::signed(ALICE()),
                gt.clone(),
                new_account.clone(),
                balance!(100000),
            )
            .expect("Failed to transfer balance");

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                balance!(440000),
            );
            let quote_outcome = crate::Module::<Runtime>::quote(
                &dex_id,
                &GoldenTicket.into(),
                &BlackPepper.into(),
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(100000),
                    min_amount_out: 0,
                },
            )
            .expect("Failed to quote.");
            let outcome = crate::Module::<Runtime>::exchange(
                &new_account,
                &new_account,
                &dex_id,
                &GoldenTicket.into(),
                &BlackPepper.into(),
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(100000),
                    min_amount_out: 0,
                },
            )
            .expect("Failed to perform swap.");
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &new_account.clone()).unwrap(),
                0,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &new_account.clone()).unwrap(),
                balance!(31230.802697411355232759),
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &new_account.clone()).unwrap(),
                outcome.amount,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &new_account.clone()).unwrap(),
                quote_outcome.amount,
            );
        },
    )]);
}

#[test]
fn swap_pair_outcome_should_match_actual_desired_amount_in() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, _repr: AccountId, _fee_repr: AccountId| {
            use sp_core::crypto::AccountId32;
            let new_account = AccountId32::from([3; 32]);
            assets::Pallet::<Runtime>::transfer(
                Origin::signed(ALICE()),
                bp.clone(),
                new_account.clone(),
                balance!(100000),
            )
            .expect("Failed to transfer balance");

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                balance!(1756000),
            );
            let quote_outcome = crate::Module::<Runtime>::quote(
                &dex_id,
                &BlackPepper.into(),
                &GoldenTicket.into(),
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(100000),
                    min_amount_out: 0,
                },
            )
            .expect("Failed to quote.");
            let outcome = crate::Module::<Runtime>::exchange(
                &new_account,
                &new_account,
                &dex_id,
                &BlackPepper.into(),
                &GoldenTicket.into(),
                SwapAmount::WithDesiredInput {
                    desired_amount_in: balance!(100000),
                    min_amount_out: 0,
                },
            )
            .expect("Failed to perform swap.");
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &new_account.clone()).unwrap(),
                0,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &new_account.clone()).unwrap(),
                147098360655737705086834,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &new_account.clone()).unwrap(),
                outcome.amount,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &new_account.clone()).unwrap(),
                quote_outcome.amount,
            );
        },
    )]);
}

#[test]
fn swap_pair_outcome_should_match_actual_desired_amount_out_with_values_for_math_error_testing() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, _repr: AccountId, _fee_repr: AccountId| {
            use sp_core::crypto::AccountId32;
            let new_account = AccountId32::from([3; 32]);
            assets::Pallet::<Runtime>::transfer(
                Origin::signed(ALICE()),
                gt.clone(),
                new_account.clone(),
                balance!(100000.000000000000027777), // FIXME: why such a huge calculation error compared to WithDesiredInput(100000): ...027777?
            )
            .expect("Failed to transfer balance");

            // TODO: uncomment when ..027777 error is fixed
            // assert_eq!(
            //     assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
            //     balance!(440000),
            // );
            let quote_outcome = crate::Module::<Runtime>::quote(
                &dex_id,
                &GoldenTicket.into(),
                &BlackPepper.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(31230.802697411355232759),
                    max_amount_in: Balance::MAX,
                },
            )
            .expect("Failed to quote.");
            let _outcome = crate::Module::<Runtime>::exchange(
                &new_account,
                &new_account,
                &dex_id,
                &GoldenTicket.into(),
                &BlackPepper.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(31230.802697411355232759),
                    max_amount_in: Balance::MAX,
                },
            )
            .expect("Failed to perform swap.");
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &new_account.clone()).unwrap(),
                0,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &new_account.clone()).unwrap(),
                balance!(31230.802697411355232759),
            );
            assert_eq!(
                balance!(100000.000000000000027777), // FIXME: why such a huge calculation error compared to WithDesiredInput(100000): ...027777?
                quote_outcome.amount,
            );
            // TODO: FIXME: for case with desired output, outcome indicates calculated input
            // 100000.000000000000027777
            //assert_eq!(balance!(100000), outcome.amount);
        },
    )]);
}

#[test]
fn swap_pair_outcome_should_match_actual_desired_amount_out() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id, gt, bp, _, _, _, _repr: AccountId, _fee_repr: AccountId| {
            use sp_core::crypto::AccountId32;
            let new_account = AccountId32::from([3; 32]);
            assets::Pallet::<Runtime>::transfer(
                Origin::signed(ALICE()),
                bp.clone(),
                new_account.clone(),
                balance!(100000),
            )
            .expect("Failed to transfer balance");

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                balance!(1756000),
            );
            let quote_outcome = crate::Module::<Runtime>::quote(
                &dex_id,
                &BlackPepper.into(),
                &GoldenTicket.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(147098.360655737704918033),
                    max_amount_in: Balance::MAX,
                },
            )
            .expect("Failed to quote.");
            let outcome = crate::Module::<Runtime>::exchange(
                &new_account,
                &new_account,
                &dex_id,
                &BlackPepper.into(),
                &GoldenTicket.into(),
                SwapAmount::WithDesiredOutput {
                    desired_amount_out: balance!(147098.360655737704918033),
                    max_amount_in: Balance::MAX,
                },
            )
            .expect("Failed to perform swap.");
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &new_account.clone()).unwrap(),
                0,
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &new_account.clone()).unwrap(),
                //TODO: what is the problem here ?
                //balance!(147098.360655737704918033),
                balance!(146655.737704918032786886),
            );
            assert_eq!(balance!(100000), quote_outcome.amount);
            assert_eq!(balance!(100000), outcome.amount);
        },
    )]);
}

#[test]
fn swap_pair_liquidity_after_operation_check() {
    crate::Module::<Runtime>::preset_deposited_small(vec![Rc::new(
        |dex_id, _gt, _bp, _, _, _, _repr: AccountId, _fee_repr: AccountId| {
            assert_noop!(
                crate::Module::<Runtime>::swap_pair(
                    Origin::signed(ALICE()),
                    ALICE(),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    SwapAmount::WithDesiredOutput {
                        desired_amount_out: balance!(0.0099999),
                        max_amount_in: Balance::MAX,
                    }
                ),
                crate::Error::<Runtime>::PoolBecameInvalidAfterOperation
            );
        },
    )]);
}

#[test]
fn withdraw_all_liquidity() {
    crate::Module::<Runtime>::preset_deposited_pool(vec![Rc::new(
        |dex_id,
         gt,
         bp,
         _,
         tech_acc_id: crate::mock::TechAccountId,
         _,
         _repr: AccountId,
         _fee_repr: AccountId| {
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                balance!(540000.0),
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                balance!(1856000.0),
            );

            let tech_asset: AssetId = crate::Module::<Runtime>::get_marking_asset(&tech_acc_id)
                .expect("Failed to get marking asset")
                .into();
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&tech_asset, &ALICE()).unwrap(),
                balance!(227683.9915321233119024),
            );

            assert_noop!(
                crate::Module::<Runtime>::withdraw_liquidity(
                    Origin::signed(ALICE()),
                    dex_id,
                    GoldenTicket.into(),
                    BlackPepper.into(),
                    balance!(227683.9915321233119025),
                    0,
                    0
                ),
                crate::Error::<Runtime>::SourceBalanceOfLiquidityTokensIsNotLargeEnough
            );

            assert_ok!(crate::Module::<Runtime>::withdraw_liquidity(
                Origin::signed(ALICE()),
                dex_id,
                GoldenTicket.into(),
                BlackPepper.into(),
                balance!(227683.9915321233119024),
                0,
                0
            ));

            let tech_asset: AssetId = crate::Module::<Runtime>::get_marking_asset(&tech_acc_id)
                .expect("Failed to get marking asset")
                .into();
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&tech_asset, &ALICE()).unwrap(),
                0,
            );

            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&gt, &ALICE()).unwrap(),
                balance!(899999.999999999999998418),
            );
            assert_eq!(
                assets::Pallet::<Runtime>::free_balance(&bp, &ALICE()).unwrap(),
                balance!(1999999.999999999999999367),
            );
            // small fractions are lost due to min_liquidity locked for initial provider
            // and also rounding proportions such that user does not withdraw more thus breaking the pool
            // 900000.0 - 540000.0 = 360000.0
            // 2000000.0 - 1856000.0 = 144000.0
        },
    )]);
}

#[test]
fn deposit_liquidity_with_different_slippage_behavior() {
    crate::Module::<Runtime>::run_tests_with_different_slippage_behavior(
        RunTestsWithSlippageBehaviors {
            initial_deposit: (balance!(360000), balance!(144000)),
            desired_amount: balance!(2999),
            tests: vec![Rc::new(
                |dex_id,
                 _gt,
                 _bp,
                 _,
                 _tech_acc_id: crate::mock::TechAccountId,
                 _,
                 _repr: AccountId,
                 _fee_repr: AccountId| {
                    assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
                        Origin::signed(ALICE()),
                        dex_id,
                        GoldenTicket.into(),
                        BlackPepper.into(),
                        balance!(360000),
                        balance!(144000),
                        balance!(345000),
                        balance!(137000),
                    ));
                },
            )],
        },
    );
}

#[test]
fn withdraw_liquidity_with_different_slippage_behavior() {
    crate::Module::<Runtime>::run_tests_with_different_slippage_behavior(
        RunTestsWithSlippageBehaviors {
            initial_deposit: (balance!(360000), balance!(144000)),
            desired_amount: balance!(2999),
            tests: vec![Rc::new(
                |dex_id,
                 _gt,
                 _bp,
                 _,
                 _tech_acc_id: crate::mock::TechAccountId,
                 _,
                 _repr: AccountId,
                 _fee_repr: AccountId| {
                    assert_ok!(crate::Module::<Runtime>::withdraw_liquidity(
                        Origin::signed(ALICE()),
                        dex_id,
                        GoldenTicket.into(),
                        BlackPepper.into(),
                        balance!(227683),
                        balance!(352000),
                        balance!(141000),
                    ));
                },
            )],
        },
    );
}

#[test]
fn variants_of_deposit_liquidity_twice() {
    let variants: Vec<Balance> = vec![1u128, 10u128, 100u128, 1000u128, 10000u128];

    for scale in variants {
        crate::Module::<Runtime>::run_tests_with_different_slippage_behavior(
            RunTestsWithSlippageBehaviors {
                initial_deposit: (balance!(10.13097) * scale, balance!(8.09525) * scale),
                desired_amount: balance!(0.0005) * scale,
                tests: vec![Rc::new(
                    |dex_id,
                     _gt,
                     _bp,
                     _,
                     _tech_acc_id: crate::mock::TechAccountId,
                     _,
                     _repr: AccountId,
                     _fee_repr: AccountId| {
                        assert_ok!(crate::Module::<Runtime>::deposit_liquidity(
                            Origin::signed(ALICE()),
                            dex_id,
                            GoldenTicket.into(),
                            BlackPepper.into(),
                            balance!(20) * scale,
                            balance!(15.98291400432839) * scale,
                            balance!(19.9) * scale,
                            balance!(15.90299943430675) * scale,
                        ));
                    },
                )],
            },
        );
    }
}
