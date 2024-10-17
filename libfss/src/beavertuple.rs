use crate::prg::PrgSeed;
use crate::prg::FixedKeyPrgStream;
use crate::bits_to_u32;
use crate::Group;

use super::RingElm;
use serde::Deserialize;
use serde::Serialize;

const NUMERIC_LEN:usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeaverTuple{
    pub a: RingElm,
    pub b: RingElm,
    pub ab: RingElm,
    pub delta_a: RingElm,
    pub delta_b: RingElm,
}

impl BeaverTuple{
    // fn new(ra: RingElm, rb: RingElm, rc: RingElm) -> Self{
    //     BeaverTuple { a: ra, b: rb, ab: rc, delta_a:RingElm::zero(), delta_b:RingElm::zero(), }
    // }

    pub fn gen_beaver(beavertuples0: &mut Vec<BeaverTuple>, beavertuples1: &mut Vec<BeaverTuple>, seed: &PrgSeed) {
        let mut stream = FixedKeyPrgStream::new();
        stream.set_key(&seed.key);

        let rd_bits = stream.next_bits(NUMERIC_LEN*5);
        let a0 = RingElm::from( bits_to_u32(&rd_bits[..NUMERIC_LEN]) );
        let b0 = RingElm::from( bits_to_u32(&rd_bits[NUMERIC_LEN..2*NUMERIC_LEN]) );

        let a1 = RingElm::from( bits_to_u32(&rd_bits[2*NUMERIC_LEN..3*NUMERIC_LEN]) );
        let b1 = RingElm::from( bits_to_u32(&rd_bits[3*NUMERIC_LEN..4*NUMERIC_LEN]));

        let ab0 = RingElm::from( bits_to_u32(&rd_bits[4*NUMERIC_LEN..5*NUMERIC_LEN]) );

        let mut a = RingElm::zero();
        a.add(&a0);
        a.add(&a1);

        let mut b = RingElm::zero();
        b.add(&b0);
        b.add(&b1);

        let mut ab = RingElm::one();
        ab.mul(&a);
        ab.mul(&b);

        ab.sub(&ab0);

        let beaver0 = BeaverTuple{
            a: a0,
            b: b0,
            ab: ab0,
            delta_a:RingElm::zero(),
            delta_b:RingElm::zero(),
        };

        let beaver1 = BeaverTuple{
            a: a1,
            b: b1,
            ab: ab,
            delta_a:RingElm::zero(),
            delta_b:RingElm::zero(),
        };
        beavertuples0.push(beaver0);
        beavertuples1.push(beaver1);
            
    }
    
    pub fn beaver_mul0(&mut self, alpha: RingElm, beta: RingElm)-> Vec<u8>{
        self.delta_a = alpha - self.a;
        self.delta_b = beta - self.b;

        let mut container  = Vec::<u8>::new();
        container.append(&mut self.delta_a.to_u32().unwrap().to_be_bytes().to_vec());
        container.append(&mut self.delta_b.to_u32().unwrap().to_be_bytes().to_vec());
        container
    }

    /*The multiplication of [alpha] x [beta], the values of beaver_share are [a], [b], and [ab], d and e are the reconstructed values of alpha-a, beta-b*/
    pub fn beaver_mul1(&mut self, is_server: bool, other_half:&Vec<u8> ) -> RingElm{
        assert_eq!(other_half.len(),8usize);
        for i in 0..2{
            let mut ybuf: [u8; 4]= [0; 4];
            for j in 0..4{
                ybuf[j] = other_half[i*4+j];
            }
            if i==0{
                self.delta_a.add(&RingElm::from(u32::from_be_bytes(ybuf)));
            }
            else{
                self.delta_b.add(&RingElm::from(u32::from_be_bytes(ybuf)));
            }
        }
        let mut result= RingElm::zero();
        if is_server{
            result.add(&(self.delta_a*self.delta_b) );
        }
        result.add(&(self.delta_a*self.b) );
        result.add(&(self.delta_b*self.a) );
        result.add(& self.ab);
        result
    }

    pub fn mul_open(&mut self, alpha: RingElm, beta: RingElm) -> (RingElm, RingElm){
        self.delta_a = alpha - self.a;
        self.delta_b = beta - self.b;
        (self.delta_a, self.delta_b)
    }

    pub fn mul_compute(&mut self, is_server: bool, alpha: &RingElm, beta: &RingElm) -> RingElm{
        self.delta_a = alpha.clone();
        self.delta_b = beta.clone();
        let mut result= RingElm::zero();
        if is_server{
            result.add(&(self.delta_a*self.delta_b) );
        }
        result.add(&(self.delta_a*self.b) );
        result.add(&(self.delta_b*self.a) );
        result.add(& self.ab);
        result
    }

}
