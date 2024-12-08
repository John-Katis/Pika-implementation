use crate::mpc_party::MPCParty;
use fss::*;
use fss::RingElm;
use crate::offline_data::*;

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>) -> Vec<RingElm> {

    // Protocol 2(a): reconstruct x = (r - a) mod 2^k 
    let r = p.offlinedata.r_share[0]; // Retreive r share
    let a = p.offlinedata.x_share[0]; // Retreive a share

    // Compute this party's share of x locally
    let local_x = r.wrapping_sub(a); // (r - a) mod 2^k 

    // Exchange the shares of x with the other party and get the reconstructed x (publicly known mask)
    let x: u16  = p.netlayer.exchange_u16_vec(vec![local_x]).await[0];


    // Protocol 2(b): compute yσ (EvalAll routine -> implement in DPF key)
    let dpf_key = &p.offlinedata.k_share[0]; // Each party retrieves its DPF key

    // Each party evaluates their DPF keys to obtain yσ contains non-zero value at one index corresponding to a)
    let y_sigma = dpf_key.eval_all(); 


    // Protocol 2(c): compute u
    // Load the function database
    let func_db = load_func_db();

    // Initialize the v_sum to zero
    let mut v_sum = RingElm::zero(); 

    // Compute the sum: v = Σ(y[i+x] * func_db[i])
    for i in 0..func_db.len() {
        // Compute shifty for y_sigma c
        let shift_index = i.wrapping_add(x as usize) % y_sigma.len();

        // Shift y_sigma with shift_index
        let y_val = if y_sigma[shift_index] { RingElm::one() } else { RingElm::zero() };

        // Access the function database value
        let func_val = RingElm::from(func_db[i] as u32);

        // Accumulate the product into v_sum - no party knows which value is being retreived
        v_sum = v_sum + (y_val * func_val);
    }

    // Set sigma according to the party
    let sigma: bool = p.netlayer.is_server;

    // Determine the scaling factor (-1)^sigma which ensures output is correctly reconstructed in protocol 3
    let scaling_factor = if sigma {
        let mut neg_one = RingElm::one();
        neg_one.negate(); // Negate 1 to get -1 - server
        neg_one
    } else {
        RingElm::one() // 1 if sigma is false - client
    };

    // Scale the sum using the scaling factor (-1)^sigma * v_sum
    let v = scaling_factor * v_sum; 


    // Protocol 3 - output Beaver triple (v * w) (compute v * w secretly without revealing them to either party)

    // Retrieve w share (sign bit) for this party
    let w_share = p.offlinedata.w_share[0];

    // Retrieve the Beaver triple for this computation
    let beaver_triple = &mut p.offlinedata.beavers[0];

    // Compute delta values (serialized into Vec<u8>) (difference between v*w and values derived from beaver triple)
    let delta_values_u8 = beaver_triple.beaver_mul0(v, w_share);

    // Wrap delta_values_u8 into a Vec<Vec<u8>> for exchange_byte_vec
    let delta_values_vec_u8 = vec![delta_values_u8];

    // Exchange serialized delta values between parties (so that each party can complete the multiplication without learning the others values)
    let exchanged_deltas_vec = p.netlayer.exchange_byte_vec(&delta_values_vec_u8).await;

    // Extract the exchanged Vec<u8> from the Vec<Vec<u8>>
    let exchanged_deltas_u8 = &exchanged_deltas_vec[0];

    // Combine shared inputs (v and w) with the precomputed Beaver triple to output the party's share of the final product
    let result = beaver_triple.beaver_mul1(p.netlayer.is_server, exchanged_deltas_u8);

    vec![result] // Return the result (party's share of the product)

}


// Load function database (sigmoid, tanh, ReLU)
fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    match read_file("../data/tanh_table.bin") {
        Ok(value) => ret = value,
        Err(e) => println!("Error reading file: {}", e),  
    }
    ret
}