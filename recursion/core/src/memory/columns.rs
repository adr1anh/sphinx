use wp1_derive::AlignedBorrow;

use crate::air::Block;

#[derive(AlignedBorrow, Default, Debug, Clone)]
#[repr(C)]
pub struct MemoryInitCols<T> {
    pub addr: T,
    pub timestamp: T,
    pub value: Block<T>,
    pub is_real: T,
}
