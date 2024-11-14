use fss::beavertuple::BeaverTuple;
use fss::dpf::*;
use fss::RingElm;
use fss::Group;
use fss::prg::PrgSeed;
use fss::{bits_to_u32,bits_to_u16};
use fss::prg::FixedKeyPrgStream;
use bincode::Error;
use std::fs::File;
use std::io::Write;
use std::io::Read;
use fss::Share;
//use std::mem;
use std::time::Instant;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub const INPUT_DOMAIN:usize = 32;
pub const BOUNDED_DOMAIN:usize = 16;


pub fn write_file<T: serde::ser::Serialize>(path:&str, value:&T){
    let mut file = File::create(path).expect("create failed");
    file.write_all(&bincode::serialize(&value).expect("Serialize value error")).expect("Write key error.");
}

pub fn read_file<T: DeserializeOwned>(path: &str) -> Result<T, Error> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let value = bincode::deserialize(&buf)?;
    Ok(value)
}

// 6.1 FUNCTION IMPLEMENTATIONS (e.g., tanh, sigmoid, ReLU)
// Quantize each function into the range [-100, 100]
// Sigmoid function table generator
fn generate_sigmoid_table() -> Vec<f32> {
    (-100..=100).map(|i| 1.0 / (1.0 + (-((i as f32) / 10.0)).exp())).collect()
}

// Tanh function table generator
fn generate_tanh_table() -> Vec<f32> {
    (-100..=100).map(|i| ((i as f32) / 10.0).tanh()).collect()
}

// ReLU function table generator
fn generate_relu_table() -> Vec<f32> {
    (-100..=100).map(|i| ((i as f32) / 10.0).max(0.0)).collect()
}

// Save function tables to file
fn save_function_table<T: Serialize>(path: &str, table: &T) {
    let serialized_data = bincode::serialize(table).expect("Failed to serialize function table");
    let mut file = File::create(path).expect("Failed to create file");
    file.write_all(&serialized_data).expect("Failed to write table data to file");
}

// Generate and save the truth tables for each function
pub fn save_function_tables() {
    let sigmoid_table = generate_sigmoid_table();
    save_function_table("../data/sigmoid_table.bin", &sigmoid_table);

    let tanh_table = generate_tanh_table();
    save_function_table("../data/tanh_table.bin", &tanh_table);

    let relu_table = generate_relu_table();
    save_function_table("../data/relu_table.bin", &relu_table);
}

pub struct BasicOffline {
    // seed: PrgSeed,
    pub k_share: Vec<DPFKey<bool>>, //dpf keys
    pub x_share: Vec<u16>, //share of input x
    pub r_share: Vec<u16>, //alpha
    pub w_share: Vec<RingElm>,
    pub beavers: Vec<BeaverTuple>,
    pub overhead: f32 // define overhead
}

impl BasicOffline{
    pub fn new() -> Self{
        Self{k_share: Vec::new(), x_share: Vec::new(), r_share: Vec::new(), w_share: Vec::new(), beavers: Vec::new(), overhead: 0f32}
    }

    pub fn load_data(&mut self,idx:&u8){
        match read_file(&format!("../data/k{}.bin", idx)) {
            Ok(value) => self.k_share = value,
            Err(e) => println!("Error reading key file: {}", e),
        }

        match read_file(&format!("../data/x{}.bin", idx)) {
            Ok(value) => self.x_share = value,
            Err(e) => println!("Error reading a share file: {}", e)
        }

        match read_file(&format!("../data/r{}.bin", idx)) {
            Ok(value) => self.r_share = value,
            Err(e) => println!("Error reading a share file: {}", e)
        }

        match read_file(&format!("../data/w{}.bin", idx)) {
            Ok(value) => self.w_share = value,
            Err(e) => println!("Error reading w share file: {}", e)
        }

        match read_file(&format!("../data/bvt{}.bin", idx)) {
            Ok(value) => self.beavers = value,
            Err(e) => println!("Error reading beaver tuple file: {}", e),  // Or handle the error as needed
        }

        match read_file("../data/overhead.bin") {
            Ok(value) => self.overhead = value,
            Err(e) => println!("Error reading beaver tuple file: {}", e),  // Or handle the error as needed
        }
    }

    // Implementation for pika protocol steps 0 & 1 
    pub fn gen_data(&self, input_bool_vectors: Vec<Vec<bool>>){
        // Start the timer to measure overhead
        let start_time = Instant::now();

        // Helper function for conversion from a u16 integer to a boolean vector
        fn u16_to_boolean_vector(num: u16) -> Vec<bool> {
            (0..16).map(|i| ((num >> i) & 1) == 1).rev().collect()
        }

        // Loop through each quantized input vector and perform tasks 1-6
        for (index, _quantized_x) in input_bool_vectors.iter().enumerate() {

            // Input X
            let quantized_x = &input_bool_vectors[index][0..input_bool_vectors[index].len()/2];

            // Setting seed to generate randomness
            let seed = PrgSeed::random();
            let mut stream = FixedKeyPrgStream::new();
            stream.set_key(&seed.key);
        
            // Generating random bits - enough for randomness for all 3 parties and generating
            // shares of x and w
            let mut share_gen_bits = stream.next_bits(3*BOUNDED_DOMAIN+INPUT_DOMAIN);

            // This will be used as input to the function that generates the DPF keys (only true value for DPF)
            let beta: bool = true;

            // Initializing vactors in which the shares of the values will be stored
            // These need to be vectors for the write_file and read_file functions to work
            let mut x_vec0: Vec<u16> = Vec::new();
            let mut x_vec1: Vec<u16> = Vec::new();

            let mut r_vec_0: Vec<u16> = Vec::new();
            let mut r_vec_1: Vec<u16> = Vec::new();

            let mut dpf_0: Vec<DPFKey<bool>> = Vec::new();
            let mut dpf_1: Vec<DPFKey<bool>> = Vec::new();

            let mut w_vec_0: Vec<RingElm> = Vec::new();
            let mut w_vec_1: Vec<RingElm> = Vec::new();

            let beaver_size: usize = 1;
            let mut beavertuples0 = Vec::new();
            let mut beavertuples1 = Vec::new();


            // 1. SPLIT INPUT X INTO SHARES
            // Loop through each bit in the quantized_x
            for &bit in quantized_x.iter() {
                // Convert each bit (true or false) from `quantized_x` to a `RingElm` (0 or 1)
                let value: RingElm = if bit { RingElm::from(1u32) } else { RingElm::from(0u32) };

                // Ensure there are enough bits to generate a random share before draining
                if share_gen_bits.len() < 16 {
                    // Adds more random bits if there are not enough
                    share_gen_bits.extend(stream.next_bits(3 * BOUNDED_DOMAIN + INPUT_DOMAIN));
                }
                
                // Drain 16 bits from `share_gen_bits` for `x0`
                let x0_bits = share_gen_bits.drain(0..16).collect::<Vec<bool>>();
                let x0_value = bits_to_u32(&x0_bits);

                // Convert x0 value to `RingElm` 
                let x0_ringelm = RingElm::from(x0_value);

                // Compute x1 and ensure `value = x0 + x1`
                let x1_ringelm = value - x0_ringelm;

                // Convert RingElm shares to u16 and store
                x_vec0.push(x0_ringelm.to_u32().unwrap() as u16); // Party 0's share of x
                x_vec1.push(x1_ringelm.to_u32().unwrap() as u16); // Party 1's share of x
            }

            // Save the shares of x for each party
            write_file(&format!("../data/x{}.bin", 0), &x_vec0);
            write_file(&format!("../data/x{}.bin", 1), &x_vec1);


            // 2. EXTRACT r, r0, r1 - r USED BY THE DEALER, r0, r1 SHARES FOR EACH PARTY
            // Extract r form a random bit stream
            let r: u32 = bits_to_u32(&stream.next_bits(32)); 
            
            // Convert r to a ring element
            let r_ringelm = RingElm::from(r);
            
            // Split r into two shares r0 and r1
            let (r0_ringelm, r1_ringelm) = r_ringelm.share(); 

            // Convert RingElm shares to u16 and store
            r_vec_0.push(r0_ringelm.to_u32().unwrap() as u16);
            r_vec_1.push(r1_ringelm.to_u32().unwrap() as u16);

            // Save the shares of r for each party
            write_file(&format!("../data/r{}.bin", 0), &r_vec_0);
            write_file(&format!("../data/r{}.bin", 1), &r_vec_1);


            // 3. DPF KEYS BASED ON R - EXTRACT CONTROL BIT
            // Generate a random 16-bit value for the control bit (target index)
            let r = bits_to_u16(&stream.next_bits(16));

            // Convert integer to a boolean vector (representing the control bit (target index))
            let alpha_bits = u16_to_boolean_vector(r);

            // Generate the DPF keys
            let (dpf_key0, dpf_key1, _control_bit) = DPFKey::<bool>::gen(&alpha_bits, &beta);

            // Store the DPF keys
            dpf_0.push(dpf_key0);
            dpf_1.push(dpf_key1);

            // Save the DPF key for each party
            write_file(&format!("../data/k{}.bin", 0), &dpf_0);
            write_file(&format!("../data/k{}.bin", 1), &dpf_1);


            // 4. W BIT ("sign bit") BASED ON CONTROL BIT (used as control bit to verify computation's output)
            // Drain 16 bits for the first share
            let w0_bits = share_gen_bits.drain(0..16).collect::<Vec<bool>>();

            // Convert to a u32 integer
            let w0_value = bits_to_u32(&w0_bits);

            // Convert to a ring element
            let w0 = RingElm::from(w0_value);

            // Based on the value of control bit beta w is set to true or false
            let w = if beta { RingElm::one() } else { RingElm::zero() };

            // Compute second share w1 and ensure w = w0 + w1
            let w1 = w - w0;

            // Store the shares
            w_vec_0.push(w0);
            w_vec_1.push(w1);

            // Save the W bit for each party
            write_file(&format!("../data/w{}.bin", 0), &w_vec_0);
            write_file(&format!("../data/w{}.bin", 1), &w_vec_1);


            // 5. BEAVER TRIPLE (enable efficient secure multiplication in the online phase)
            // Generate beaver triples
            for _ in 0..beaver_size {
                BeaverTuple::gen_beaver(&mut beavertuples0, &mut beavertuples1, &seed);
            }
            
            // Save Beaver triples
            write_file(&format!("../data/bvt{}.bin", 0), &beavertuples0);
            write_file(&format!("../data/bvt{}.bin", 1), &beavertuples1);
        }

        // 6. FUNCTION TRUTH TABLE
        save_function_tables();

        // End timer and calculate overhead
        let overhead = start_time.elapsed().as_secs_f32();

        // Save the overhead value
        write_file("../data/overhead.bin", &overhead);
    }
}