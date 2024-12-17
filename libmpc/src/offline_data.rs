use fss::beavertuple::BeaverTuple;
use fss::dpf::*;
use fss::RingElm;
use fss::Group;
use fss::prg::PrgSeed;
use fss::bits_to_u32;
use fss::prg::FixedKeyPrgStream;
use bincode::Error;
use std::fs::File;
use std::io::Write;
use std::io::Read;
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


// FUNCTION IMPLEMENTATIONS 
// Sigmoid: f(x) = 1 / (1 + e^(-x))
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

// Tanh: f(x) = (e^(x) - e^(-x)) / (e^(x) + e^(-x))
fn tanh(x: f32) -> f32 {
    x.tanh()
}

// ReLU: f(x) = max(0, x)
fn relu(x: f32) -> f32 {
    x.max(0.0)
}

// Function to generate the function truth table
fn generate_function_table<F>(func: F) -> Vec<f32>
where
    F: Fn(f32) -> f32,
{
    // Initialize a vector with enough elements
    let mut table = Vec::with_capacity(65534);

    // Step 0: Iterate from 0 to maximum value for a 16-bit unsigned integer 65534 (all possible bit patterns form 16-bit numbers as the input domain is 2^16)
    for i in 0..u16::MAX {
        // Step 1: Derive the sign of the integer according to MSBs (allows to wrok with positive and neg. values)
        let sign = i & (1 << 15) != 0;

        // Step 2: Extract the rest 15 bits (magnitude)
        let rest_bits = i & !(1 << 15);

        // Step 3: Based on the sign, create either a positive or negative number as f32 (to avoid error accumulation)
        let mut f32_number = if sign {
            -(rest_bits as f32) / (1 << 9) as f32 // scale to fixed-point representation to [-63.999, -0.001]
        } else {
            rest_bits as f32 / (1 << 9) as f32 // scale to fixed-point representation [0.0, 63.999]
        };

        // Handle special case - 32768 has special bit pattern that is 
        // evaluated to −0.0 therefore set value is manually set to 64.0
        if i == (u16::MAX / 2) + 1 {
            f32_number = 64f32;
        };

        // Step 4: Evaluate the positive or negative f32 fixed-point number 
        let truth_val = func(f32_number);
        table.push(truth_val); // push truth value into the table
    }
    table
}

// Save function tables to file
fn save_function_table<T: Serialize>(path: &str, table: &T) {
    let serialized_data = bincode::serialize(table).expect("Failed to serialize function table");
    let mut file = File::create(path).expect("Failed to create file");
    file.write_all(&serialized_data).expect("Failed to write table data to file");
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
            Err(e) => println!("Error reading beaver tuple file: {}", e),  
        }

        match read_file("../data/overhead.bin") {
            Ok(value) => self.overhead = value,
            Err(e) => println!("Error reading beaver tuple file: {}", e),  
        }
    }

    // Implementation for pika protocol offline steps 0 & 1 
    pub fn gen_data(&self, input_bool_vectors: Vec<Vec<bool>>){
        // Start the timer to measure overhead
        let start_time = Instant::now();

        // Loop through each input bool vector and extract quantized_x (first half of boolean representaiton of input - 16 first most significan bits to stay in input domain ) 
        // (in this project for simplicity we have only one input value) 
        for (index, _quantized_x) in input_bool_vectors.iter().enumerate() {

            // Input X
            let quantized_x = &input_bool_vectors[index][0..input_bool_vectors[index].len()/2];
            
            // Setting seed to generate randomness
            let seed = PrgSeed::random();
            let mut stream = FixedKeyPrgStream::new();
            stream.set_key(&seed.key);
        
            // Generating random bits - enough randomness for all 3 parties 
            let share_gen_bits = stream.next_bits(3*BOUNDED_DOMAIN+INPUT_DOMAIN);

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
            // Select 16 random bits from the first bounded domain
            let x0_bits = &share_gen_bits[0 * BOUNDED_DOMAIN..1 * BOUNDED_DOMAIN]; 

            // Initialize variables to accumulate bits
            let mut x0_accumulator = 0u16; 
            let mut x1_accumulator = 0u16; 
            let mut bit_count = 0;

            // Loop over quantized_x bits and x0 share bits
            for (&bit, &x0_bit) in quantized_x.iter().zip(x0_bits.iter()) {
                let x1_bit = bit ^ x0_bit; // Compute x1 bits using XOR (x - x0) because (x = x0 + x1)

                // Accumulate bits to the correct position into a complete 16-bit integer 
                x0_accumulator |= (x0_bit as u16) << bit_count;
                x1_accumulator |= (x1_bit as u16) << bit_count;
                bit_count += 1;

                // If the accumulator is full, push it
                if bit_count == 16 {
                    x_vec0.push(x0_accumulator);
                    x_vec1.push(x1_accumulator);
                }
            }

            // Save the shares of x for each party
            write_file(&format!("../data/x{}.bin", 0), &x_vec0);
            write_file(&format!("../data/x{}.bin", 1), &x_vec1);


            // 2. EXTRACT r, r0, r1 - r USED BY P2, r0, r1 SHARES FOR EACH PARTY P0 and P1
            // Generate random r bits and r0 bits
            let r_bits = &share_gen_bits[1 * BOUNDED_DOMAIN..2 * BOUNDED_DOMAIN];
            let r0_bits = &share_gen_bits[2 * BOUNDED_DOMAIN..3 * BOUNDED_DOMAIN];

            // Initialize variables to accumulate bits
            let mut r0_accumulator = 0u16; 
            let mut r1_accumulator = 0u16; 
            let mut bit_count = 0;

            // Loop over r bits and r_0 bits
            for (&r_bit, &r0_bit) in r_bits.iter().zip(r0_bits.iter()) {
                let r1_bit = r_bit ^ r0_bit; // Compute r1 bits using XOR (r - r0) because (r = r0 + r1)

                // Accumulate bits to the correct position into a complete 16-bit integer 
                r0_accumulator |= (r0_bit as u16) << bit_count;
                r1_accumulator |= (r1_bit as u16) << bit_count;
                bit_count += 1;

                // If the accumulator is full, push it
                if bit_count == 16 {
                    r_vec_0.push(r0_accumulator);
                    r_vec_1.push(r1_accumulator);
                }
            }

            // Save the shares of r for each party
            write_file(&format!("../data/r{}.bin", 0), &r_vec_0);
            write_file(&format!("../data/r{}.bin", 1), &r_vec_1);


            // 3. DPF KEYS BASED ON R - EXTRACT CONTROL BIT
            // Convert array of bits to a vector (representing the target index for DPF)
            let alpha_bits = r_bits.to_vec();

            // Generate the DPF keys (DPF - evaluated to beta (1) at alpha and 0 everywhere else)
            let (dpf_key0, dpf_key1, _control_bit) = DPFKey::<bool>::gen(&alpha_bits, &beta);

            // Store the DPF keys
            dpf_0.push(dpf_key0);
            dpf_1.push(dpf_key1);

            // Save the DPF key for each party
            write_file(&format!("../data/k{}.bin", 0), &dpf_0);
            write_file(&format!("../data/k{}.bin", 1), &dpf_1);


            // 4. W BIT ("sign bit") BASED ON CONTROL BIT (used in online phase for correct beaver multiplication output)
            
            // Generate 32 random w0 bits (since it's used for secure multiplication)
            let w0_bits = &share_gen_bits[3*BOUNDED_DOMAIN..3*BOUNDED_DOMAIN+INPUT_DOMAIN];

            // Convert the 32-bit boolean sequence to a u32 integer
            let w0_value = bits_to_u32(&w0_bits);

            // Convert the u32 value to a `RingElm`
            let w0 = RingElm::from(w0_value);

            // Based on the value of control bit w is set to 1/-1 and later on indicates if final result should be positive/negative
            let mut neg_one = RingElm::one();
            neg_one.negate();
            let w = if _control_bit { RingElm::one() } else { neg_one };

            // Compute second share w1 and ensure w = w0 + w1
            let w1 = w - w0;

            // Store the shares
            w_vec_0.push(w0);
            w_vec_1.push(w1);

            // Save the W bit for each party
            write_file(&format!("../data/w{}.bin", 0), &w_vec_0);
            write_file(&format!("../data/w{}.bin", 1), &w_vec_1);


            // 5. BEAVER TRIPLE (enable secure multiplication in the online phase without any party knowing the actual inputs or product during computation)
            // Generate one beaver triple (beaver_size = 1) and splits it into shares
            for _ in 0..beaver_size {
                BeaverTuple::gen_beaver(&mut beavertuples0, &mut beavertuples1, &seed);
            }
            
            // Save Beaver triples
            write_file(&format!("../data/bvt{}.bin", 0), &beavertuples0);
            write_file(&format!("../data/bvt{}.bin", 1), &beavertuples1);
        }

        // 6. FUNCTION TRUTH TABLE
        // Generate the function truth table
        let tanh_table = generate_function_table(tanh);
        //let sigmoid_table = generate_function_table(sigmoid);
        //let relu_table = generate_function_table(relu);

        // Save the truth table to a file
        save_function_table("../data/tanh_table.bin", &tanh_table);
        //save_function_table("../data/sigmoid_table.bin", &sigmoid_table);
        //save_function_table("../data/relu_table.bin", &relu_table);

        // End timer and calculate overhead
        let overhead = start_time.elapsed().as_secs_f32();

        // Save the overhead value
        write_file("../data/overhead.bin", &overhead);
    }
}