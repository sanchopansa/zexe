// use pairing::{
//     CurveAffine,
//     CurveProjective,
//     Engine
// };

// use pairing::ff::{
//     PrimeField,
//     Field,
//     PrimeFieldRepr,
//     ScalarEngine};

// multiexp is using a technicality of used pippenger version in multiexp:
// it says which bases exist ,since some polynomials might evaluate to zero
// (e.g., the variable is not a part of any constraints, for B - it only
// participates in A) so you don't include these bases in the generated
// parameters (pk/vk)

use algebra::{
    AffineCurve as CurveAffine, Field, PairingEngine as Engine, PrimeField,
    ProjectiveCurve as CurveProjective,
};

use bit_vec::{self, BitVec};
use std::{io, iter, sync::Arc};

use crate::SynthesisError;

/// An object that builds a source of bases.
pub trait SourceBuilder<G: CurveAffine>: Send + Sync + 'static + Clone {
    type Source: Source<G>;

    fn new(self) -> Self::Source;
}

/// A source of bases, like an iterator.
pub trait Source<G: CurveAffine> {
    /// Parses the element from the source. Fails if the point is at infinity.
    fn add_assign_mixed(
        &mut self,
        to: &mut <G as CurveAffine>::Projective,
    ) -> Result<(), SynthesisError>;

    /// Skips `amt` elements from the source, avoiding deserialization.
    fn skip(&mut self, amt: usize) -> Result<(), SynthesisError>;
}

impl<G: CurveAffine> SourceBuilder<G> for (Arc<Vec<G>>, usize) {
    type Source = (Arc<Vec<G>>, usize);

    fn new(self) -> (Arc<Vec<G>>, usize) {
        (self.0.clone(), self.1)
        // wraps an array with an additional index
    }
}

impl<G: CurveAffine> Source<G> for (Arc<Vec<G>>, usize) {
    /// Parses the element from the source. Fails if the point is at infinity.
    fn add_assign_mixed(
        &mut self,
        to: &mut <G as CurveAffine>::Projective,
    ) -> Result<(), SynthesisError> {
        if self.0.len() <= self.1 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "expected more bases when adding from source",
            )
            .into());
        }
        // ensure that vector is longer enough

        if self.0[self.1].is_zero() {
            return Err(SynthesisError::UnexpectedIdentity);
        }
        // vector does not allow any zero values at index

        to.add_assign_mixed(&self.0[self.1]);
        // add indexed value base to "to"

        self.1 += 1;
        // increment index

        Ok(())
    }

    fn skip(&mut self, amt: usize) -> Result<(), SynthesisError> {
        if self.0.len() <= self.1 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "expected more bases skipping from source",
            )
            .into());
        }

        self.1 += amt;
        // skip a series of bases

        Ok(())
    }
}

pub trait QueryDensity {
    /// Returns whether the base exists.
    type Iter: Iterator<Item = bool>;

    fn iter(self) -> Self::Iter;
    fn get_query_size(self) -> Option<usize>;
}

#[derive(Clone)]
pub struct FullDensity;

impl AsRef<FullDensity> for FullDensity {
    fn as_ref(&self) -> &FullDensity {
        self
    }
}

// For fulldensity query size is None and iter is an endless loop of 1
impl<'a> QueryDensity for &'a FullDensity {
    type Iter = iter::Repeat<bool>;

    fn iter(self) -> Self::Iter {
        iter::repeat(true)
    }

    fn get_query_size(self) -> Option<usize> {
        None
    }
}

#[derive(Clone)]
pub struct DensityTracker {
    bv:            BitVec,
    total_density: usize,
}

// DensityTracker: iter is bv and lenth is bv length
impl<'a> QueryDensity for &'a DensityTracker {
    type Iter = bit_vec::Iter<'a>;

    fn iter(self) -> Self::Iter {
        self.bv.iter()
    }

    fn get_query_size(self) -> Option<usize> {
        Some(self.bv.len())
    }
}

impl DensityTracker {
    pub fn new() -> DensityTracker {
        DensityTracker {
            bv:            BitVec::new(),
            total_density: 0,
        }
    }

    pub fn add_element(&mut self) {
        self.bv.push(false);
    }

    // total density only gets adjusted with "inc"
    // then i also set value to true for a given index
    pub fn inc(&mut self, idx: usize) {
        if !self.bv.get(idx).unwrap() {
            self.bv.set(idx, true);
            self.total_density += 1;
        }
    }

    pub fn get_total_density(&self) -> usize {
        self.total_density
    }
}
