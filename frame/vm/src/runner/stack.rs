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

//! EVM stack-based runner.
use sp_std::{marker::PhantomData, vec, vec::Vec, boxed::Box, mem, collections::btree_set::BTreeSet, convert::TryInto, if_std};
use sp_core::{U256, H256, H160};
use sp_runtime::traits::UniqueSaturatedInto;
use frame_support::{
	debug, ensure, traits::{Get, Currency, ExistenceRequirement},
	storage::{StorageMap, StorageDoubleMap},
};
use sha3::{Keccak256, Digest};
use sha2::Sha256;
use fp_vm::{ExecutionInfo, CallInfo, CreateInfo, Log, Vicinity, ExtendExitReason};
use evm::{ExitReason, ExitError, ExitFatal, Transfer};
use evm::backend::Backend as BackendT;
use evm::executor::{StackExecutor, StackSubstateMetadata, StackState as StackStateT};
use crate::{
	Config, AccountStorages, FeeCalculator, AccountCodes, Module, Event,
	Error, AddressMapping, PrecompileSet,
};
#[cfg(feature = "std")]
use ssvm::host::HostContext as HostInterface;
#[cfg(feature = "std")]
use ssvm::types::*;
use crate::runner::Runner as RunnerT;

#[derive(PartialEq)]
pub enum ByteCodeKind {
	EVM,
	EWASM,
}

fn is_wasm(code: &Vec<u8>) -> bool{
	if code.get(0..4).unwrap_or(&vec![0; 4]) == [0x00, 0x61, 0x73, 0x6d] {
		return true;
	}
	else {
		return false;
	}
}

#[derive(Default)]
pub struct Runner<T: Config> {
	_marker: PhantomData<T>,
}

impl<T: Config> Runner<T> {

	/// pre-processing shared between two vm types, avoid duplicate code
	pub(self) fn pre_processing(
		source: H160,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>
		) -> Result<(U256, U256), Error<T>>
	{
		// Gas price check is skipped when performing a gas estimation.
		let gas_price = match gas_price {
			Some(gas_price) => {
				ensure!(gas_price >= T::FeeCalculator::min_gas_price(), Error::<T>::GasPriceTooLow);
				gas_price
			},
			None => Default::default(),
		};
		let total_fee = gas_price.checked_mul(U256::from(gas_limit))
			.ok_or(Error::<T>::FeeOverflow)?;
		let total_payment = value.checked_add(total_fee).ok_or(Error::<T>::PaymentOverflow)?;
		let source_account = Module::<T>::account_basic(&source);
		ensure!(source_account.balance >= total_payment, Error::<T>::BalanceLow);
		Module::<T>::withdraw_fee(&source, total_fee)?;
		if let Some(nonce) = nonce {
			ensure!(source_account.nonce == nonce, Error::<T>::InvalidNonce);
		}
		return Ok((gas_price, total_fee));
	}

	/// post-processing shared between two vm types, avoid duplicate code
	pub(self) fn post_processing<'config>(
		source: H160,
		total_fee: U256,
		actual_fee: U256,
		state: &mut VmStackState<'_, 'config, T>
		)
	{
		debug::debug!(
			target: "vm",
			"Execution [source: {:?}, total_fee: {}, actual_fee: {}]",
			source,
			total_fee,
			actual_fee
		);

		Module::<T>::deposit_fee(&source, total_fee.saturating_sub(actual_fee));
		for address in &state.substate.deletes {
			debug::debug!(
				target: "vm",
				"Deleting account at {:?}",
				address
			);
			Module::<T>::remove_account(&address)
		}

		for log in &state.substate.logs {
			debug::trace!(
				target: "vm",
				"Inserting log for {:?}, topics ({}) {:?}, data ({}): {:?}]",
				log.address,
				log.topics.len(),
				log.topics,
				log.data.len(),
				log.data
			);
			Module::<T>::deposit_event(Event::<T>::Log(Log {
				address: log.address,
				topics: log.topics.clone(),
				data: log.data.clone(),
			}));
		}
	}

	/// Execute an EVM operation.
	pub fn execute_evm<'config, F, R>(
		source: H160,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &'config evm::Config,
		f: F,
	) -> Result<ExecutionInfo<R>, Error<T>> where
		F: FnOnce(&mut StackExecutor<'config, VmStackState<'_, 'config, T>>) -> (ExitReason, R),
	{
		let (gas_price, total_fee) = Self::pre_processing(source, value, gas_limit, gas_price, nonce).unwrap();

		let vicinity = Vicinity {
			gas_price,
			origin: source,
		};
		let metadata = StackSubstateMetadata::new(gas_limit, &config);
		let state = VmStackState::new(&vicinity, metadata, None);
		let mut executor = StackExecutor::new_with_precompile(
			state,
			config,
			T::Precompiles::execute,
		);
		let (reason, retv) = f(&mut executor);
		let used_gas = U256::from(executor.used_gas());
		let actual_fee = executor.fee(gas_price);
		let mut state = executor.into_state();

		Self::post_processing(source, total_fee, actual_fee, &mut state);

		Ok(ExecutionInfo {
			value: retv,
			exit_reason: ExtendExitReason::ExitReason(reason),
			used_gas,
			logs: state.substate.logs,
		})
	}

	#[cfg(feature = "std")]
	pub(self) fn execute_precompiles(
		target: &H160,
		data: &Vec<u8>,
		gas_limit: &u64,
  ) -> (bool, Vec<u8>, i64) {
		let s: String = rustc_hex::ToHexIter::new(target.as_bytes().iter()).collect::<String>();
		match &s[..] {
			"0000000000000000000000000000000000000002" => {
			return (true, Sha256::digest(&data).to_vec(), *gas_limit as i64);
		}
			"0000000000000000000000000000000000000009" => {
			return (true, Keccak256::digest(&data).to_vec(), *gas_limit as i64);
		}
		_ => {
				return (false, vec![0u8], *gas_limit as i64);
			}
		}
	}

	/// Execute an SSVM operation.
	#[cfg(feature = "std")]
	pub fn execute_ssvm<'config>(
		source: H160,
		target: H160,
		value: U256,
		data: Vec<u8>,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		call_kind: CallKind,
		config: &'config evm::Config,
		salt: Option<H256>,
	) -> Result<(Vec<u8>, ExtendExitReason, U256, Vec<Log>), Error<T>> {

		let (gas_price, total_fee) = Self::pre_processing(source, value, gas_limit, gas_price, nonce).unwrap();


		// No coinbase, difficulty in substrate nodes.
		let coinbase = H160::zero();
		let difficulty = U256::zero();
		let block_number: u128 = frame_system::Module::<T>::block_number().unique_saturated_into();
		let timestamp: u128 = pallet_timestamp::Module::<T>::get().unique_saturated_into();
		let code = match call_kind {
			CallKind::EVMC_CALL => AccountCodes::get(&target),
			CallKind::EVMC_CREATE => data.to_owned(),
			CallKind::EVMC_CREATE2 => data.to_owned(),
			_ => vec![0; 0],
		};
		let tx_context = TxContext::new(
			gas_price,
			source,
			coinbase,
			block_number.try_into().unwrap(),
			timestamp.try_into().unwrap(),
			gas_limit as i64,
			difficulty,
			);
		let is_static = false;
		let depth = 0;
		let vicinity = Vicinity {
			gas_price,
			origin: source,
		};
		let metadata = StackSubstateMetadata::new(gas_limit, &config);
		let mut state = VmStackState::<T>::new(&vicinity, metadata, Some(tx_context));
		state.inc_nonce(source);
		state.substate.enter(gas_limit, is_static);
		let (output, gas_left, status_code) = {
			let (is_precompiles, output, gas_left) = Self::execute_precompiles(&target, &data, &gas_limit);
			if is_precompiles {
				(output, gas_left, StatusCode::EVMC_SUCCESS)
			}
			else {
				let vm = ssvm::create();
				let (output, gas_left, status_code) = vm.execute(
				&mut state,
				Revision::EVMC_BYZANTIUM,
				call_kind,
				is_static,
				depth,
				gas_limit as i64,
				target.as_fixed_bytes(),
				source.as_fixed_bytes(),
				&data[..],
				&value.into(),
				&code,
				&salt.unwrap_or(H256::zero()).as_fixed_bytes(),
				);
				(output.to_vec(), gas_left, status_code)
			}
		};
		let used_gas = gas_limit as i64 - gas_left;
		let actual_fee = U256::from(used_gas) * gas_price;

		Self::post_processing(source, total_fee, actual_fee, &mut state);

		let _ = match status_code {
			StatusCode::EVMC_SUCCESS => {
				if call_kind == CallKind::EVMC_CREATE || call_kind == CallKind::EVMC_CREATE2 {
					AccountCodes::insert(target, output.to_owned());
				}
				state.substate.exit_commit()
			},
			StatusCode::EVMC_REVERT => state.substate.exit_revert(),
			_ => state.substate.exit_discard(),
		};

		Ok((
			output,
			ExtendExitReason::EVMCStatusCode(status_code.into()),
			U256::from(used_gas),
			state.substate.logs
		))
	}
}

impl<T: Config> RunnerT<T> for Runner<T> {
	type Error = Error<T>;

	fn call(
		source: H160,
		target: H160,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CallInfo, Self::Error> {
		if_std! {
			let code = AccountCodes::get(&target);
			if is_wasm(&code) {
				return match Self::execute_ssvm(
					source,
					target,
					value,
					input,
					gas_limit,
					gas_price,
					nonce,
					CallKind::EVMC_CALL,
					config,
					None
				) {
					Ok((value, exit_reason, used_gas, logs)) => {
						Ok(ExecutionInfo {
							value: value,
							exit_reason: exit_reason,
							used_gas: used_gas,
							logs: logs,
						})
					},
					Err(e) => Err(e),
				}
			}
		}
		return Self::execute_evm(
			source,
			value,
			gas_limit,
			gas_price,
			nonce,
			config,
			|executor| executor.transact_call(
				source,
				target,
				value,
				input,
				gas_limit,
			),
		);
	}

	fn create(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CreateInfo, Self::Error> {
		if_std! {
			if is_wasm(&init) {
				let address = create_address(source,
					nonce.unwrap_or(Module::<T>::account_basic(&source).nonce));
				return match Self::execute_ssvm(
					source,
					address,
					value,
					init,
					gas_limit,
					gas_price,
					nonce,
					CallKind::EVMC_CREATE,
					config,
					None
				) {
					Ok((_, exit_reason, used_gas, logs)) => {
						Ok(ExecutionInfo {
							value: address,
							exit_reason: exit_reason,
							used_gas: used_gas,
							logs: logs,
						})
					},
					Err(e) => Err(e),
				}
			}
		}
		return Self::execute_evm(
			source,
			value,
			gas_limit,
			gas_price,
			nonce,
			config,
			|executor| {
				let address = executor.create_address(
					evm::CreateScheme::Legacy { caller: source },
				);
				(executor.transact_create(
					source,
					value,
					init,
					gas_limit,
				), address)
			},
		);
	}

	fn create2(
		source: H160,
		init: Vec<u8>,
		salt: H256,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CreateInfo, Self::Error> {
		if_std! {
			if is_wasm(&init) {
				let code_hash = H256::from_slice(Keccak256::digest(&init).as_slice());
				let address = create2_address(source, salt, code_hash);
				return match Self::execute_ssvm(
					source,
					address,
					value,
					init,
					gas_limit,
					gas_price,
					nonce,
					CallKind::EVMC_CREATE2,
					config,
					Some(salt)
				) {
					Ok((_, exit_reason, used_gas, logs)) => {
						Ok(ExecutionInfo {
							value: address,
							exit_reason: exit_reason,
							used_gas: used_gas,
							logs: logs,
						})
					},
					Err(e) => Err(e),
				}
			}
		}
		let code_hash = H256::from_slice(Keccak256::digest(&init).as_slice());
		Self::execute_evm(
			source,
			value,
			gas_limit,
			gas_price,
			nonce,
			config,
			|executor| {
				let address = executor.create_address(
					evm::CreateScheme::Create2 { caller: source, code_hash, salt },
				);
				(executor.transact_create2(
					source,
					value,
					init,
					salt,
					gas_limit,
				), address)
			},
		)
	}
}

pub fn create_address(caller: H160, nonce: U256) -> H160 {
	let mut stream = rlp::RlpStream::new_list(2);
	stream.append(&caller);
	stream.append(&nonce);
	H256::from_slice(Keccak256::digest(&stream.out()).as_slice()).into()
}

pub fn create2_address(caller: H160, salt: H256, code_hash: H256) -> H160 {
	let mut hasher = Keccak256::new();
	hasher.input(&[0xff]);
	hasher.input(&caller[..]);
	hasher.input(&salt[..]);
	hasher.input(&code_hash[..]);
	H256::from_slice(hasher.result().as_slice()).into()
}

struct SubstrateStackSubstate<'config> {
	metadata: StackSubstateMetadata<'config>,
	deletes: BTreeSet<H160>,
	logs: Vec<Log>,
	parent: Option<Box<SubstrateStackSubstate<'config>>>,
}

impl<'config> SubstrateStackSubstate<'config> {
	pub fn metadata(&self) -> &StackSubstateMetadata<'config> {
		&self.metadata
	}

	pub fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
		&mut self.metadata
	}

	pub fn enter(&mut self, gas_limit: u64, is_static: bool) {
		let mut entering = Self {
			metadata: self.metadata.spit_child(gas_limit, is_static),
			parent: None,
			deletes: BTreeSet::new(),
			logs: Vec::new(),
		};
		mem::swap(&mut entering, self);

		self.parent = Some(Box::new(entering));

		sp_io::storage::start_transaction();
	}

	pub fn exit_commit(&mut self) -> Result<(), ExitError> {
		let mut exited = *self.parent.take().expect("Cannot commit on root substate");
		mem::swap(&mut exited, self);

		self.metadata.swallow_commit(exited.metadata)?;
		self.logs.append(&mut exited.logs);
		self.deletes.append(&mut exited.deletes);

		sp_io::storage::commit_transaction();
		Ok(())
	}

	pub fn exit_revert(&mut self) -> Result<(), ExitError> {
		let mut exited = *self.parent.take().expect("Cannot discard on root substate");
		mem::swap(&mut exited, self);

		self.metadata.swallow_revert(exited.metadata)?;
		self.logs.append(&mut exited.logs);

		sp_io::storage::rollback_transaction();
		Ok(())
	}

	pub fn exit_discard(&mut self) -> Result<(), ExitError> {
		let mut exited = *self.parent.take().expect("Cannot discard on root substate");
		mem::swap(&mut exited, self);

		self.metadata.swallow_discard(exited.metadata)?;
		self.logs.append(&mut exited.logs);

		sp_io::storage::rollback_transaction();
		Ok(())
	}

	pub fn deleted(&self, address: H160) -> bool {
		if self.deletes.contains(&address) {
			return true
		}

		if let Some(parent) = self.parent.as_ref() {
			return parent.deleted(address)
		}

		false
	}

	pub fn set_deleted(&mut self, address: H160) {
		self.deletes.insert(address);
	}

	pub fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
		self.logs.push(Log {
			address, topics, data,
		});
	}
}

#[derive(Clone, Debug, Copy)]
pub struct TxContext {
	tx_gas_price: U256,
	tx_origin: H160,
	block_coinbase: H160,
	block_number: i64,
	block_timestamp: i64,
	block_gas_limit: i64,
	block_difficulty: U256,
}

impl TxContext {
	pub fn new(
		tx_gas_price: U256,
		tx_origin: H160,
		block_coinbase: H160,
		block_number: i64,
		block_timestamp: i64,
		block_gas_limit: i64,
		block_difficulty: U256,
		) -> Self {
		Self {
			tx_gas_price,
			tx_origin,
			block_coinbase,
			block_number,
			block_timestamp,
			block_gas_limit,
			block_difficulty,
		}
	}
}

/// Substrate backend for VM.
pub struct VmStackState<'vicinity, 'config, T> {
	vicinity: &'vicinity Vicinity,
	substate: SubstrateStackSubstate<'config>,
	tx_context: Option<TxContext>,
	_marker: PhantomData<T>,
}

impl<'vicinity, 'config, T: Config> VmStackState<'vicinity, 'config, T> {
	/// Create a new backend with given vicinity.
	pub fn new(vicinity: &'vicinity Vicinity, metadata: StackSubstateMetadata<'config>, tx_context: Option<TxContext>) -> Self {
		Self {
			vicinity,
			substate: SubstrateStackSubstate {
				metadata,
				deletes: BTreeSet::new(),
				logs: Vec::new(),
				parent: None,
			},
			tx_context: tx_context,
			_marker: PhantomData
		}
	}
}

impl<'vicinity, 'config, T: Config> BackendT for VmStackState<'vicinity, 'config, T> {
	fn gas_price(&self) -> U256 { self.vicinity.gas_price }
	fn origin(&self) -> H160 { self.vicinity.origin }

	fn block_hash(&self, number: U256) -> H256 {
		if number > U256::from(u32::max_value()) {
			H256::default()
		} else {
			let number = T::BlockNumber::from(number.as_u32());
			H256::from_slice(frame_system::Module::<T>::block_hash(number).as_ref())
		}
	}

	fn block_number(&self) -> U256 {
		let number: u128 = frame_system::Module::<T>::block_number().unique_saturated_into();
		U256::from(number)
	}

	fn block_coinbase(&self) -> H160 {
		H160::default()
	}

	fn block_timestamp(&self) -> U256 {
		let now: u128 = pallet_timestamp::Module::<T>::get().unique_saturated_into();
		U256::from(now / 1000)
	}

	fn block_difficulty(&self) -> U256 {
		U256::zero()
	}

	fn block_gas_limit(&self) -> U256 {
		U256::zero()
	}

	fn chain_id(&self) -> U256 {
		U256::from(T::ChainId::get())
	}

	fn exists(&self, _address: H160) -> bool {
		true
	}

	fn basic(&self, address: H160) -> evm::backend::Basic {
		let account = Module::<T>::account_basic(&address);

		evm::backend::Basic {
			balance: account.balance,
			nonce: account.nonce,
		}
	}

	fn code(&self, address: H160) -> Vec<u8> {
		AccountCodes::get(&address)
	}

	fn storage(&self, address: H160, index: H256) -> H256 {
		AccountStorages::get(address, index)
	}

	fn original_storage(&self, _address: H160, _index: H256) -> Option<H256> {
		None
	}
}

#[cfg(feature = "std")]
impl<'vicinity, 'config, T: Config> HostInterface for VmStackState<'vicinity, 'config, T> {
	fn account_exists(&mut self, _addr: &[u8; ADDRESS_LENGTH]) -> bool {
		true
	}

	fn get_storage(&mut self, address: &Address, key: &Bytes32) -> Bytes32 {
		let ret = AccountStorages::get(H160::from(address), H256::from(key));
		ret.to_fixed_bytes()
	}

	fn set_storage(&mut self, address: &Address, key: &Bytes32, value: &Bytes32) -> StorageStatus {
		if H256::from(value.to_owned())== H256::default() {
			debug::debug!(
				target: "ssvm",
				"Removing storage for {:?} [index: {:?}]",
				address,
				key,
			);
			AccountStorages::remove(H160::from(address.to_owned()), H256::from(key.to_owned()));
		} else {
			debug::debug!(
				target: "ssvm",
				"Updating storage for {:?} [index: {:?}, value: {:?}]",
				address,
				key,
				value,
			);
			AccountStorages::insert(H160::from(address.to_owned()), H256::from(key.to_owned()),
				H256::from(value.to_owned()));
		}
		StorageStatus::EVMC_STORAGE_MODIFIED
	}

	fn get_balance(&mut self, address: &Address) -> Bytes32 {
		let account = Module::<T>::account_basic(&H160::from(address));
		account.balance.into()
	}

	fn get_code_size(&mut self, address: &Address) -> usize {
		AccountCodes::decode_len(H160::from(address)).unwrap_or(0)
	}

	fn get_code_hash(&mut self, address: &Address) -> Bytes32 {
		H256::from_slice(Keccak256::digest(&AccountCodes::get(H160::from(address))).as_slice())
		.into()
	}

	fn copy_code(
		&mut self,
		_addr: &Address,
		_offset: &usize,
		_buffer_data: &*mut u8,
		_buffer_size: &usize,
		) -> usize {
		0
	}

	fn selfdestruct(&mut self, _addr: &Address, _beneficiary: &Address) {}

	fn get_tx_context(&mut self) -> (Bytes32, Address, Address, i64, i64, i64, Bytes32) {
		let tx_ctx = self.tx_context.unwrap();
		(
			tx_ctx.tx_gas_price.into(),
			tx_ctx.tx_origin.to_fixed_bytes(),
			tx_ctx.block_coinbase.to_fixed_bytes(),
			tx_ctx.block_number,
			tx_ctx.block_timestamp,
			tx_ctx.block_gas_limit,
			tx_ctx.block_difficulty.into(),
			)
	}

	fn get_block_hash(&mut self, block_number: i64) -> Bytes32 {
		let number = U256::from(block_number);
		if number > U256::from(u32::max_value()) {
			H256::default().into()
		} else {
			let number = T::BlockNumber::from(number.as_u32());
			H256::from_slice(frame_system::Module::<T>::block_hash(number).as_ref()).into()
		}
	}

	fn emit_log(&mut self, address: &Address, topics: &Vec<Bytes32>, data: &Bytes) {
		self.substate.log(H160::from(address.to_owned()),
			topics
			.iter()
			.map(|b32| H256::from(b32))
			.collect::<Vec<H256>>(),
			data.to_vec());
	}

	fn call(
		&mut self,
		kind: CallKind,
		destination: &Address,
		sender: &Address,
		value: &Bytes32,
		input: &[u8],
		gas: i64,
		_depth: i32,
		_is_static: bool,
		salt: &Bytes32,
		) -> (Vec<u8>, i64, Address, StatusCode) {

			fn reason2status(reason: &ExitReason) -> StatusCode {
				match reason {
					ExitReason::Succeed(_)             => StatusCode::EVMC_SUCCESS,
					ExitReason::Error(status) => {
						match status {
							ExitError::StackUnderflow      => StatusCode::EVMC_STACK_UNDERFLOW,
							ExitError::StackOverflow       => StatusCode::EVMC_STACK_OVERFLOW,
							ExitError::InvalidJump         => StatusCode::EVMC_BAD_JUMP_DESTINATION,
							ExitError::InvalidRange        => StatusCode::EVMC_INVALID_MEMORY_ACCESS,
							ExitError::DesignatedInvalid   => StatusCode::EVMC_INVALID_INSTRUCTION,
							ExitError::CallTooDeep         => StatusCode::EVMC_CALL_DEPTH_EXCEEDED,
							ExitError::CreateCollision     => StatusCode::EVMC_FAILURE,
							ExitError::CreateContractLimit => StatusCode::EVMC_CONTRACT_VALIDATION_FAILURE,
							ExitError::OutOfOffset         => StatusCode::EVMC_FAILURE,
							ExitError::OutOfGas            => StatusCode::EVMC_OUT_OF_GAS,
							ExitError::OutOfFund           => StatusCode::EVMC_FAILURE,
							ExitError::PCUnderflow         => StatusCode::EVMC_FAILURE,
							ExitError::CreateEmpty         => StatusCode::EVMC_FAILURE,
							_                              => StatusCode::EVMC_FAILURE,
						}
					}
					ExitReason::Revert(_)   => StatusCode::EVMC_REVERT,
					ExitReason::Fatal(status) => {
						match status {
							ExitFatal::NotSupported        => StatusCode::EVMC_UNDEFINED_INSTRUCTION,
							ExitFatal::UnhandledInterrupt  => StatusCode::EVMC_WASM_TRAP,
							ExitFatal::CallErrorAsFatal(_) => StatusCode::EVMC_FAILURE,
							_                              => StatusCode::EVMC_FAILURE,
						}
					}
				}
			}

			fn get_value<T>(info: &Result<CallInfo, T>) -> Vec<u8> {
				match info {
	        Ok(info) => return info.value.clone(),
	        Err(_) => vec![0; 0]
      	}
			}

			fn get_address<T>(info: &Result<CreateInfo, T>) -> Address {
				match info {
	        Ok(info) => return info.value.into(),
	        Err(_) => [0u8; ADDRESS_LENGTH]
      	}
			}

			fn get_gas_left<R, T>(info: &Result<ExecutionInfo<R>, T>, orig_gas_left: i64) -> i64 {
				match info {
	        Ok(info) => orig_gas_left - info.used_gas.as_u64() as i64,
	        Err(_) => {
	        	// FIXME: fail tx need cost
	        	orig_gas_left
	        }
	      }
	    }

	    fn get_status_code<R, T>(info: &Result<ExecutionInfo<R>, T>) -> StatusCode {
	    	match info {
	    		Ok(info) => {
	    			match &info.exit_reason {
	    				ExtendExitReason::ExitReason(reason) => reason2status(reason),
	        		ExtendExitReason::EVMCStatusCode(status) => status.to_owned().into()
	        	}
	        },
	        Err(_) => StatusCode::EVMC_FAILURE,
      	}
			}

			let source = H160::from(sender);
			let account_id = T::AddressMapping::into_account_id(source);
			let account_basic = Module::<T>::account_basic(&source);
			frame_system::Module::<T>::inc_account_nonce(&account_id);
			let target = H160::from(destination);
      match kind {
        CallKind::EVMC_CALL => {
        	let info = T::Runner::call(source, target, AccountCodes::get(&target), U256::from(value), gas as u64, None, Some(account_basic.nonce), T::config());
        	(get_value(&info), get_gas_left(&info, gas), [0u8; ADDRESS_LENGTH], get_status_code(&info))
        }
        CallKind::EVMC_CREATE => {
        	let info = T::Runner::create(source, input.to_vec(), U256::from(value), gas as u64, None, Some(account_basic.nonce), T::config());
        	(vec![0; 0], get_gas_left(&info, gas), get_address(&info), get_status_code(&info))
        }
        CallKind::EVMC_CREATE2 => {
        	let info = T::Runner::create2(source, input.to_vec(), H256::from(salt), U256::from(value), gas as u64, None, Some(account_basic.nonce), T::config());
        	(vec![0; 0], get_gas_left(&info, gas), get_address(&info), get_status_code(&info))
        }
        _ => {
        	// EVMC_DELEGATECALL, EVMC_CALLCODE not supported yet
	        (vec![0; 0], gas, [0u8; ADDRESS_LENGTH], StatusCode::EVMC_REJECTED)
        }
      }
	}
}

impl<'vicinity, 'config, T: Config> StackStateT<'config> for VmStackState<'vicinity, 'config, T> {
	fn metadata(&self) -> &StackSubstateMetadata<'config> {
		self.substate.metadata()
	}

	fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
		self.substate.metadata_mut()
	}

	fn enter(&mut self, gas_limit: u64, is_static: bool) {
		self.substate.enter(gas_limit, is_static)
	}

	fn exit_commit(&mut self) -> Result<(), ExitError> {
		self.substate.exit_commit()
	}

	fn exit_revert(&mut self) -> Result<(), ExitError> {
		self.substate.exit_revert()
	}

	fn exit_discard(&mut self) -> Result<(), ExitError> {
		self.substate.exit_discard()
	}

	fn is_empty(&self, address: H160) -> bool {
		Module::<T>::is_account_empty(&address)
	}

	fn deleted(&self, address: H160) -> bool {
		self.substate.deleted(address)
	}

	fn inc_nonce(&mut self, address: H160) {
		let account_id = T::AddressMapping::into_account_id(address);
		frame_system::Module::<T>::inc_account_nonce(&account_id);
	}

	fn set_storage(&mut self, address: H160, index: H256, value: H256) {
		if value == H256::default() {
			debug::debug!(
				target: "evm",
				"Removing storage for {:?} [index: {:?}]",
				address,
				index,
			);
			AccountStorages::remove(address, index);
		} else {
			debug::debug!(
				target: "evm",
				"Updating storage for {:?} [index: {:?}, value: {:?}]",
				address,
				index,
				value,
			);
			AccountStorages::insert(address, index, value);
		}
	}

	fn reset_storage(&mut self, address: H160) {
		AccountStorages::remove_prefix(address);
	}

	fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
		self.substate.log(address, topics, data)
	}

	fn set_deleted(&mut self, address: H160) {
		self.substate.set_deleted(address)
	}

	fn set_code(&mut self, address: H160, code: Vec<u8>) {
		debug::debug!(
			target: "evm",
			"Inserting code ({} bytes) at {:?}",
			code.len(),
			address
		);
		Module::<T>::create_account(address, code);
	}

	fn transfer(&mut self, transfer: Transfer) -> Result<(), ExitError> {
		let source = T::AddressMapping::into_account_id(transfer.source);
		let target = T::AddressMapping::into_account_id(transfer.target);

		T::Currency::transfer(
			&source,
			&target,
			transfer.value.low_u128().unique_saturated_into(),
			ExistenceRequirement::AllowDeath,
		).map_err(|_| ExitError::OutOfFund)
	}

	fn reset_balance(&mut self, _address: H160) {
		// Do nothing on reset balance in Substrate.
		//
		// This function exists in EVM because a design issue
		// (arguably a bug) in SELFDESTRUCT that can cause total
		// issurance to be reduced. We do not need to replicate this.
	}

	fn touch(&mut self, _address: H160) {
		// Do nothing on touch in Substrate.
		//
		// EVM pallet considers all accounts to exist, and distinguish
		// only empty and non-empty accounts. This avoids many of the
		// subtle issues in EIP-161.
	}
}
