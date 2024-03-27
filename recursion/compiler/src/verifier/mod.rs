pub mod challenger;
pub mod constraints;
pub mod folder;
pub mod fri;

pub use constraints::*;

use p3_field::PrimeField;
use std::marker::PhantomData;
use wp1_core::stark::StarkGenericConfig;

use crate::prelude::Config;

#[derive(Clone)]
pub struct StarkGenericBuilderConfig<N, SC> {
    marker: PhantomData<(N, SC)>,
}

impl<N: PrimeField, SC: StarkGenericConfig + Clone> Config for StarkGenericBuilderConfig<N, SC> {
    type N = N;
    type F = SC::Val;
    type EF = SC::Challenge;
}
