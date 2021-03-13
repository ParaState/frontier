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

#![cfg_attr(not(feature = "std"), no_std)]

mod precompile;

use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
use sp_std::vec::Vec;
use sp_std::convert::TryFrom;
use sp_core::{U256, H160};
pub use evm::{ExitReason, ExitSucceed, ExitError, ExitFatal, ExitRevert};
#[cfg(feature = "std")]
pub use ssvm::types::StatusCode;
pub use evm::backend::{Basic as Account, Log};
pub use precompile::{Precompile, PrecompileSet, LinearCostPrecompile};

#[derive(Clone, Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
/// External input from the transaction.
pub struct Vicinity {
	/// Current transaction gas price.
	pub gas_price: U256,
	/// Origin of the transaction.
	pub origin: H160,
}

/// EVNC status code
#[derive(Clone, Debug, Copy, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum EVMCStatusCode {
	EvmcSuccess,
	EvmcFailure,
	EvmcRevert,
	EvmcOutOfGas,
	EvmcInvalidInstruction,
	EvmcUndefinedInstruction,
	EvmcStackOverflow,
	EvmcStackUnderflow,
	EvmcBadJumpDestination,
	EvmcInvalidMemoryAccess,
	EvmcCallDepthExceeded,
	EvmcStaticModeViolation,
	EvmcPrecompileFailure,
	EvmcContractValidationFailure,
	EvmcArgumentOutOfRange,
	EvmcWasmUnreachableInstruction,
	EvmcWasmTrap,
	EvmcInternalError,
	EvmcRejected,
	EvmcOutOfMemory
}

#[cfg(feature = "std")]
impl TryFrom<EVMCStatusCode> for StatusCode {
	type Error = ();

	fn try_from(s: EVMCStatusCode) -> Result<Self, Self::Error> {
		match s {
			EVMCStatusCode::EvmcSuccess                    => Ok(StatusCode::EVMC_SUCCESS),
			EVMCStatusCode::EvmcFailure                    => Ok(StatusCode::EVMC_FAILURE),
			EVMCStatusCode::EvmcRevert                     => Ok(StatusCode::EVMC_REVERT),
			EVMCStatusCode::EvmcOutOfGas                   => Ok(StatusCode::EVMC_OUT_OF_GAS),
			EVMCStatusCode::EvmcInvalidInstruction         => Ok(StatusCode::EVMC_INVALID_INSTRUCTION),
			EVMCStatusCode::EvmcUndefinedInstruction       => Ok(StatusCode::EVMC_UNDEFINED_INSTRUCTION),
			EVMCStatusCode::EvmcStackOverflow              => Ok(StatusCode::EVMC_STACK_OVERFLOW),
			EVMCStatusCode::EvmcStackUnderflow             => Ok(StatusCode::EVMC_STACK_UNDERFLOW),
			EVMCStatusCode::EvmcBadJumpDestination         => Ok(StatusCode::EVMC_BAD_JUMP_DESTINATION),
			EVMCStatusCode::EvmcInvalidMemoryAccess        => Ok(StatusCode::EVMC_INVALID_MEMORY_ACCESS),
			EVMCStatusCode::EvmcCallDepthExceeded          => Ok(StatusCode::EVMC_CALL_DEPTH_EXCEEDED),
			EVMCStatusCode::EvmcStaticModeViolation        => Ok(StatusCode::EVMC_STATIC_MODE_VIOLATION),
			EVMCStatusCode::EvmcPrecompileFailure          => Ok(StatusCode::EVMC_PRECOMPILE_FAILURE),
			EVMCStatusCode::EvmcContractValidationFailure  => Ok(StatusCode::EVMC_CONTRACT_VALIDATION_FAILURE),
			EVMCStatusCode::EvmcArgumentOutOfRange         => Ok(StatusCode::EVMC_ARGUMENT_OUT_OF_RANGE),
			EVMCStatusCode::EvmcWasmUnreachableInstruction => Ok(StatusCode::EVMC_WASM_UNREACHABLE_INSTRUCTION),
			EVMCStatusCode::EvmcWasmTrap                   => Ok(StatusCode::EVMC_WASM_TRAP),
			EVMCStatusCode::EvmcInternalError              => Ok(StatusCode::EVMC_INTERNAL_ERROR),
			EVMCStatusCode::EvmcRejected                   => Ok(StatusCode::EVMC_REJECTED),
			EVMCStatusCode::EvmcOutOfMemory                => Ok(StatusCode::EVMC_OUT_OF_MEMORY),
		}
	}
}

#[cfg(feature = "std")]
	impl TryFrom<StatusCode> for EVMCStatusCode {
	type Error = ();

	fn try_from(s: StatusCode) -> Result<Self, Self::Error> {
		match s {
			StatusCode::EVMC_SUCCESS                      => Ok(EVMCStatusCode::EvmcSuccess),
			StatusCode::EVMC_FAILURE                      => Ok(EVMCStatusCode::EvmcFailure),
			StatusCode::EVMC_REVERT                       => Ok(EVMCStatusCode::EvmcRevert),
			StatusCode::EVMC_OUT_OF_GAS                   => Ok(EVMCStatusCode::EvmcOutOfGas),
			StatusCode::EVMC_INVALID_INSTRUCTION          => Ok(EVMCStatusCode::EvmcInvalidInstruction),
			StatusCode::EVMC_UNDEFINED_INSTRUCTION        => Ok(EVMCStatusCode::EvmcUndefinedInstruction),
			StatusCode::EVMC_STACK_OVERFLOW               => Ok(EVMCStatusCode::EvmcUndefinedInstruction),
			StatusCode::EVMC_STACK_UNDERFLOW              => Ok(EVMCStatusCode::EvmcStackUnderflow),
			StatusCode::EVMC_BAD_JUMP_DESTINATION         => Ok(EVMCStatusCode::EvmcBadJumpDestination),
			StatusCode::EVMC_INVALID_MEMORY_ACCESS        => Ok(EVMCStatusCode::EvmcInvalidMemoryAccess),
			StatusCode::EVMC_CALL_DEPTH_EXCEEDED          => Ok(EVMCStatusCode::EvmcCallDepthExceeded),
			StatusCode::EVMC_STATIC_MODE_VIOLATION        => Ok(EVMCStatusCode::EvmcStaticModeViolation),
			StatusCode::EVMC_PRECOMPILE_FAILURE           => Ok(EVMCStatusCode::EvmcPrecompileFailure),
			StatusCode::EVMC_CONTRACT_VALIDATION_FAILURE  => Ok(EVMCStatusCode::EvmcContractValidationFailure),
			StatusCode::EVMC_ARGUMENT_OUT_OF_RANGE        => Ok(EVMCStatusCode::EvmcArgumentOutOfRange),
			StatusCode::EVMC_WASM_UNREACHABLE_INSTRUCTION => Ok(EVMCStatusCode::EvmcWasmUnreachableInstruction),
			StatusCode::EVMC_WASM_TRAP                    => Ok(EVMCStatusCode::EvmcWasmTrap),
			StatusCode::EVMC_INTERNAL_ERROR               => Ok(EVMCStatusCode::EvmcInternalError),
			StatusCode::EVMC_REJECTED                     => Ok(EVMCStatusCode::EvmcRejected),
			StatusCode::EVMC_OUT_OF_MEMORY                => Ok(EVMCStatusCode::EvmcOutOfMemory),
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ExtendExitReason {
	ExitReason(ExitReason),
	EVMCStatusCode(EVMCStatusCode)
}

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct ExecutionInfo<T> {
	pub exit_reason: ExtendExitReason,
	pub value: T,
	pub used_gas: U256,
	pub logs: Vec<Log>,
}

pub type CallInfo = ExecutionInfo<Vec<u8>>;
pub type CreateInfo = ExecutionInfo<H160>;

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum CallOrCreateInfo {
	Call(CallInfo),
	Create(CreateInfo),
}
