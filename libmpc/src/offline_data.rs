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
use std::mem;
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

pub struct BasicOffline {
    // seed: PrgSeed,
    pub k_share: Vec<DPFKey<bool>>, //dpf keys
    pub x_share: Vec<u16>, //share of input x
    pub r_share: Vec<u16>, //alpha
    pub w_share: Vec<RingElm>,
    pub beavers: Vec<BeaverTuple>
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

    pub fn gen_data(&self, input_bool_vectors: Vec<Vec<bool>>){
        
        // Input X
        let quantized_x = &input_bool_vectors[index][0..input_bool_vectors[index].len()/2];

        // Setting seed to generate randomness
        let seed = PrgSeed::random();
        let mut stream = FixedKeyPrgStream::new();
        stream.set_key(&seed.key);

        // Generating random bits - 
        // enough for randomness for all 3 parties and generating
        // shares of x and w
        let share_gen_bits = stream.next_bits(3*BOUNDED_DOMAIN+INPUT_DOMAIN);

        // This will be used as input to the function
        // that generates the DPF keys
        let beta: bool = true;

        // Initializing vactors in which the shares of the values will be stored
        // These need to be vectors for the write_file and read_file functions to work
        let mut xVec0: Vec<u16> = Vec::new();
        let mut xVec1: Vec<u16> = Vec::new();

        let mut rVec_0: Vec<u16> = Vec::new();
        let mut rVec_1: Vec<u16> = Vec::new();

        let mut dpf_0: Vec<DPFKey<bool>> = Vec::new();
        let mut dpf_1: Vec<DPFKey<bool>> = Vec::new();

        let mut wVec_0: Vec<RingElm> = Vec::new();

        let mut wVec_1: Vec<RingElm> = Vec::new();

        let beaver_size: usize = 1;
        let mut beavertuples0 = Vec::new();
        let mut beavertuples1 = Vec::new();
        
        
        //TODO implement pika protocol steps 0 & 1 by generating the following:

        // 1. SPLIT INPUT X INTO SHARES
        // 2. EXTRACT r, r0, r1 - r USED BY THE DEALER, r0, r1 SHARES FOR EACH PARTY
        // 3. DPF KEYS BASED ON R - EXTRACT CONTROL BIT
        // 4. W BIT BASED ON CONTROL BIT
        // 5. BEAVER TRIPLE

        // 6. FUNCTION TRUTH TABLE 

        // 7. USE write_file TO STORE THE VECTORS WITH THE VALUES
    }
}

// 6.1 FUNCTION IMPLEMENTATIONS (e.g., tanh, sigmoid, ReLU)