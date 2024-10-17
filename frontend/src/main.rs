use libmpc::mpc_party::MPCParty;
use libmpc::protocols::pika::*;
use libmpc::mpc_platform::NetInterface;
use libmpc::offline_data::BasicOffline;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::env;
use std::time::Instant;
use std::time::Duration;

const LAN_ADDRESS: &'static str = "127.0.0.1:8088";
const WAN_ADDRESS: &'static str = "192.168.1.1:8088";

#[tokio::main]
async fn main() {
    // Boolean for server
    let mut is_server=false;


    // Parsing command line input (run with 0 creates the server)
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // The first command-line argument (index 1) is accessed using args[1]
        let first_argument = args[1].parse::<u8>();

        // Check if the parsing was successful
        match first_argument {
            Ok(value) => {
                match value{
                    0 => is_server = true,
                    1 => {},
                    _ => eprintln!("Error: Party role illegale"),
                }
            }
            Err(_) => {
                eprintln!("Error: Unable to parse the first argument as a u8 value.");
            }
        }
    } else {
        eprintln!("No arguments provided.");
    }


    // Initializing required objects for communication and computation

    let index_id = if is_server{0u8} else {1u8};
    let netlayer = NetInterface::new(is_server,LAN_ADDRESS).await;
    let offlinedata = BasicOffline::new();
    let mut p: MPCParty<BasicOffline> = MPCParty::new(offlinedata, netlayer);
    p.setup(10, 10);


    // Reading input from file

    let mut input_vec: Vec<Vec<bool>> = Vec::new();
    match read_bool_vectors_from_file("../input/input1.txt") {
        Ok(u32_vector) => { input_vec = u32_vector; }
        Err(e) => { eprintln!("Error: {}", e); }
    }

    
    // OFFLINE PHASE
    // creating keys, randomness and input shares

    let offline_time: f32 = gen_offlinedata(input_vec).as_secs_f32();
    p.offlinedata.load_data(&index_id);
    p.netlayer.reset_timer().await;


    // ONLINE PHASE

    if is_server{
        pika_eval(&mut p).await;
    }else{
        pika_eval(&mut p).await;
    }

    // BENCHMARKING statistics can be gathered here
    // OUTPUT can be reconstructed here

}


// Creates the offline object and calls the method to create the data shares
fn gen_offlinedata(input_bool_vectors: Vec<Vec<bool>>){
    let offline = BasicOffline::new();
    offline.gen_data(input_bool_vectors);    
}


// Read an integer as a boolean vector (0 and 1)
fn read_bool_vectors_from_file(file_path: &str) -> io::Result<Vec<Vec<bool>>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut bool_vector: Vec<Vec<bool>> = Vec::new();

    for line in reader.lines() {
        let value_str = line?;

        match value_str.trim().parse::<i32>() {
            
            Ok(value_i32) => {
                let value_u32 = if value_i32 >= 0 {
                    value_i32 as u32
                } else {
                    // Flip the sign bit for negative numbers
                    (value_i32.abs() as u32) ^ (1 << 31)
                };

                let bools = u32_to_bool_vector(value_u32);
                bool_vector.push(bools);
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Error parsing line '{}': {}", value_str, e)));
            }
        }
    }

    Ok(bool_vector)
}


// Helper for above function
// Turns a u32 type to a boolean vector
fn u32_to_bool_vector(value: u32) -> Vec<bool> {
    let bytes = value.to_be_bytes();
    let mut bool_vector = Vec::new();

    for byte in bytes.iter() {
        for i in (0..8).rev() {
            bool_vector.push((byte & (1 << i)) != 0);
        }
    }

    bool_vector
}
