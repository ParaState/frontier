// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(test)]

use super::*;

use std::{str::FromStr, collections::BTreeMap};
use frame_support::{
	assert_ok, assert_err, impl_outer_origin, parameter_types, impl_outer_dispatch,
	traits::GenesisBuild,
};
use sp_core::{Blake2Hasher, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum OuterCall for Test where origin: Origin {
		self::EVM,
	}
}

pub struct PalletInfo;

impl frame_support::traits::PalletInfo for PalletInfo {
	fn index<P: 'static>() -> Option<usize> {
		return Some(0)
	}

	fn name<P: 'static>() -> Option<&'static str> {
		return Some("TestName")
	}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = OuterCall;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type Balance = u64;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

/// Fixed gas price of `0`.
pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		// Gas price is always one token per gas.
		0.into()
	}
}

impl Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();

	type CallOrigin = EnsureAddressRoot<Self::AccountId>;
	type WithdrawOrigin = EnsureAddressNever<Self::AccountId>;

	type AddressMapping = HashedAddressMapping<Blake2Hasher>;
	type Currency = Balances;
	type Runner = crate::runner::stack::Runner<Self>;

	type Event = ();
	type Precompiles = ();
	type ChainId = ();
	type BlockGasLimit = ();
	type OnChargeTransaction = ();
}

type System = frame_system::Pallet<Test>;
type Balances = pallet_balances::Pallet<Test>;
type EVM = Pallet<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut accounts = BTreeMap::new();
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000001").unwrap(),
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::from(1000000),
			storage: Default::default(),
			code: vec![
				0x00, // STOP
			],
		}
	);
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000002").unwrap(),
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::from(1000000),
			storage: Default::default(),
			code: vec![
				0xff, // INVALID
			],
		}
	);
	let beneficiaries: Vec<H160> = vec![];

	pallet_balances::GenesisConfig::<Test>::default().assimilate_storage(&mut t).unwrap();
	<GenesisConfig as GenesisBuild<Test>>::assimilate_storage(&GenesisConfig { accounts, beneficiaries }, &mut t).unwrap();
	t.into()
}

#[cfg(feature = "debug")]
#[test]
fn fail_call_return_ok() {
	new_test_ext().execute_with(|| {
		assert_ok!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::default(),
			None,
		));

		assert_ok!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000002").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::default(),
			None,
		));
	});
}

#[cfg(not(feature = "debug"))]
#[test]
fn fail_call_return_forbidden() {
	new_test_ext().execute_with(|| {
		assert_err!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::default(),
			None,
		), Error::<Test>::Forbidden);

		assert_err!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000002").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::default(),
			None,
		), Error::<Test>::Forbidden);
	});
}

#[test]
fn fee_deduction() {
	new_test_ext().execute_with(|| {
		// Create an EVM address and the corresponding Substrate address that will be charged fees and refunded
		let evm_addr = H160::from_str("1000000000000000000000000000000000000003").unwrap();
		let substrate_addr = <Test as Config>::AddressMapping::into_account_id(evm_addr);

		// Seed account
		let _ = <Test as Config>::Currency::deposit_creating(&substrate_addr, 100);
		assert_eq!(Balances::free_balance(&substrate_addr), 100);

		// Deduct fees as 10 units
		let imbalance = <<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::withdraw_fee(&evm_addr, U256::from(10)).unwrap();
		assert_eq!(Balances::free_balance(&substrate_addr), 90);

		// Refund fees as 5 units
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), imbalance).unwrap();
		assert_eq!(Balances::free_balance(&substrate_addr), 95);
	});
}

fn ten_units_imbalance(evm_addr: &H160) -> <<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::LiquidityInfo {
	<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::withdraw_fee(&evm_addr, U256::from(10)).unwrap()
}

#[test]
fn rotate_license_fee_beneficiaries() {
	new_test_ext().execute_with(|| {
		// Create an EVM address and the corresponding Substrate address that will be charged fees and refunded
		let evm_addr = H160::from_str("1000000000000000000000000000000000000004").unwrap();
		let substrate_addr = <Test as Config>::AddressMapping::into_account_id(evm_addr);

		// Seed account
		let _ = <Test as Config>::Currency::deposit_creating(&substrate_addr, 100);
		assert_eq!(Balances::free_balance(&substrate_addr), 100);

		// Burn 5 units as gas-fees
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), ten_units_imbalance(&evm_addr));
		assert_eq!(Balances::free_balance(&substrate_addr), 95);

		// Add 3 beneficiaries
		let evm_addr_beneficiary_1 = H160::from_str("2000000000000000000000000000000000000001").unwrap();
		let substrate_addr_beneficiary_1 = <Test as Config>::AddressMapping::into_account_id(evm_addr_beneficiary_1);
		EVM::add_beneficiary(
			Origin::root(),
			evm_addr_beneficiary_1,
		);
		let evm_addr_beneficiary_2 = H160::from_str("2000000000000000000000000000000000000002").unwrap();
		let substrate_addr_beneficiary_2 = <Test as Config>::AddressMapping::into_account_id(evm_addr_beneficiary_2);
		EVM::add_beneficiary(
			Origin::root(),
			evm_addr_beneficiary_2,
		);
		let evm_addr_beneficiary_3 = H160::from_str("2000000000000000000000000000000000000003").unwrap();
		let substrate_addr_beneficiary_3 = <Test as Config>::AddressMapping::into_account_id(evm_addr_beneficiary_3);
		EVM::add_beneficiary(
			Origin::root(),
			evm_addr_beneficiary_3,
		);
		assert_eq!(<BenefitCount<Test>>::get(), 3);

		// Charge 5 units to beneficiary_1 as license-fee
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), ten_units_imbalance(&evm_addr));
		assert_eq!(Balances::free_balance(&substrate_addr), 90);
		assert_eq!(Balances::free_balance(&substrate_addr_beneficiary_1), 5);

		// Charge 5 units to beneficiary_2 as license-fee
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), ten_units_imbalance(&evm_addr));
		assert_eq!(Balances::free_balance(&substrate_addr), 85);
		assert_eq!(Balances::free_balance(&substrate_addr_beneficiary_2), 5);

		// Charge 5 units to beneficiary_3 as license-fee
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), ten_units_imbalance(&evm_addr));
		assert_eq!(Balances::free_balance(&substrate_addr), 80);
		assert_eq!(Balances::free_balance(&substrate_addr_beneficiary_3), 5);

		// Charge 5 units to beneficiary_1 as license-fee
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), ten_units_imbalance(&evm_addr));
		assert_eq!(Balances::free_balance(&substrate_addr), 75);
		assert_eq!(Balances::free_balance(&substrate_addr_beneficiary_1), 10);

		// Delete one beneficiary with index 2
		EVM::delete_beneficiary(
			Origin::root(),
			2,
		);
		assert_eq!(<BenefitCount<Test>>::get(), 2);
	});
}

#[test]
fn set_eth_addr() {
	new_test_ext().execute_with(|| {
		let evm_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let sender: AccountId32 = [0u8;32].into();
		assert_eq!(EVM::eth_addr(&sender), None);
		assert_ok!(EVM::set_eth_addr(Origin::signed(sender.clone()), evm_addr));
		assert_eq!(EVM::eth_addr(&sender), Some(evm_addr));
	});
}
