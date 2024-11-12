use crate::mpc_party::MPCParty;
use fss::*;
use fss::RingElm;
use crate::offline_data::*;

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>) -> Vec<RingElm> {

    // Protocol 2(a): reconstruct x = (r - a) mod 2^k
    // Retreive r share via in memory for one party (for party 0 r0 and for party 1 r1)
    let r = p.offlinedata.r_share[0];

    // Retreive a (x) shares via in memory for one party (for party 0 r0 and for party 1 r1)
    let a = p.offlinedata.x_share[0];

    // Exchange r and a shares - round 1 - so exchange is done for proper reconstruction
    let exchanged_values = p.netlayer.exchange_u16_vec(vec![r, a]).await;

    // Store shares 
    let exchanged_r = exchanged_values[0];
    let exchanged_a = exchanged_values[1];

    // Reconstruct x based on exchanged shares
    let modulus = 1u16.wrapping_shl(TOTAL_BITS as u32); // Define modulus 2^k where TOTAL_BITS is k
    let x = (exchanged_r.wrapping_sub(exchanged_a) + modulus) % modulus; // Compute x


    // Protocol 2(b): compute y_sigma and exchange DPF evaluations
    // Each party retrieves its DPF key
    let dpf_key = &p.offlinedata.k_share[0];

    // Evaluate the DPF keys over the entire domain and return a vector of boolean values for each party
    let y_sigma = dpf_key.eval_all();

    // Exchange y_sigma in a single round - round 2
    let y_sigma_exchanged = p.netlayer.exchange_bool_vec(y_sigma.clone()).await;


    // Protocol 2(c): compute u using values from func_db based on exchanged y_sigma
    let func_db = load_func_db(); // Load the function database

    // Compute u based on y_sigma_exchanged and func_db
    let u: Vec<RingElm> = y_sigma_exchanged.iter()
        .cycle() // repeat values in y_sigma_exchanged
        .skip(x as usize) // shift starting point by x
        .take(y_sigma_exchanged.len()) // limit the repetition to the length of y_sigma_exchanged
        .enumerate() 
        .map(|(i, &b)| {
            let func_value = func_db.get(i).copied().unwrap_or(1.0); // retrieve function value at index i from function database
            let ring_val = if b { RingElm::one() } else { RingElm::zero() }; // convert b to ring element
            ring_val * RingElm::from(func_value as u32) // multiply ring value by function value
        }) // map each element in the shifted sequence 
        .collect(); // collect results into a vector


    // Protocol 3: Secure Beaver multiplication with u and w shares, single exchange
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