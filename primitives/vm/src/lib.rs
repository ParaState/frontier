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
use sp_std::convert::From;
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
impl From<EVMCStatusCode> for StatusCode {
	fn from(s: EVMCStatusCode) -> Self {
		match s {
			EVMCStatusCode::EvmcSuccess                    => StatusCode::EVMC_SUCCESS,
			EVMCStatusCode::EvmcFailure                    => StatusCode::EVMC_FAILURE,
			EVMCStatusCode::EvmcRevert                     => StatusCode::EVMC_REVERT,
			EVMCStatusCode::EvmcOutOfGas                   => StatusCode::EVMC_OUT_OF_GAS,
			EVMCStatusCode::EvmcInvalidInstruction         => StatusCode::EVMC_INVALID_INSTRUCTION,
			EVMCStatusCode::EvmcUndefinedInstruction       => StatusCode::EVMC_UNDEFINED_INSTRUCTION,
			EVMCStatusCode::EvmcStackOverflow              => StatusCode::EVMC_STACK_OVERFLOW,
			EVMCStatusCode::EvmcStackUnderflow             => StatusCode::EVMC_STACK_UNDERFLOW,
			EVMCStatusCode::EvmcBadJumpDestination         => StatusCode::EVMC_BAD_JUMP_DESTINATION,
			EVMCStatusCode::EvmcInvalidMemoryAccess        => StatusCode::EVMC_INVALID_MEMORY_ACCESS,
			EVMCStatusCode::EvmcCallDepthExceeded          => StatusCode::EVMC_CALL_DEPTH_EXCEEDED,
			EVMCStatusCode::EvmcStaticModeViolation        => StatusCode::EVMC_STATIC_MODE_VIOLATION,
			EVMCStatusCode::EvmcPrecompileFailure          => StatusCode::EVMC_PRECOMPILE_FAILURE,
			EVMCStatusCode::EvmcContractValidationFailure  => StatusCode::EVMC_CONTRACT_VALIDATION_FAILURE,
			EVMCStatusCode::EvmcArgumentOutOfRange         => StatusCode::EVMC_ARGUMENT_OUT_OF_RANGE,
			EVMCStatusCode::EvmcWasmUnreachableInstruction => StatusCode::EVMC_WASM_UNREACHABLE_INSTRUCTION,
			EVMCStatusCode::EvmcWasmTrap                   => StatusCode::EVMC_WASM_TRAP,
			EVMCStatusCode::EvmcInternalError              => StatusCode::EVMC_INTERNAL_ERROR,
			EVMCStatusCode::EvmcRejected                   => StatusCode::EVMC_REJECTED,
			EVMCStatusCode::EvmcOutOfMemory                => StatusCode::EVMC_OUT_OF_MEMORY,
		}
	}
}

#[cfg(feature = "std")]
impl From<StatusCode> for EVMCStatusCode {
	fn from(s: StatusCode) -> Self {
		match s {
			StatusCode::EVMC_SUCCESS                      => EVMCStatusCode::EvmcSuccess,
			StatusCode::EVMC_FAILURE                      => EVMCStatusCode::EvmcFailure,
			StatusCode::EVMC_REVERT                       => EVMCStatusCode::EvmcRevert,
			StatusCode::EVMC_OUT_OF_GAS                   => EVMCStatusCode::EvmcOutOfGas,
			StatusCode::EVMC_INVALID_INSTRUCTION          => EVMCStatusCode::EvmcInvalidInstruction,
			StatusCode::EVMC_UNDEFINED_INSTRUCTION        => EVMCStatusCode::EvmcUndefinedInstruction,
			StatusCode::EVMC_STACK_OVERFLOW               => EVMCStatusCode::EvmcUndefinedInstruction,
			StatusCode::EVMC_STACK_UNDERFLOW              => EVMCStatusCode::EvmcStackUnderflow,
			StatusCode::EVMC_BAD_JUMP_DESTINATION         => EVMCStatusCode::EvmcBadJumpDestination,
			StatusCode::EVMC_INVALID_MEMORY_ACCESS        => EVMCStatusCode::EvmcInvalidMemoryAccess,
			StatusCode::EVMC_CALL_DEPTH_EXCEEDED          => EVMCStatusCode::EvmcCallDepthExceeded,
			StatusCode::EVMC_STATIC_MODE_VIOLATION        => EVMCStatusCode::EvmcStaticModeViolation,
			StatusCode::EVMC_PRECOMPILE_FAILURE           => EVMCStatusCode::EvmcPrecompileFailure,
			StatusCode::EVMC_CONTRACT_VALIDATION_FAILURE  => EVMCStatusCode::EvmcContractValidationFailure,
			StatusCode::EVMC_ARGUMENT_OUT_OF_RANGE        => EVMCStatusCode::EvmcArgumentOutOfRange,
			StatusCode::EVMC_WASM_UNREACHABLE_INSTRUCTION => EVMCStatusCode::EvmcWasmUnreachableInstruction,
			StatusCode::EVMC_WASM_TRAP                    => EVMCStatusCode::EvmcWasmTrap,
			StatusCode::EVMC_INTERNAL_ERROR               => EVMCStatusCode::EvmcInternalError,
			StatusCode::EVMC_REJECTED                     => EVMCStatusCode::EvmcRejected,
			StatusCode::EVMC_OUT_OF_MEMORY                => EVMCStatusCode::EvmcOutOfMemory,
		}
	}
}

impl From<ExtendExitReason> for ExitReason {
	fn from(s: ExtendExitReason) -> Self {
		match s {
			ExtendExitReason::ExitReason(reason) => reason,
			ExtendExitReason::EVMCStatusCode(status) => {
				match status {
					EVMCStatusCode::EvmcSuccess                    => ExitReason::Succeed(ExitSucceed::Returned),
					EVMCStatusCode::EvmcFailure                    => ExitReason::Fatal(ExitFatal::Other("Evmc Failure".into())),
					EVMCStatusCode::EvmcRevert                     => ExitReason::Revert(ExitRevert::Reverted),
					EVMCStatusCode::EvmcOutOfGas                   => ExitReason::Error(ExitError::OutOfGas),
					EVMCStatusCode::EvmcInvalidInstruction         => ExitReason::Error(ExitError::DesignatedInvalid),
					EVMCStatusCode::EvmcUndefinedInstruction       => ExitReason::Fatal(ExitFatal::NotSupported),
					EVMCStatusCode::EvmcStackOverflow              => ExitReason::Error(ExitError::StackOverflow),
					EVMCStatusCode::EvmcStackUnderflow             => ExitReason::Error(ExitError::StackUnderflow),
					EVMCStatusCode::EvmcBadJumpDestination         => ExitReason::Error(ExitError::InvalidJump),
					EVMCStatusCode::EvmcInvalidMemoryAccess        => ExitReason::Error(ExitError::InvalidRange),
					EVMCStatusCode::EvmcCallDepthExceeded          => ExitReason::Error(ExitError::CallTooDeep),
					EVMCStatusCode::EvmcStaticModeViolation        => ExitReason::Error(ExitError::Other("Evmc Static Mode Violation".into())),
					EVMCStatusCode::EvmcPrecompileFailure          => ExitReason::Error(ExitError::Other("Evmc Precompile Failure".into())),
					EVMCStatusCode::EvmcContractValidationFailure  => ExitReason::Error(ExitError::Other("Evmc Contract Validation Failure".into())),
					EVMCStatusCode::EvmcArgumentOutOfRange         => ExitReason::Error(ExitError::Other("Evmc Argument Out Of Range".into())),
					EVMCStatusCode::EvmcWasmUnreachableInstruction => ExitReason::Fatal(ExitFatal::UnhandledInterrupt),
					EVMCStatusCode::EvmcWasmTrap                   => ExitReason::Fatal(ExitFatal::UnhandledInterrupt),
					EVMCStatusCode::EvmcInternalError              => ExitReason::Error(ExitError::Other("Evmc Internal Error".into())),
					EVMCStatusCode::EvmcRejected                   => ExitReason::Error(ExitError::Other("Evmc Rejected".into())),
					EVMCStatusCode::EvmcOutOfMemory                => ExitReason::Error(ExitError::Other("Evmc Out Of Memory".into())),
				}
			}
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
