use crate::prg;
use crate::Group;
use serde::Deserialize;
use serde::Serialize;
use std::mem;
use crate::TupleExt;
use crate::TupleMapToExt;
use crate::prg::PrgOutput;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CorWord{
    seed: prg::PrgSeed,
    bits: (bool, bool),
}

pub struct EvalState{
    seed: Vec<prg::PrgSeed>,
    t_bit: Vec<bool>,
    bit: Vec<bool>,
    tau: Vec<PrgOutput>,
}

impl EvalState {
    fn slice_to_index(&self, index: usize) -> EvalState {
        let seed = self.seed.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());
        let bit = self.bit.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());
        let t_bit = self.t_bit.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());
        let tau = self.tau.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());

        EvalState { seed, bit, t_bit, tau }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DPFKey<T> {
    key_idx: bool,
    root_seed: prg::PrgSeed,
    cor_words: Vec<CorWord>,
    word: T,
}

fn gen_cor_word(bit: bool, bits: &mut (bool, bool), seeds: &mut (prg::PrgSeed, prg::PrgSeed)) -> CorWord
{
    let data = seeds.map(|s| s.expand());
    let keep = bit;
    let lose = !keep;

    let cw = CorWord {
        seed: data.0.seeds.get(lose) ^ data.1.seeds.get(lose),
        bits: (
            data.0.bits.0 ^ data.1.bits.0 ^ bit ^ true,
            data.0.bits.1 ^ data.1.bits.1 ^ bit,
        ),
    };
    for (b, seed) in seeds.iter_mut() {
        *seed = data.get(b).seeds.get(keep).clone();

        if *bits.get(b) {
            *seed = &*seed ^ &cw.seed;
        }

        let mut newbit = *data.get(b).bits.get(keep);
        if *bits.get(b) {
            newbit ^= cw.bits.get(keep);
        }

        *bits.get_mut(b) = newbit;
    }

    cw
}

fn u16_to_boolean_vector(num: u16) -> Vec<bool> {
    (0..16).map(|i| ((num >> i) & 1) == 1).rev().collect()
}

fn find_first_difference_index(v1: &[bool], v2: &[bool]) -> usize {
    for (i, (b1, b2)) in v1.iter().zip(v2.iter()).enumerate() {
        if b1 != b2 {
            return i as usize;
        }
    }
    v1.len() as usize
}

impl<T> DPFKey<T> where T: prg::FromRng + Clone + Group + std::fmt::Debug + std::cmp::PartialEq
{
    pub fn gen(alpha_bits: &[bool], value:&T) -> (DPFKey<T>, DPFKey<T>, bool) {
        let root_seeds = (prg::PrgSeed::zero(), prg::PrgSeed::one());
        let root_bits = (false, true);

        let mut seeds = root_seeds.clone();
        let mut bits = root_bits;
        let mut cor_words: Vec<CorWord> = Vec::new();
        let mut last_word:T = T::zero();

        for (i, &bit) in alpha_bits.iter().enumerate() {
            let cw = gen_cor_word(bit, &mut bits, &mut seeds);
            cor_words.push(cw);
            // Generate the last word
            if i==alpha_bits.len()-1{
                let converted = seeds.map(|s| s.convert());
                last_word.add(&value);
                last_word.sub(&converted.0.word);
                last_word.add(&converted.1.word);
                if bits.1 {
                    last_word.negate();
                }
            }
        }
        (
            DPFKey::<T> {
                key_idx: false,
                root_seed: root_seeds.0,
                cor_words: cor_words.clone(),
                word: last_word.clone(),
            },
            DPFKey::<T> {
                key_idx: true,
                root_seed: root_seeds.1,
                cor_words: cor_words,
                word:  last_word,
            },
            bits.0
        )
    }

    pub fn eval(&self, idx: &Vec<bool>) -> T {
        debug_assert!(idx.len() <= self.domain_size());
        debug_assert!(!idx.is_empty());

        let mut seed: prg::PrgSeed = self.root_seed.clone();
        // let dir = self.key_idx;
        let mut t_bit:bool = self.key_idx;

        let mut word:T = T::zero();

        for level in 0..idx.len() {
            let bit = idx[level];
            
            // Step 1: compute tau
            // 2 bis, 2 seeds
            // let tau = seed.expand_dir(!dir, dir);
            let tau = seed.expand();
            seed = tau.seeds.get(bit).clone();
            if t_bit{
                seed = &seed ^ &self.cor_words[level].seed;
                let new_bit = *tau.bits.get(bit);
                t_bit = new_bit ^ self.cor_words[level].bits.get(bit);
                
            }else{ //when t_bit is false, update seed and t_bit as orginal expanded tau value
                t_bit = *tau.bits.get(bit);
            }

            if level==idx.len()-1{
                let converted = seed.convert::<T>();
                word.add(&converted.word);
                if t_bit {
                    word.add(&self.word);
                }

                if self.key_idx {
                    word.negate();
                }
            }
        }

        word
    }

    pub fn eval_all(&self) -> Vec<T> {
        let mut y_vec: Vec<T> = Vec::new();        
        // let mut res: T;
        let mut prev_state: EvalState;
        let mut prev_num_bool: Vec<bool>;

        let max_value: u16 = u16::MAX;
        let half_value: u16 = (max_value / 2) + 1;
     
        for i in 0..2 {
            // Start from 0 and 1^k/2 outside of iteration
            let init_16b: u16 = i * half_value;
            let init_16b_bool_vec: Vec<bool> = u16_to_boolean_vector(init_16b);

            let iter_end: u16 = if i == 0 {
                half_value
            } else {
                max_value
            };

            let (res, state) = Self::stateful_eval_no_prev_state(&self, &init_16b_bool_vec);
            
            prev_state = state;
            prev_num_bool = init_16b_bool_vec;
            y_vec.push(res);

            for num in init_16b+1..iter_end {
                let num_bool_vec: Vec<bool> = u16_to_boolean_vector(num);
                let idx_diff = find_first_difference_index(&prev_num_bool, &num_bool_vec);

                let (res, state) = Self::stateful_eval(&self, &num_bool_vec, &prev_state, idx_diff);
                
                y_vec.push(res);

                prev_state = state;
                prev_num_bool = num_bool_vec;
            }
        }
        y_vec

    }

    pub fn stateful_eval(&self, idx: &Vec<bool>, prev_state: &EvalState, idx_diff: usize) -> (T, EvalState) {
        // INITIALIZE STATE / PARAMETERS
        let mut new_state = prev_state.slice_to_index(idx_diff-1);
        // State of the bit that is exactly previous to the one where the first difference is found
        let start_idx:usize = idx_diff;
        let mut seed:prg::PrgSeed = new_state.seed[new_state.seed.len()-1].clone();
        let mut t_bit:bool = new_state.t_bit[new_state.t_bit.len()-1];

        let mut word:T = T::zero();

        // Start from the index of the first bit difference - otherwise the same as eval but with state update and return
        for level in start_idx..idx.len() {
            let bit = idx[level];
            let tau = seed.expand();
            seed = tau.seeds.get(bit).clone();
            if t_bit{
                seed = &seed ^ &self.cor_words[level].seed;
                let new_bit = *tau.bits.get(bit);
                t_bit = new_bit ^ self.cor_words[level].bits.get(bit);
                
            }else{ //when t_bit is false, update seed and t_bit as orginal expanded tau value
                t_bit = *tau.bits.get(bit);
            }

            if level==idx.len()-1{
                let converted = seed.convert::<T>();
                word.add(&converted.word);
                if t_bit {
                    word.add(&self.word);
                }

                if self.key_idx {
                    word.negate();
                }
            }
            // UPDATE STATE
            new_state.seed.push(seed.clone());
            new_state.t_bit.push(t_bit);
            new_state.bit.push(bit);
            new_state.tau.push(tau);
        }
        (word, new_state)
    }

    pub fn stateful_eval_no_prev_state(&self, idx: &Vec<bool>) -> (T, EvalState) {
        // INITIALIZE STATE / PARAMETERS from 0 (no previous state)
        let mut new_state = EvalState {
            seed: Vec::new(),
            bit: Vec::new(),
            t_bit: Vec::new(),
            tau: Vec::new(),
        };

        let mut seed: prg::PrgSeed = self.root_seed.clone();
        let mut t_bit:bool = self.key_idx;
        
        let mut word:T = T::zero();

        for level in 0..idx.len() {
            let bit = idx[level];
            let tau = seed.expand();
            seed = tau.seeds.get(bit).clone();
            if t_bit{
                seed = &seed ^ &self.cor_words[level].seed;
                let new_bit = *tau.bits.get(bit);
                t_bit = new_bit ^ self.cor_words[level].bits.get(bit);
                
            }else{ //when t_bit is false, update seed and t_bit as orginal expanded tau value
                t_bit = *tau.bits.get(bit);
            }

            if level==idx.len()-1{
                let converted = seed.convert::<T>();
                word.add(&converted.word);
                if t_bit {
                    word.add(&self.word);
                }

                if self.key_idx {
                    word.negate();
                }
            }
            // UPDATE STATE
            new_state.seed.push(seed.clone());
            new_state.t_bit.push(t_bit);
            new_state.bit.push(bit);
            new_state.tau.push(tau);
        }
        (word, new_state)
    }

    pub fn domain_size(&self) -> usize {
        self.cor_words.len()
    }

    pub fn key_size(&self) -> usize {
        let mut key_size = 0usize;
        key_size += mem::size_of_val(&self.key_idx);
        key_size += mem::size_of_val(&self.root_seed);
        key_size += mem::size_of_val(&*self.cor_words);
        key_size += mem::size_of_val(&self.word);
        key_size
    }
}
