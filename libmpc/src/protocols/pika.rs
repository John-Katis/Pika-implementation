use crate::mpc_party::MPCParty;
use fss::*;
use fss::RingElm;
use crate::offline_data::*;

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>) -> Vec<RingElm> {

    // Protocol 2(a): reconstruct x = (r - a) mod 2^k -> r: random val, a: secret sharing of user input
    // Retreive r share via in memory for one party (for party 0 r0 and for party 1 r1)
    let r = p.offlinedata.r_share[0];

    // Retreive a (x) shares via in memory for one party (for party 0 a0 and for party 1 a1)
    let a = p.offlinedata.x_share[0];

    // Exchange r and a shares - round 2 (round 1 is manually added in benchmarking representing offline phase)
    let exchanged_values = p.netlayer.exchange_u16_vec(vec![r, a]).await;

    // Store shares 
    let exchanged_r = exchanged_values[0];
    let exchanged_a = exchanged_values[1];

    let modulus = 1u32.wrapping_shl(TOTAL_BITS as u32); // Define modulus 2^k where TOTAL_BITS is k
    let x = ((exchanged_r as u32).wrapping_sub(exchanged_a as u32) + modulus) % modulus; // Reconstruct x


    // Protocol 2(b): compute yσ (EvalAll routine -> implement in DPF key)
    let dpf_key = &p.offlinedata.k_share[0]; // Each party retrieves its DPF key

    // Each party evaluates their DPF keys to obtain yσ (vector indicating positions in look-up table based on x)
    let y_sigma = dpf_key.eval_all(); 


    // Protocol 2(c): compute u
    let func_db = load_func_db(); // Load the function database

    // Shift y_sigma by x and multiply each shifted value by the corresponding function output
    let u: Vec<RingElm> = y_sigma.iter()
        .cycle() // repeat values in y_sigma
        .skip(x as usize) // shift y_sigma by x
        .take(y_sigma.len()) // limit the repetition to the length of y_sigma
        .enumerate() 
        .map(|(i, &b)| {
            let func_value = func_db.get(i).copied().unwrap_or(1.0); // retrieve function value at index i from function database
            let ring_val = if b { RingElm::one() } else { RingElm::zero() }; // convert b to ring element
            ring_val * RingElm::from(func_value as u32) // multiply ring value by function value
        }) 
        .collect(); // collect results into a vector


    // Protocol 3 - output beaver triple (u * w)
    // Retrieve w share for each party
    let w_share = p.offlinedata.w_share[0];

    // Exchange u and w shares - 3 round
    let u_w_combined_exchanged = p.netlayer.exchange_ring_vec([u[0], w_share].to_vec()).await;

    // Each party now holds shares of u and w
    let u_exchanged = u_w_combined_exchanged[0];
    let w_share_exchanged = u_w_combined_exchanged[1];

    // Retrieve beaver triples
    let beaver_triple = &mut p.offlinedata.beavers[0];

    // Calculate delta values (intermediate values for secure computation) 
    let delta_values = beaver_triple.beaver_mul0(u_exchanged, w_share_exchanged);

    // Complete Beaver multiplication
    let result = beaver_triple.beaver_mul1(p.netlayer.is_server, &delta_values);

    vec![result] // Return result (parties share of the product)
}


// Load function database (sigmoid, tanh, ReLU)
fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    match read_file("../data/relu_table.bin") {
        Ok(value) => ret = value,
        Err(e) => println!("Error reading file: {}", e),  
    }
    ret
}