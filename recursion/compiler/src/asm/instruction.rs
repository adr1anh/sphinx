use alloc::collections::BTreeMap;
use alloc::format;
use core::fmt;
use wp1_recursion_core::cpu::Instruction;
use wp1_recursion_core::runtime::Opcode;

use crate::util::canonical_i32_to_field;
use p3_field::{ExtensionField, PrimeField32};

use super::ZERO;

#[derive(Debug, Clone)]
pub enum AsmInstruction<F, EF> {
    // Field operations
    /// Load work (dst, src, index, offset, size) : load a value from the address stored at src(fp) into dstfp).
    LW(i32, i32, i32, F, F),
    LWI(i32, i32, F, F, F),
    /// Store word (dst, src, index, offset, size) : store a value from src(fp) into the address stored at dest(fp).
    SW(i32, i32, i32, F, F),
    SWI(i32, i32, F, F, F),
    // Get immediate (dst, value) : load a value into the dest(fp).
    IMM(i32, F),
    /// Add, dst = lhs + rhs.
    ADD(i32, i32, i32),
    /// Add immediate, dst = lhs + rhs.
    ADDI(i32, i32, F),
    /// Subtract, dst = lhs - rhs.
    SUB(i32, i32, i32),
    /// Subtract immediate, dst = lhs - rhs.
    SUBI(i32, i32, F),
    /// Subtract value from immediate, dst = lhs - rhs.
    SUBIN(i32, F, i32),
    /// Multiply, dst = lhs * rhs.
    MUL(i32, i32, i32),
    /// Multiply immediate.
    MULI(i32, i32, F),
    /// Divide, dst = lhs / rhs.
    DIV(i32, i32, i32),
    /// Divide immediate, dst = lhs / rhs.
    DIVI(i32, i32, F),
    /// Divide value from immediate, dst = lhs / rhs.
    DIVIN(i32, F, i32),

    // Extension operations
    /// Load an ext value (dst, src, index, offset, size) : load a value from the address stored at src(fp) into dst(fp).
    LE(i32, i32, i32, F, F),
    LEI(i32, i32, F, F, F),
    /// Store an ext value (dst, src, index, offset, size) : store a value from src(fp) into address stored at dst(fp).
    SE(i32, i32, i32, F, F),
    SEI(i32, i32, F, F, F),
    /// Get immediate extension value (dst, value) : load a value into the dest(fp).
    EIMM(i32, EF),
    /// Add extension, dst = lhs + rhs.
    EADD(i32, i32, i32),
    /// Add immediate extension, dst = lhs + rhs.
    EADDI(i32, i32, EF),
    /// Subtract extension, dst = lhs - rhs.
    ESUB(i32, i32, i32),
    /// Subtract immediate extension, dst = lhs - rhs.
    ESUBI(i32, i32, EF),
    /// Subtract value from immediate extension, dst = lhs - rhs.
    ESUBIN(i32, EF, i32),
    /// Multiply extension, dst = lhs * rhs.
    EMUL(i32, i32, i32),
    /// Multiply immediate extension.
    EMULI(i32, i32, EF),
    /// Divide extension, dst = lhs / rhs.
    EDIV(i32, i32, i32),
    /// Divide immediate extension, dst = lhs / rhs.
    EDIVI(i32, i32, EF),
    /// Divide value from immediate extension, dst = lhs / rhs.
    EDIVIN(i32, EF, i32),

    // Mixed base-extension operations
    /// Add base to extension, dst = lhs + rhs.
    EADDF(i32, i32, i32),
    /// Add immediate base to extension, dst = lhs + rhs.
    EADDFI(i32, i32, F),
    /// Add immediate extension element to base, dst = lhs + rhs.
    FADDEI(i32, i32, EF),
    // Subtract base from extension, dst = lhs - rhs.
    ESUBF(i32, i32, i32),
    /// Subtract immediate base from extension, dst = lhs - rhs.
    ESUBFI(i32, i32, F),
    /// Subtract value from immediate base to extension, dst = lhs - rhs.
    ESUBFIN(i32, F, i32),
    /// Subtract extension from base, dst = lhs - rhs.
    FSUBE(i32, i32, i32),
    /// Subtract immediate extension from base, dst = lhs - rhs.
    FSUBEI(i32, i32, EF),
    /// Subtract value from immediate extension to base, dst = lhs - rhs.
    FSUBEIN(i32, EF, i32),
    /// Multiply base and extension, dst = lhs * rhs.
    EMULF(i32, i32, i32),
    /// Multiply immediate base and extension.
    EMULFI(i32, i32, F),
    /// Multiply base by immediate extension, dst = lhs * rhs.
    FMULEI(i32, i32, EF),
    /// Divide base and extension, dst = lhs / rhs.
    EDIVF(i32, i32, i32),
    /// Divide immediate base and extension, dst = lhs / rhs.
    EDIVFI(i32, i32, F),
    /// Divide value from immediate base to extension, dst = lhs / rhs.
    EDIVFIN(i32, F, i32),
    /// Divide extension from immediate base, dst = lhs / rhs.
    FDIVI(i32, i32, EF),
    /// Divide value from immediate extension to base, dst = lhs / rhs.
    FDIVIN(i32, EF, i32),

    /// Jump and link
    JAL(i32, F, F),
    /// Jump and link value
    JALR(i32, i32, i32),
    /// Branch not equal
    BNE(F, i32, i32),
    /// Branch not equal increment c by 1.
    BNEINC(F, i32, i32),
    /// Branch not equal immediate
    BNEI(F, i32, F),
    BNEIINC(F, i32, F),
    /// Branch equal
    BEQ(F, i32, i32),
    /// Branch equal immediate
    BEQI(F, i32, F),
    /// Branch not equal extension
    EBNE(F, i32, i32),
    /// Branch not equal immediate extension
    EBNEI(F, i32, EF),
    /// Branch equal extension
    EBEQ(F, i32, i32),
    /// Branch equal immediate extension
    EBEQI(F, i32, EF),
    /// Trap
    TRAP,
    /// Break(label)
    Break(F),

    // HintBits(dst, src) Decompose the field element `src` into bits and write them to the array
    // starting at the address stored at `dst`.
    HintBits(i32, i32),

    /// Perform a permutation of the Poseidon2 hash function on the array specified by the ptr.
    Poseidon2Permute(i32, i32),
    Poseidon2Compress(i32, i32, i32),

    PrintV(i32),
    PrintF(i32),
    PrintE(i32),
    Ext2Felt(i32, i32),

    HintLen(i32),
    Hint(i32),
    // FRIFold(m, input) specific instructions.
    FriFold(i32, i32),
    Commit(i32),
}

impl<F: PrimeField32, EF: ExtensionField<F>> AsmInstruction<F, EF> {
    pub fn j(label: F) -> Self {
        AsmInstruction::JAL(ZERO, label, F::zero())
    }

    pub fn to_machine(self, pc: usize, label_to_pc: &BTreeMap<F, usize>) -> Instruction<F> {
        let i32_f = canonical_i32_to_field::<F>;
        let i32_f_arr = |x: i32| {
            [
                canonical_i32_to_field::<F>(x),
                F::zero(),
                F::zero(),
                F::zero(),
            ]
        };
        let f_u32 = |x: F| [x, F::zero(), F::zero(), F::zero()];
        let zero = [F::zero(), F::zero(), F::zero(), F::zero()];
        match self {
            AsmInstruction::Break(_) => panic!("Unresolved break instruction"),
            AsmInstruction::LW(dst, src, index, offset, size) => Instruction::new(
                Opcode::LW,
                i32_f(dst),
                i32_f_arr(src),
                i32_f_arr(index),
                offset,
                size,
                false,
                false,
            ),
            AsmInstruction::LWI(dst, src, index, offset, size) => Instruction::new(
                Opcode::LW,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(index),
                offset,
                size,
                false,
                true,
            ),
            AsmInstruction::SW(dst, src, index, offset, size) => Instruction::new(
                Opcode::SW,
                i32_f(dst),
                i32_f_arr(src),
                i32_f_arr(index),
                offset,
                size,
                false,
                false,
            ),
            AsmInstruction::SWI(dst, src, index, offset, size) => Instruction::new(
                Opcode::SW,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(index),
                offset,
                size,
                false,
                true,
            ),

            AsmInstruction::IMM(dst, value) => Instruction::new(
                Opcode::LW,
                i32_f(dst),
                f_u32(value),
                zero,
                F::zero(),
                F::one(),
                true,
                false,
            ),
            AsmInstruction::ADD(dst, lhs, rhs) => Instruction::new(
                Opcode::ADD,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::ADDI(dst, lhs, rhs) => Instruction::new(
                Opcode::ADD,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::SUB(dst, lhs, rhs) => Instruction::new(
                Opcode::SUB,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::SUBI(dst, lhs, rhs) => Instruction::new(
                Opcode::SUB,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::SUBIN(dst, lhs, rhs) => Instruction::new(
                Opcode::SUB,
                i32_f(dst),
                f_u32(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::MUL(dst, lhs, rhs) => Instruction::new(
                Opcode::MUL,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::MULI(dst, lhs, rhs) => Instruction::new(
                Opcode::MUL,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::DIV(dst, lhs, rhs) => Instruction::new(
                Opcode::DIV,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::DIVI(dst, lhs, rhs) => Instruction::new(
                Opcode::DIV,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::DIVIN(dst, lhs, rhs) => Instruction::new(
                Opcode::DIV,
                i32_f(dst),
                f_u32(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::LE(dst, src, index, offset, size) => Instruction::new(
                Opcode::LE,
                i32_f(dst),
                i32_f_arr(src),
                i32_f_arr(index),
                offset,
                size,
                false,
                false,
            ),
            AsmInstruction::LEI(dst, src, index, offset, size) => Instruction::new(
                Opcode::LE,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(index),
                offset,
                size,
                false,
                true,
            ),
            AsmInstruction::SE(dst, src, index, offset, size) => Instruction::new(
                Opcode::SE,
                i32_f(dst),
                i32_f_arr(src),
                i32_f_arr(index),
                offset,
                size,
                false,
                false,
            ),
            AsmInstruction::SEI(dst, src, index, offset, size) => Instruction::new(
                Opcode::SE,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(index),
                offset,
                size,
                false,
                true,
            ),
            AsmInstruction::EIMM(dst, value) => Instruction::new(
                Opcode::LE,
                i32_f(dst),
                value.as_base_slice().try_into().unwrap(),
                zero,
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::EADD(dst, lhs, rhs) => Instruction::new(
                Opcode::EADD,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::EADDI(dst, lhs, rhs) => Instruction::new(
                Opcode::EADD,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::ESUB(dst, lhs, rhs) => Instruction::new(
                Opcode::ESUB,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::ESUBI(dst, lhs, rhs) => Instruction::new(
                Opcode::ESUB,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::ESUBIN(dst, lhs, rhs) => Instruction::new(
                Opcode::ESUB,
                i32_f(dst),
                lhs.as_base_slice().try_into().unwrap(),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::EMUL(dst, lhs, rhs) => Instruction::new(
                Opcode::EMUL,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::EMULI(dst, lhs, rhs) => Instruction::new(
                Opcode::EMUL,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::EDIV(dst, lhs, rhs) => Instruction::new(
                Opcode::EDIV,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::EDIVI(dst, lhs, rhs) => Instruction::new(
                Opcode::EDIV,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::EDIVIN(dst, lhs, rhs) => Instruction::new(
                Opcode::EDIV,
                i32_f(dst),
                lhs.as_base_slice().try_into().unwrap(),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::EADDF(dst, lhs, rhs) => Instruction::new(
                Opcode::EFADD,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::EADDFI(dst, lhs, rhs) => Instruction::new(
                Opcode::EFADD,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::FADDEI(dst, lhs, rhs) => Instruction::new(
                Opcode::EFADD,
                i32_f(dst),
                rhs.as_base_slice().try_into().unwrap(),
                i32_f_arr(lhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::ESUBF(dst, lhs, rhs) => Instruction::new(
                Opcode::EFSUB,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::ESUBFI(dst, lhs, rhs) => Instruction::new(
                Opcode::EFSUB,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::ESUBFIN(dst, lhs, rhs) => Instruction::new(
                Opcode::EFSUB,
                i32_f(dst),
                f_u32(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::FSUBE(dst, lhs, rhs) => Instruction::new(
                Opcode::FESUB,
                i32_f(dst),
                i32_f_arr(rhs),
                i32_f_arr(lhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::FSUBEI(dst, lhs, rhs) => Instruction::new(
                Opcode::FESUB,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::FSUBEIN(dst, lhs, rhs) => Instruction::new(
                Opcode::FESUB,
                i32_f(dst),
                lhs.as_base_slice().try_into().unwrap(),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::EMULF(dst, lhs, rhs) => Instruction::new(
                Opcode::EFMUL,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::EMULFI(dst, lhs, rhs) => Instruction::new(
                Opcode::EFMUL,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::FMULEI(dst, lhs, rhs) => Instruction::new(
                Opcode::EFMUL,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::EDIVF(dst, lhs, rhs) => Instruction::new(
                Opcode::EFDIV,
                i32_f(dst),
                i32_f_arr(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::EDIVFI(dst, lhs, rhs) => Instruction::new(
                Opcode::EFDIV,
                i32_f(dst),
                i32_f_arr(lhs),
                f_u32(rhs),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::EDIVFIN(dst, lhs, rhs) => Instruction::new(
                Opcode::FEDIV,
                i32_f(dst),
                f_u32(lhs),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::FDIVI(dst, lhs, rhs) => Instruction::new(
                Opcode::FEDIV,
                i32_f(dst),
                i32_f_arr(lhs),
                rhs.as_base_slice().try_into().unwrap(),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::FDIVIN(dst, lhs, rhs) => Instruction::new(
                Opcode::EFDIV,
                i32_f(dst),
                lhs.as_base_slice().try_into().unwrap(),
                i32_f_arr(rhs),
                F::zero(),
                F::zero(),
                true,
                false,
            ),
            AsmInstruction::BEQ(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::BEQ,
                    i32_f(lhs),
                    i32_f_arr(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    false,
                    true,
                )
            }
            AsmInstruction::BEQI(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::BEQ,
                    i32_f(lhs),
                    f_u32(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    true,
                    true,
                )
            }
            AsmInstruction::BNE(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::BNE,
                    i32_f(lhs),
                    i32_f_arr(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    false,
                    true,
                )
            }
            AsmInstruction::BNEINC(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::BNEINC,
                    i32_f(lhs),
                    i32_f_arr(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    false,
                    true,
                )
            }
            AsmInstruction::BNEI(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::BNE,
                    i32_f(lhs),
                    f_u32(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    true,
                    true,
                )
            }
            AsmInstruction::BNEIINC(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::BNEINC,
                    i32_f(lhs),
                    f_u32(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    true,
                    true,
                )
            }
            AsmInstruction::EBNE(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::EBNE,
                    i32_f(lhs),
                    i32_f_arr(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    false,
                    true,
                )
            }
            AsmInstruction::EBNEI(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::EBNE,
                    i32_f(lhs),
                    rhs.as_base_slice().try_into().unwrap(),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    true,
                    true,
                )
            }
            AsmInstruction::EBEQ(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::EBEQ,
                    i32_f(lhs),
                    i32_f_arr(rhs),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    false,
                    true,
                )
            }
            AsmInstruction::EBEQI(label, lhs, rhs) => {
                let offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::EBEQ,
                    i32_f(lhs),
                    rhs.as_base_slice().try_into().unwrap(),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    true,
                    true,
                )
            }
            AsmInstruction::JAL(dst, label, offset) => {
                let pc_offset =
                    F::from_canonical_usize(label_to_pc[&label]) - F::from_canonical_usize(pc);
                Instruction::new(
                    Opcode::JAL,
                    i32_f(dst),
                    f_u32(pc_offset),
                    f_u32(offset),
                    F::zero(),
                    F::zero(),
                    false,
                    true,
                )
            }
            AsmInstruction::JALR(dst, label, offset) => Instruction::new(
                Opcode::JALR,
                i32_f(dst),
                i32_f_arr(label),
                i32_f_arr(offset),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::TRAP => Instruction::new(
                Opcode::TRAP,
                F::zero(),
                zero,
                zero,
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::HintBits(dst, src) => Instruction::new(
                Opcode::HintBits,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::Poseidon2Permute(dst, src) => Instruction::new(
                Opcode::Poseidon2Perm,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::PrintF(dst) | AsmInstruction::PrintV(dst) => Instruction::new(
                Opcode::PrintF,
                i32_f(dst),
                f_u32(F::zero()),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::PrintE(dst) => Instruction::new(
                Opcode::PrintE,
                i32_f(dst),
                f_u32(F::zero()),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::Ext2Felt(dst, src) => Instruction::new(
                Opcode::Ext2Felt,
                i32_f(dst),
                i32_f_arr(src),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::HintLen(dst) => Instruction::new(
                Opcode::HintLen,
                i32_f(dst),
                i32_f_arr(dst),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::Hint(dst) => Instruction::new(
                Opcode::Hint,
                i32_f(dst),
                i32_f_arr(dst),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::FriFold(m, ptr) => Instruction::new(
                Opcode::FRIFold,
                i32_f(m),
                i32_f_arr(ptr),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                false,
                true,
            ),
            AsmInstruction::Poseidon2Compress(result, src1, src2) => Instruction::new(
                Opcode::Poseidon2Compress,
                i32_f(result),
                i32_f_arr(src1),
                i32_f_arr(src2),
                F::zero(),
                F::zero(),
                false,
                false,
            ),
            AsmInstruction::Commit(pv_hash) => Instruction::new(
                Opcode::Commit,
                i32_f(pv_hash),
                f_u32(F::zero()),
                f_u32(F::zero()),
                F::zero(),
                F::zero(),
                true,
                true,
            ),
        }
    }

    pub fn fmt(&self, labels: &BTreeMap<F, String>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsmInstruction::Break(_) => panic!("Unresolved break instruction"),
            AsmInstruction::LW(dst, src, index, offset, size) => {
                write!(
                    f,
                    "lw    ({})fp, ({})fp, ({})fp, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::LWI(dst, src, index, offset, size) => {
                write!(
                    f,
                    "lwi   ({})fp, ({})fp, {}, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::SW(dst, src, index, offset, size) => {
                write!(
                    f,
                    "sw    ({})fp, ({})fp, ({})fp, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::SWI(dst, src, index, offset, size) => {
                write!(
                    f,
                    "swi   ({})fp, ({})fp, {}, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::IMM(dst, value) => write!(f, "imm   ({})fp, {}", dst, value),
            AsmInstruction::ADD(dst, lhs, rhs) => {
                write!(f, "add   ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::ADDI(dst, lhs, rhs) => {
                write!(f, "addi  ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::SUB(dst, lhs, rhs) => {
                write!(f, "sub   ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::SUBI(dst, lhs, rhs) => {
                write!(f, "subi  ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::SUBIN(dst, lhs, rhs) => {
                write!(f, "subin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::MUL(dst, lhs, rhs) => {
                write!(f, "mul   ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::MULI(dst, lhs, rhs) => {
                write!(f, "muli  ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::DIV(dst, lhs, rhs) => {
                write!(f, "div   ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::DIVI(dst, lhs, rhs) => {
                write!(f, "divi  ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::DIVIN(dst, lhs, rhs) => {
                write!(f, "divin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EIMM(dst, value) => write!(f, "eimm  ({})fp, {}", dst, value),
            AsmInstruction::LE(dst, src, index, offset, size) => {
                write!(
                    f,
                    "le    ({})fp, ({})fp, ({})fp, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::LEI(dst, src, index, offset, size) => {
                write!(
                    f,
                    "lei   ({})fp, ({})fp, {}, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::SE(dst, src, index, offset, size) => {
                write!(
                    f,
                    "se    ({})fp, ({})fp, ({})fp, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::SEI(dst, src, index, offset, size) => {
                write!(
                    f,
                    "sei   ({})fp, ({})fp, {}, {}, {}",
                    dst, src, index, offset, size
                )
            }
            AsmInstruction::EADD(dst, lhs, rhs) => {
                write!(f, "eadd  ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EADDI(dst, lhs, rhs) => {
                write!(f, "eaddi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::ESUB(dst, lhs, rhs) => {
                write!(f, "esub  ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::ESUBI(dst, lhs, rhs) => {
                write!(f, "esubi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::ESUBIN(dst, lhs, rhs) => {
                write!(f, "esubin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EMUL(dst, lhs, rhs) => {
                write!(f, "emul  ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EMULI(dst, lhs, rhs) => {
                write!(f, "emuli ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::EDIV(dst, lhs, rhs) => {
                write!(f, "ediv  ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EDIVI(dst, lhs, rhs) => {
                write!(f, "edivi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::EDIVIN(dst, lhs, rhs) => {
                write!(f, "edivin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::JAL(dst, label, offset) => {
                if *offset == F::zero() {
                    return write!(
                        f,
                        "j     ({})fp, {}",
                        dst,
                        labels.get(label).unwrap_or(&format!(".L{}", label))
                    );
                }
                write!(
                    f,
                    "jal   ({})fp, {}, {}",
                    dst,
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    offset
                )
            }
            AsmInstruction::EADDF(dst, lhs, rhs) => {
                write!(f, "eaddf ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EADDFI(dst, lhs, rhs) => {
                write!(f, "eaddfi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::FADDEI(dst, lhs, rhs) => {
                write!(f, "faddei ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::ESUBF(dst, lhs, rhs) => {
                write!(f, "esubf ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::ESUBFI(dst, lhs, rhs) => {
                write!(f, "esubfi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::ESUBFIN(dst, lhs, rhs) => {
                write!(f, "esubfin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::FSUBE(dst, lhs, rhs) => {
                write!(f, "fsube ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::FSUBEI(dst, lhs, rhs) => {
                write!(f, "fsubei ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::FSUBEIN(dst, lhs, rhs) => {
                write!(f, "fsubein ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EMULF(dst, lhs, rhs) => {
                write!(f, "emulf ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EMULFI(dst, lhs, rhs) => {
                write!(f, "emulfi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::FMULEI(dst, lhs, rhs) => {
                write!(f, "fmulei ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::EDIVF(dst, lhs, rhs) => {
                write!(f, "edivf ({})fp, ({})fp, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::EDIVFI(dst, lhs, rhs) => {
                write!(f, "edivfi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::EDIVFIN(dst, lhs, rhs) => {
                write!(f, "edivfin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::FDIVI(dst, lhs, rhs) => {
                write!(f, "fdivi ({})fp, ({})fp, {}", dst, lhs, rhs)
            }
            AsmInstruction::FDIVIN(dst, lhs, rhs) => {
                write!(f, "fdivin ({})fp, {}, ({})fp", dst, lhs, rhs)
            }
            AsmInstruction::JALR(dst, label, offset) => {
                write!(f, "jalr  ({})fp, ({})fp, ({})fp", dst, label, offset)
            }
            AsmInstruction::BNE(label, lhs, rhs) => {
                write!(
                    f,
                    "bne   {}, ({})fp, ({})fp",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::BNEI(label, lhs, rhs) => {
                write!(
                    f,
                    "bnei  {}, ({})fp, {}",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::BNEINC(label, lhs, rhs) => {
                write!(
                    f,
                    "bneinc {}, ({})fp, {}",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::BNEIINC(label, lhs, rhs) => {
                write!(
                    f,
                    "bneiinc {}, ({})fp, {}",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::BEQ(label, lhs, rhs) => {
                write!(
                    f,
                    "beq  {}, ({})fp, ({})fp",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::BEQI(label, lhs, rhs) => {
                write!(
                    f,
                    "beqi {}, ({})fp, {}",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::EBNE(label, lhs, rhs) => {
                write!(
                    f,
                    "ebne  {}, ({})fp, ({})fp",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::EBNEI(label, lhs, rhs) => {
                write!(
                    f,
                    "ebnei {}, ({})fp, {}",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::EBEQ(label, lhs, rhs) => {
                write!(
                    f,
                    "ebeq  {}, ({})fp, ({})fp",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::EBEQI(label, lhs, rhs) => {
                write!(
                    f,
                    "ebeqi {}, ({})fp, {}",
                    labels.get(label).unwrap_or(&format!(".L{}", label)),
                    lhs,
                    rhs
                )
            }
            AsmInstruction::TRAP => write!(f, "trap"),
            AsmInstruction::HintBits(dst, src) => write!(f, "hint_bits ({})fp, ({})fp", dst, src),
            AsmInstruction::Poseidon2Permute(dst, src) => {
                write!(f, "poseidon2_permute ({})fp, ({})fp", dst, src)
            }
            AsmInstruction::PrintF(dst) => {
                write!(f, "print_f ({})fp", dst)
            }
            AsmInstruction::PrintV(dst) => {
                write!(f, "print_v ({})fp", dst)
            }
            AsmInstruction::PrintE(dst) => {
                write!(f, "print_e ({})fp", dst)
            }
            AsmInstruction::Ext2Felt(dst, src) => write!(f, "ext2felt ({})fp, {})fp", dst, src),
            AsmInstruction::HintLen(dst) => write!(f, "hint_len ({})fp", dst),
            AsmInstruction::Hint(dst) => write!(f, "hint ({})fp", dst),
            AsmInstruction::FriFold(m, input_ptr) => {
                write!(f, "fri_fold ({})fp, ({})fp", m, input_ptr)
            }
            AsmInstruction::Poseidon2Compress(result, src1, src2) => {
                write!(
                    f,
                    "poseidon2_compress ({})fp, {})fp, {})fp",
                    result, src1, src2
                )
            }
            AsmInstruction::Commit(pv_hash) => {
                write!(f, "commit ({})fp", pv_hash)
            }
        }
    }
}
