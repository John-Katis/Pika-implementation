pub mod prg;
pub mod dpf;
pub mod beavertuple;

mod ring;

pub use crate::ring::RingElm;


trait TupleMapToExt<T, U> {
    type Output;
    fn map<F: FnMut(&T) -> U>(&self, f: F) -> Self::Output;
}

type TupleMutIter<'a, T> =
    std::iter::Chain<std::iter::Once<(bool, &'a mut T)>, std::iter::Once<(bool, &'a mut T)>>;
 trait TupleExt<T> {
    fn get(&self, val: bool) -> &T;
    fn get_mut(&mut self, val: bool) -> &mut T;
    fn iter_mut(&mut self) -> TupleMutIter<T>;
}

impl<T, U> TupleMapToExt<T, U> for (T, T) {
    type Output = (U, U);

    #[inline(always)]
    fn map<F: FnMut(&T) -> U>(&self, mut f: F) -> Self::Output {
        (f(&self.0), f(&self.1))
    }
}

impl<T> TupleExt<T> for (T, T) {
    #[inline(always)]
    fn get(&self, val: bool) -> &T {
        match val {
            false => &self.0,
            true => &self.1,
        }
    }

    #[inline(always)]
    fn get_mut(&mut self, val: bool) -> &mut T {
        match val {
            false => &mut self.0,
            true => &mut self.1,
        }
    }

    fn iter_mut(&mut self) -> TupleMutIter<T> {
        std::iter::once((false, &mut self.0)).chain(std::iter::once((true, &mut self.1)))
    }
}


// Additive group, such as (Z_n, +)
pub trait Group {
    fn zero() -> Self;
    fn one() -> Self;
    fn negate(&mut self);
    fn add(&mut self, other: &Self);
    fn mul(&mut self, other: &Self);
    fn sub(&mut self, other: &Self);
}

pub trait Share: Group + prg::FromRng + Clone {
    fn random() -> Self {
        let mut out = Self::zero();
        out.randomize();
        out
    }

    fn share(&self) -> (Self, Self) {
        let mut s0 = Self::zero();
        s0.randomize();
        let mut s1 = self.clone();
        s1.sub(&s0);

        (s0, s1)
    }

    fn share_random() -> (Self, Self) {
        (Self::random(), Self::random())
    }
}

pub fn bits_to_u16(bits: &[bool]) -> u16 {
    assert!(bits.len() <= 16);
    let mut out = 0u16;

    for &bit in bits {
        out = (out << 1) | bit as u16;
    }
    out
}

pub fn bits_to_u32(bits: &[bool]) -> u32 {
    assert!(bits.len() <= 32);
    let mut out = 0u32;

    for i in 0..bits.len() {
        let b32: u32 = bits[i].into();
        out |= b32 << i;
    }
    out
}

pub fn u64_to_bits(input: u64) -> Vec<bool> {

    let mut out: Vec<bool> = Vec::new();
    for i in 0..64 {
        let bit = (input & (1 << i)) != 0;
        out.push(bit);
    }
    out
}
