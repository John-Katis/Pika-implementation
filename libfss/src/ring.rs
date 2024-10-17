use num::ToPrimitive;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::u32;
use std::ops::{Add, Sub, Mul};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RingElm {
    value: u32,
}

impl Add for RingElm {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        RingElm {
            value: self.value.wrapping_add( rhs.value ),
        }
    }
}
impl Sub for RingElm {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        RingElm {
            value: self.value.wrapping_sub( rhs.value ),
        }
    }
}
impl Mul for RingElm {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        RingElm {
            value: self.value.wrapping_mul( rhs.value ),
        }
    }
}


impl RingElm {
    pub fn to_vec(&self, len: usize) -> Vec<RingElm> {
        std::iter::repeat(self.clone()).take(len).collect()
    }

    pub fn print(&self){
        print!("{} ", self.value);
    }

    pub fn to_u32(&self) -> Option<u32> {
        self.value.to_u32()
    }

    pub fn to_u8_vec(&self) -> Vec<u8> {
        self.value.to_be_bytes().to_vec()
    }
}

/*******/
impl From<f32> for RingElm {
    #[inline]
    fn from(inp: f32) -> Self {
        RingElm {
            value: inp as u32,
        }
    }
}

impl From<u32> for RingElm {
    #[inline]
    fn from(inp: u32) -> Self {
        RingElm {
            value: inp,
        }
    }
}

impl From<Vec<u8>> for RingElm {
    #[inline]
    fn from(bytes:Vec<u8>) -> Self {
        if bytes.len() != 4 {
            panic!("Invalid conversion: Vec<u8> must be exactly 4 bytes");
        }
        RingElm {
            value: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }
}

impl Ord for RingElm {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for RingElm {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.value.cmp(&other.value))
    }
}

impl crate::Group for RingElm {
    #[inline]
    fn zero() -> Self {
        RingElm::from(0)
    }

    #[inline]
    fn one() -> Self {
        RingElm::from(1)
    }

    #[inline]
    fn add(&mut self, other: &Self) {
        self.value = self.value.wrapping_add( other.value );
    }

    #[inline]
    fn sub(&mut self, other: &Self) {
        self.value = self.value.wrapping_sub( other.value );
    }

    #[inline]
    fn mul(&mut self, other: &Self) {
        self.value = self.value.wrapping_mul( other.value );
    }

     #[inline]
    fn negate(&mut self) {
        self.value = u32::MAX - &self.value;
    }
}

impl crate::Group for bool {
    #[inline]
    fn zero() -> Self {
        false
    }

    #[inline]
    fn one() -> Self {
        true
    }

    #[inline]
    fn add(&mut self, other: &Self) {
        *self = *self ^ other;
    }

    #[inline]
    fn sub(&mut self, other: &Self) {
        *self = *self ^ other;
    }

    #[inline]
    fn mul(&mut self, other: &Self) {
        *self = *self & other;
    }

     #[inline]
    fn negate(&mut self) {
        *self = !*self;
    }
}

impl crate::prg::FromRng for RingElm {
    #[inline]
    fn from_rng(&mut self, rng: &mut impl rand::Rng) {
        self.value = rng.next_u32();
    }
}

impl crate::Share for RingElm {
}

impl<T> crate::Group for (T, T) where T: crate::Group + Clone,
{
    #[inline]
    fn zero() -> Self {
        (T::zero(), T::zero())
    }

    #[inline]
    fn one() -> Self {
        (T::one(), T::one())
    }

    #[inline]
    fn add(&mut self, other: &Self) {
        self.0.add(&other.0);
        self.1.add(&other.1);
    }

    #[inline]
    fn mul(&mut self, other: &Self) {
        self.0.mul(&other.0);
        self.1.mul(&other.1);
    }

    #[inline]
    fn sub(&mut self, other: &Self) {
        let mut inv0 = other.0.clone();
        let mut inv1 = other.1.clone();
        inv0.negate();
        inv1.negate();
        self.0.add(&inv0);
        self.1.add(&inv1);
    }

    #[inline]
    fn negate(&mut self) {
        self.0.negate();
        self.1.negate();
    }
}
