use crate::mpc_party::MPCParty;
use fss::*;
use fss::RingElm;
use crate::offline_data::*;

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>) -> Vec<RingElm> {

    // Protocol 2(a): reconstruct x = (r - a) mod 2^k to retreive the correct value corresponding to a
    let r = p.offlinedata.r_share[0]; // Retreive r share
    let a = p.offlinedata.x_share[0]; // Retreive a share

    // Compute this party's share of x locally
    let local_x = r.wrapping_sub(a); // (r - a) mod 2^k (where k is 16)

    // Exchange the shares of x with the other party and get the reconstructed x (publicly known mask)
    let x: u16  = p.netlayer.exchange_u16_vec(vec![local_x]).await[0];


    // Protocol 2(b): compute yσ (evaluation of the DPF keys)
    let dpf_key = &p.offlinedata.k_share[0]; // Each party retrieves its DPF key

    // Each party evaluates their DPF keys to obtain yσ contains non-zero value at one index corresponding to shares of r
    let y_sigma = dpf_key.eval_all(); 


    // Protocol 2(c): compute dot product with function database and the shifted y_sigma
    // Load the function database
    let func_db = load_func_db();

    // Initialize the v_sum to zero
    let mut v_sum = RingElm::zero(); 

    // Compute the sum: v = Σ(y[i+x] * func_db[i]) for each element in function database
    for i in 0..func_db.len() {
        // Compute shift x for element i in y_sigma (addition wraps around if it exeeds maximum value of integer and % ensures that the index remains withing the bounds of the array y_sigma)
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

    // Determine the scaling factor (-1)^sigma which ensures the parties outputs are correctly combined in protocol 3
    let scaling_factor = if sigma {
        let mut neg_one = RingElm::one();
        neg_one.negate(); // Negate 1 to get -1 - client
        neg_one
    } else {
        RingElm::one() // 1 if sigma is false - server
    };

    // Scale the sum using the scaling factor (-1)^sigma * v_sum
    let v = scaling_factor * v_sum;

    // Protocol 3 - output Beaver multiplication result 
    // (compute v * w secretly without revealing them to either party)

    // Retrieve w share (ensures correcrt sign bit of the output) as input for the beaver multiplication
    let w_share = p.offlinedata.w_share[0];

    // Retrieve Beaver triple share (a,b,c) (a*b=c) (a,b,c are random values)
    let beaver_triple = &mut p.offlinedata.beavers[0];

    // Compute masked inputs (delta values) (delta_v = v - a) (delta_w = w - b) (neither v nor w are exposed directly)
    let delta_values_u8 = beaver_triple.beaver_mul0(v, w_share);

    // Wrap delta_values_u8 into a Vec<Vec<u8>> for exchange_byte_vec
    let delta_values_vec_u8 = vec![delta_values_u8];

    // Exchange delta values between parties (so that each party can complete the multiplication without learning the others values)
    let exchanged_deltas_vec = p.netlayer.exchange_byte_vec(&delta_values_vec_u8).await;

    // Extract the exchanged Vec<u8> from the Vec<Vec<u8>>
    let exchanged_deltas_u8 = &exchanged_deltas_vec[0];

    // Combine and compute final shares (result = c + deleta_v*b + delta_w*a + exchanged_delta_v*echanged_delta_w)
    let result = beaver_triple.beaver_mul1(p.netlayer.is_server, exchanged_deltas_u8);

    
    
    // println!("\n---------- Correctness Checks ----------");

    // // Log this party's Beaver share
    // println!("THIS PARTY BEAVER:");
    // result.print();
    // println!("");

    // let this_party_beaver: Vec<RingElm> = vec![result]; 
    // let beaver_comb = p.netlayer.exchange_ring_vec(this_party_beaver).await;

    // println!("EXCHANGE VALUE");
    // beaver_comb[0].print();
    // println!("");

    // // Print the binary representation of the exchanged value
    // println!("EXCHANGE VALUE BITS:");
    // println!("{:b}", beaver_comb[0].to_u32().unwrap());

    // // Convert the combined value to f32
    // let mut beaver_result: f32 = beaver_comb[0].to_u32().unwrap() as f32;
    // let f32_number = beaver_result / (1 << 16) as f32;

    // // Print the converted and normalized values
    // println!("Original u32 number as f32: {}", beaver_result);
    // println!("Interpreted f32 number: {}", f32_number);

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