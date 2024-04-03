use core::borrow::Borrow;
use p3_air::PairBuilder;
use p3_air::{Air, BaseAir};
use p3_field::AbstractField;
use p3_field::Field;
use p3_matrix::MatrixRowSlices;

use super::columns::{ByteMultCols, BytePreprocessedCols, NUM_BYTE_MULT_COLS};
use super::{ByteChip, ByteOpcode};
use crate::air::SP1AirBuilder;

/// The column map for the byte chip.
// pub(crate) const BYTE_COL_MAP: ByteCols<usize> = make_col_map();

/// The multiplicity indices for each byte operation.
// pub(crate) const BYTE_MULT_INDICES: [usize; NUM_BYTE_OPS] = BYTE_COL_MAP.multiplicities;

impl<F: Field> BaseAir<F> for ByteChip<F> {
    fn width(&self) -> usize {
        NUM_BYTE_MULT_COLS
    }
}

impl<AB: SP1AirBuilder + PairBuilder> Air<AB> for ByteChip<AB::F> {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local_mult: &ByteMultCols<AB::Var> = main.row_slice(0).borrow();

        let prep = builder.preprocessed();
        let local: &BytePreprocessedCols<AB::Var> = prep.row_slice(0).borrow();

        // Send all the lookups for each operation.
        for (i, opcode) in ByteOpcode::all().iter().enumerate() {
            let field_op = opcode.as_field::<AB::F>();
            let mult = local_mult.multiplicities[i];
            match opcode {
                ByteOpcode::AND => {
                    builder.receive_byte(field_op, local.and, local.b, local.c, mult)
                }
                ByteOpcode::OR => builder.receive_byte(field_op, local.or, local.b, local.c, mult),
                ByteOpcode::XOR => {
                    builder.receive_byte(field_op, local.xor, local.b, local.c, mult)
                }
                ByteOpcode::SLL => {
                    builder.receive_byte(field_op, local.sll, local.b, local.c, mult)
                }
                ByteOpcode::U8Range => {
                    builder.receive_byte(field_op, AB::F::zero(), local.b, local.c, mult)
                }
                ByteOpcode::ShrCarry => builder.receive_byte_pair(
                    field_op,
                    local.shr,
                    local.shr_carry,
                    local.b,
                    local.c,
                    mult,
                ),
                ByteOpcode::LTU => {
                    builder.receive_byte(field_op, local.ltu, local.b, local.c, mult)
                }
                ByteOpcode::MSB => {
                    builder.receive_byte(field_op, local.msb, local.b, AB::F::zero(), mult)
                }
                ByteOpcode::U16Range => builder.receive_byte(
                    field_op,
                    local.value_u16,
                    AB::F::zero(),
                    AB::F::zero(),
                    mult,
                ),
            }
        }

        // Dummy constraint for normalizing to degree 3.
        #[allow(clippy::eq_op)]
        builder.assert_zero(local.b * local.b * local.b - local.b * local.b * local.b);
    }
}
