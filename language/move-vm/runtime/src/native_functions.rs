// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{interpreter::Interpreter, loader::Resolver, logging::LogContext};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress, gas_schedule::CostTable, language_storage::CORE_CODE_ADDRESS,
    value::MoveTypeLayout, vm_status::StatusType,
};
use move_vm_natives::{
    account, bcs, debug, event, hash, ol_decimal, ol_eth_signature, ol_hash, signature, signer,
    vdf, vector,
}; //////// 0L ////////
use move_vm_types::{
    data_store::DataStore,
    gas_schedule::GasStatus,
    loaded_data::runtime_types::Type,
    natives::function::{NativeContext, NativeResult},
    values::Value,
};
use std::{collections::VecDeque, fmt::Write};

// The set of native functions the VM supports.
// The functions can line in any crate linked in but the VM declares them here.
// 2 functions have to be implemented for a `NativeFunction`:
// - `resolve` which given a function unique name ModuleAddress::ModuleName::FunctionName
// returns a `NativeFunction`
// - `dispatch` which given a `NativeFunction` invokes the native
#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeFunction {
    HashSha2_256,
    HashSha3_256,
    BCSToBytes,
    PubED25519Validate,
    SigED25519Verify,
    VectorLength,
    VectorEmpty,
    VectorBorrow,
    VectorBorrowMut,
    VectorPushBack,
    VectorPopBack,
    VectorDestroyEmpty,
    VectorSwap,
    AccountWriteEvent,
    DebugPrint,
    DebugPrintStackTrace,
    SignerBorrowAddress,
    CreateSigner,
    // functions below this line are deprecated and remain only for replaying old transactions
    DestroySigner,
    //////// 0L ////////
    VDFVerify,
    RedeemAuthKeyParse,
    DecimalDemo,
    DecimalSingle,
    DecimalPair,
    HashKeccak256,
    EthSignatureRecover,
    EthSignatureVerify,
}

impl NativeFunction {
    pub(crate) fn resolve(
        module_address: &AccountAddress,
        module_name: &str,
        function_name: &str,
    ) -> Option<NativeFunction> {
        use NativeFunction::*;

        let case = (module_address, module_name, function_name);
        Some(match case {
            (&CORE_CODE_ADDRESS, "Hash", "sha2_256") => HashSha2_256,
            (&CORE_CODE_ADDRESS, "Hash", "sha3_256") => HashSha3_256,
            (&CORE_CODE_ADDRESS, "BCS", "to_bytes") => BCSToBytes,
            (&CORE_CODE_ADDRESS, "Signature", "ed25519_validate_pubkey") => PubED25519Validate,
            (&CORE_CODE_ADDRESS, "Signature", "ed25519_verify") => SigED25519Verify,
            (&CORE_CODE_ADDRESS, "Vector", "length") => VectorLength,
            (&CORE_CODE_ADDRESS, "Vector", "empty") => VectorEmpty,
            (&CORE_CODE_ADDRESS, "Vector", "borrow") => VectorBorrow,
            (&CORE_CODE_ADDRESS, "Vector", "borrow_mut") => VectorBorrowMut,
            (&CORE_CODE_ADDRESS, "Vector", "push_back") => VectorPushBack,
            (&CORE_CODE_ADDRESS, "Vector", "pop_back") => VectorPopBack,
            (&CORE_CODE_ADDRESS, "Vector", "destroy_empty") => VectorDestroyEmpty,
            (&CORE_CODE_ADDRESS, "Vector", "swap") => VectorSwap,
            (&CORE_CODE_ADDRESS, "Event", "write_to_event_store") => AccountWriteEvent,
            (&CORE_CODE_ADDRESS, "DiemAccount", "create_signer") => CreateSigner,
            (&CORE_CODE_ADDRESS, "Debug", "print") => DebugPrint,
            (&CORE_CODE_ADDRESS, "Debug", "print_stack_trace") => DebugPrintStackTrace,
            (&CORE_CODE_ADDRESS, "Signer", "borrow_address") => SignerBorrowAddress,
            // functions below this line are deprecated and remain only for replaying old transactions
            (&CORE_CODE_ADDRESS, "DiemAccount", "destroy_signer") => DestroySigner,
            //////// 0L ////////
            (&CORE_CODE_ADDRESS, "VDF", "verify") => VDFVerify,
            (&CORE_CODE_ADDRESS, "VDF", "extract_address_from_challenge") => RedeemAuthKeyParse,
            (&CORE_CODE_ADDRESS, "Decimal", "decimal_demo") => DecimalDemo,
            (&CORE_CODE_ADDRESS, "Decimal", "single_op") => DecimalSingle,
            (&CORE_CODE_ADDRESS, "Decimal", "pair_op") => DecimalPair,
            (&CORE_CODE_ADDRESS, "XHash", "keccak_256") => HashKeccak256,
            (&CORE_CODE_ADDRESS, "EthSignature", "recover") => EthSignatureRecover,
            (&CORE_CODE_ADDRESS, "EthSignature", "verify") => EthSignatureVerify,

            _ => return None,
        })
    }

    /// Given the vector of aguments, it executes the native function.
    pub(crate) fn dispatch(
        self,
        ctx: &mut impl NativeContext,
        t: Vec<Type>,
        v: VecDeque<Value>,
    ) -> PartialVMResult<NativeResult> {
        let result = match self {
            Self::HashSha2_256 => hash::native_sha2_256(ctx, t, v),
            Self::HashSha3_256 => hash::native_sha3_256(ctx, t, v),
            Self::PubED25519Validate => signature::native_ed25519_publickey_validation(ctx, t, v),
            Self::SigED25519Verify => signature::native_ed25519_signature_verification(ctx, t, v),
            Self::VectorLength => vector::native_length(ctx, t, v),
            Self::VectorEmpty => vector::native_empty(ctx, t, v),
            Self::VectorBorrow => vector::native_borrow(ctx, t, v),
            Self::VectorBorrowMut => vector::native_borrow(ctx, t, v),
            Self::VectorPushBack => vector::native_push_back(ctx, t, v),
            Self::VectorPopBack => vector::native_pop(ctx, t, v),
            Self::VectorDestroyEmpty => vector::native_destroy_empty(ctx, t, v),
            Self::VectorSwap => vector::native_swap(ctx, t, v),
            // natives that need the full API of `NativeContext`
            Self::AccountWriteEvent => event::native_emit_event(ctx, t, v),
            Self::BCSToBytes => bcs::native_to_bytes(ctx, t, v),
            Self::DebugPrint => debug::native_print(ctx, t, v),
            Self::DebugPrintStackTrace => debug::native_print_stack_trace(ctx, t, v),
            Self::SignerBorrowAddress => signer::native_borrow_address(ctx, t, v),
            Self::CreateSigner => account::native_create_signer(ctx, t, v),
            // functions below this line are deprecated and remain only for replaying old transactions
            Self::DestroySigner => account::native_destroy_signer(ctx, t, v),
            //////// 0L ////////
            Self::VDFVerify => vdf::verify(ctx, t, v),
            Self::RedeemAuthKeyParse => vdf::extract_address_from_challenge(ctx, t, v),
            Self::DecimalDemo => ol_decimal::native_decimal_demo(ctx, t, v),
            Self::DecimalSingle => ol_decimal::native_decimal_single(ctx, t, v),
            Self::DecimalPair => ol_decimal::native_decimal_pair(ctx, t, v),
            Self::HashKeccak256 => ol_hash::native_keccak_256(ctx, t, v),
            Self::EthSignatureRecover => ol_eth_signature::native_eth_signature_recover(ctx, t, v),
            Self::EthSignatureVerify => ol_eth_signature::native_eth_signature_verify(ctx, t, v),
        };
        debug_assert!(match &result {
            Err(e) => e.major_status().status_type() == StatusType::InvariantViolation,
            Ok(_) => true,
        });
        result
    }
}

pub(crate) struct FunctionContext<'a, L: LogContext> {
    interpreter: &'a mut Interpreter<L>,
    data_store: &'a mut dyn DataStore,
    gas_status: &'a GasStatus<'a>,
    resolver: &'a Resolver<'a>,
}

impl<'a, L: LogContext> FunctionContext<'a, L> {
    pub(crate) fn new(
        interpreter: &'a mut Interpreter<L>,
        data_store: &'a mut dyn DataStore,
        gas_status: &'a mut GasStatus,
        resolver: &'a Resolver<'a>,
    ) -> FunctionContext<'a, L> {
        FunctionContext {
            interpreter,
            data_store,
            gas_status,
            resolver,
        }
    }
}

impl<'a, L: LogContext> NativeContext for FunctionContext<'a, L> {
    fn print_stack_trace<B: Write>(&self, buf: &mut B) -> PartialVMResult<()> {
        self.interpreter
            .debug_print_stack_trace(buf, self.resolver.loader())
    }

    fn cost_table(&self) -> &CostTable {
        self.gas_status.cost_table()
    }

    fn save_event(
        &mut self,
        guid: Vec<u8>,
        seq_num: u64,
        ty: Type,
        val: Value,
    ) -> PartialVMResult<bool> {
        match self.data_store.emit_event(guid, seq_num, ty, val) {
            Ok(()) => Ok(true),
            Err(e) if e.major_status().status_type() == StatusType::InvariantViolation => Err(e),
            Err(_) => Ok(false),
        }
    }

    fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<Option<MoveTypeLayout>> {
        match self.resolver.type_to_type_layout(ty) {
            Ok(ty_layout) => Ok(Some(ty_layout)),
            Err(e) if e.major_status().status_type() == StatusType::InvariantViolation => Err(e),
            Err(_) => Ok(None),
        }
    }
}
