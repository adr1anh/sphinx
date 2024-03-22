use crate::air::BaseAirBuilder;
use crate::air::MachineAir;
use crate::air::SP1AirBuilder;
use crate::memory::MemoryReadCols;
use crate::memory::MemoryWriteCols;
use crate::operations::field::field_op::FieldOpCols;
use crate::operations::field::field_op::FieldOperation;
use crate::operations::field::field_sqrt::FieldSqrtCols;
use crate::runtime::ExecutionRecord;
use crate::runtime::MemoryReadRecord;
use crate::runtime::MemoryWriteRecord;
use crate::runtime::Syscall;
use crate::runtime::SyscallCode;
use crate::syscall::precompiles::SyscallContext;
use crate::utils::bytes_to_words_le;
use crate::utils::ec::edwards::ed25519::decompress;
use crate::utils::ec::edwards::ed25519::ed25519_sqrt;
use crate::utils::ec::edwards::EdwardsParameters;
use crate::utils::ec::field::FieldParameters;
use crate::utils::ec::COMPRESSED_POINT_BYTES;
use crate::utils::ec::NUM_BYTES_FIELD_ELEMENT;
use crate::utils::ec::NUM_WORDS_FIELD_ELEMENT;
use crate::utils::limbs_from_access;
use crate::utils::limbs_from_prev_access;
use crate::utils::pad_rows;
use crate::utils::words_to_bytes_le;
use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;
use curve25519_dalek::edwards::CompressedEdwardsY;
use num::BigUint;
use num::One;
use num::Zero;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_field::PrimeField32;
use p3_matrix::MatrixRowSlices;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

use p3_matrix::dense::RowMajorMatrix;
use sp1_derive::AlignedBorrow;
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdDecompressEvent {
    pub shard: u32,
    pub clk: u32,
    pub ptr: u32,
    pub sign: bool,
    pub y_bytes: [u8; COMPRESSED_POINT_BYTES],
    pub decompressed_x_bytes: [u8; NUM_BYTES_FIELD_ELEMENT],
    pub x_memory_records: [MemoryWriteRecord; NUM_WORDS_FIELD_ELEMENT],
    pub y_memory_records: [MemoryReadRecord; NUM_WORDS_FIELD_ELEMENT],
}

pub const NUM_ED_DECOMPRESS_COLS: usize = size_of::<EdDecompressCols<u8>>();

/// A set of columns to compute `EdDecompress` given a pointer to a 16 word slice formatted as such:
/// The 31st byte of the slice is the sign bit. The second half of the slice is the 255-bit
/// compressed Y (without sign bit).
///
/// After `EdDecompress`, the first 32 bytes of the slice are overwritten with the decompressed X.
#[derive(Debug, Clone, AlignedBorrow)]
#[repr(C)]
pub struct EdDecompressCols<T> {
    pub is_real: T,
    pub shard: T,
    pub clk: T,
    pub ptr: T,
    pub sign: T,
    pub x_access: [MemoryWriteCols<T>; NUM_WORDS_FIELD_ELEMENT],
    pub y_access: [MemoryReadCols<T>; NUM_WORDS_FIELD_ELEMENT],
    pub(crate) yy: FieldOpCols<T>,
    pub(crate) u: FieldOpCols<T>,
    pub(crate) dyy: FieldOpCols<T>,
    pub(crate) v: FieldOpCols<T>,
    pub(crate) u_div_v: FieldOpCols<T>,
    pub(crate) x: FieldSqrtCols<T>,
    pub(crate) neg_x: FieldOpCols<T>,
}

impl<F: PrimeField32> EdDecompressCols<F> {
    pub fn populate<P: FieldParameters, E: EdwardsParameters>(
        &mut self,
        event: EdDecompressEvent,
        record: &mut ExecutionRecord,
    ) {
        let mut new_byte_lookup_events = Vec::new();
        self.is_real = F::from_bool(true);
        self.shard = F::from_canonical_u32(event.shard);
        self.clk = F::from_canonical_u32(event.clk);
        self.ptr = F::from_canonical_u32(event.ptr);
        self.sign = F::from_bool(event.sign);
        for i in 0..8 {
            self.x_access[i].populate(event.x_memory_records[i], &mut new_byte_lookup_events);
            self.y_access[i].populate(event.y_memory_records[i], &mut new_byte_lookup_events);
        }

        let y = &BigUint::from_bytes_le(&event.y_bytes);
        self.populate_field_ops::<P, E>(y);

        record.add_byte_lookup_events(new_byte_lookup_events);
    }

    fn populate_field_ops<P: FieldParameters, E: EdwardsParameters>(&mut self, y: &BigUint) {
        let one = BigUint::one();
        let yy = self.yy.populate::<P>(y, y, FieldOperation::Mul);
        let u = self.u.populate::<P>(&yy, &one, FieldOperation::Sub);
        let dyy = self
            .dyy
            .populate::<P>(&E::d_biguint(), &yy, FieldOperation::Mul);
        let v = self.v.populate::<P>(&one, &dyy, FieldOperation::Add);
        let u_div_v = self.u_div_v.populate::<P>(&u, &v, FieldOperation::Div);
        let x = self.x.populate::<P>(&u_div_v, ed25519_sqrt);
        self.neg_x
            .populate::<P>(&BigUint::zero(), &x, FieldOperation::Sub);
    }
}

impl<V: Copy> EdDecompressCols<V> {
    pub fn eval<AB: SP1AirBuilder<Var = V>, P: FieldParameters, E: EdwardsParameters>(
        &self,
        builder: &mut AB,
    ) where
        V: Into<AB::Expr>,
    {
        builder.assert_bool(self.sign);

        let y = limbs_from_prev_access(&self.y_access);
        self.yy
            .eval::<AB, P, _, _>(builder, &y, &y, FieldOperation::Mul);
        self.u.eval::<AB, P, _, _>(
            builder,
            &self.yy.result,
            &[AB::Expr::one()].iter(),
            FieldOperation::Sub,
        );
        let d_biguint = E::d_biguint();
        let d_const = E::BaseField::to_limbs_field::<AB::F>(&d_biguint);
        self.dyy
            .eval::<AB, P, _, _>(builder, &d_const, &self.yy.result, FieldOperation::Mul);
        self.v.eval::<AB, P, _, _>(
            builder,
            &[AB::Expr::one()].iter(),
            &self.dyy.result,
            FieldOperation::Add,
        );
        self.u_div_v.eval::<AB, P, _, _>(
            builder,
            &self.u.result,
            &self.v.result,
            FieldOperation::Div,
        );
        self.x.eval::<AB, P>(builder, &self.u_div_v.result);
        self.neg_x.eval::<AB, P, _, _>(
            builder,
            &[AB::Expr::zero()].iter(),
            &self.x.multiplication.result,
            FieldOperation::Sub,
        );

        for i in 0..NUM_WORDS_FIELD_ELEMENT {
            builder.constraint_memory_access(
                self.shard,
                self.clk,
                self.ptr.into() + AB::F::from_canonical_u32((i as u32) * 4),
                &self.x_access[i],
                self.is_real,
            );
        }
        for i in 0..NUM_WORDS_FIELD_ELEMENT {
            builder.constraint_memory_access(
                self.shard,
                self.clk,
                self.ptr.into() + AB::F::from_canonical_u32((i as u32) * 4 + 32),
                &self.y_access[i],
                self.is_real,
            );
        }

        let x_limbs = limbs_from_access(&self.x_access);
        builder
            .when(self.is_real)
            .when(self.sign)
            .assert_all_eq(self.neg_x.result, x_limbs);
        builder
            .when(self.is_real)
            .when_not(self.sign)
            .assert_all_eq(self.x.multiplication.result, x_limbs);

        builder.receive_syscall(
            self.shard,
            self.clk,
            AB::F::from_canonical_u32(SyscallCode::ED_DECOMPRESS.syscall_id()),
            self.ptr,
            self.sign,
            self.is_real,
        );
    }
}

#[derive(Default)]
pub struct EdDecompressChip<E> {
    _phantom: PhantomData<E>,
}

impl<E: EdwardsParameters> Syscall for EdDecompressChip<E> {
    fn execute(&self, rt: &mut SyscallContext<'_>, arg1: u32, sign: u32) -> Option<u32> {
        let start_clk = rt.clk;
        let slice_ptr = arg1;
        assert!(slice_ptr % 4 == 0, "Pointer must be 4-byte aligned.");
        assert!(sign <= 1, "Sign bit must be 0 or 1.");
        let sign = sign as u8;

        let (y_memory_records_vec, y_vec) = rt.mr_slice(
            slice_ptr + (COMPRESSED_POINT_BYTES as u32),
            NUM_WORDS_FIELD_ELEMENT,
        );
        let y_memory_records: [MemoryReadRecord; 8] = y_memory_records_vec.try_into().unwrap();

        let sign_bool = sign != 0;

        let y_bytes: [u8; COMPRESSED_POINT_BYTES] = words_to_bytes_le(&y_vec);

        // Copy bytes into another array so we can modify the last byte and make CompressedEdwardsY,
        // which we'll use to compute the expected X.
        // Re-insert sign bit into last bit of Y for CompressedEdwardsY format
        let mut compressed_edwards_y: [u8; COMPRESSED_POINT_BYTES] = y_bytes;
        compressed_edwards_y[compressed_edwards_y.len() - 1] &= 0b0111_1111;
        compressed_edwards_y[compressed_edwards_y.len() - 1] |= sign << 7;

        // Compute actual decompressed X
        let compressed_y = CompressedEdwardsY(compressed_edwards_y);
        let decompressed = decompress(&compressed_y);

        let mut decompressed_x_bytes = decompressed.x.to_bytes_le();
        decompressed_x_bytes.resize(32, 0u8);
        let decompressed_x_words: [u32; NUM_WORDS_FIELD_ELEMENT] =
            bytes_to_words_le(&decompressed_x_bytes);

        // Write decompressed X into slice
        let x_memory_records_vec = rt.mw_slice(slice_ptr, &decompressed_x_words);
        let x_memory_records: [MemoryWriteRecord; 8] = x_memory_records_vec.try_into().unwrap();

        let shard = rt.current_shard();
        rt.record_mut()
            .ed_decompress_events
            .push(EdDecompressEvent {
                shard,
                clk: start_clk,
                ptr: slice_ptr,
                sign: sign_bool,
                y_bytes,
                decompressed_x_bytes: decompressed_x_bytes.try_into().unwrap(),
                x_memory_records,
                y_memory_records,
            });
        None
    }

    fn num_extra_cycles(&self) -> u32 {
        0
    }
}

impl<E: EdwardsParameters> EdDecompressChip<E> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<F: PrimeField32, E: EdwardsParameters> MachineAir<F> for EdDecompressChip<E> {
    type Record = ExecutionRecord;

    fn name(&self) -> String {
        "EdDecompress".to_string()
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        for i in 0..input.ed_decompress_events.len() {
            let event = &input.ed_decompress_events[i];
            let mut row = [F::zero(); NUM_ED_DECOMPRESS_COLS];
            let cols: &mut EdDecompressCols<F> = row.as_mut_slice().borrow_mut();
            cols.populate::<E::BaseField, E>(event.clone(), output);

            rows.push(row);
        }

        pad_rows(&mut rows, || {
            let mut row = [F::zero(); NUM_ED_DECOMPRESS_COLS];
            let cols: &mut EdDecompressCols<F> = row.as_mut_slice().borrow_mut();
            let zero = BigUint::zero();
            cols.populate_field_ops::<E::BaseField, E>(&zero);
            row
        });

        RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_ED_DECOMPRESS_COLS,
        )
    }

    fn included(&self, shard: &Self::Record) -> bool {
        !shard.ed_decompress_events.is_empty()
    }
}

impl<F, E: EdwardsParameters> BaseAir<F> for EdDecompressChip<E> {
    fn width(&self) -> usize {
        NUM_ED_DECOMPRESS_COLS
    }
}

impl<AB, E: EdwardsParameters> Air<AB> for EdDecompressChip<E>
where
    AB: SP1AirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let row: &EdDecompressCols<AB::Var> = main.row_slice(0).borrow();
        row.eval::<AB, E::BaseField, E>(builder);
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        runtime::Program,
        utils::{self, tests::ED_DECOMPRESS_ELF},
    };

    #[test]
    fn test_ed_decompress() {
        utils::setup_logger();
        let program = Program::from(ED_DECOMPRESS_ELF);
        utils::run_test(program).unwrap();
    }
}
