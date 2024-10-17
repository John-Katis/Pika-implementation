use crate::mpc_party::MPCParty;
use fss::*;
use fss::RingElm;
use crate::offline_data::*;

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>)->Vec<RingElm>{

    
    //TODO implement pika protocol steps 2 & 3

    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input
    // Protocol 2(b) - compute yσ (EvalAll routine -> implement in DPF key)
    // Protocol 2(c) - compute u
    // Protocol 3    - output beaver triple (u * w)
       
}

// This function can be used to load a .bin file from the data folder in the project
// Rename the path .bin file name according to how your function database file is named
fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    match read_file("../data/func_database.bin") {
        Ok(value) => ret = value,
        Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
    }
    ret
}
