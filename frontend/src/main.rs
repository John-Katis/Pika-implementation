use libmpc::mpc_party::MPCParty;
use libmpc::protocols::pika::*;
use libmpc::mpc_platform::NetInterface;
use libmpc::offline_data::BasicOffline;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::env;
use std::time::Instant;
use std::time::Duration;

//const LAN_ADDRESS: &'static str = "127.0.0.1:8088";
//const WAN_ADDRESS: &'static str = "192.168.1.1:8088";
const LAN_ADDRESS: &'static str = "192.168.1.1:8088";

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

    // Variables to store benchmark results
    let mut offline_times = Vec::new();
    let mut online_times = Vec::new();
    let mut comm_rounds = Vec::new();
    let mut data_received_kb = Vec::new();
 
    // ------- Run the protocol 100 times --------
    let iterations = 100;
    for _ in 0..iterations {
        // OFFLINE PHASE
        let offline_time = gen_offlinedata(input_vec.clone()).as_secs_f32();
        p.offlinedata.load_data(&index_id);
        p.netlayer.reset_timer().await;
 
        // ONLINE PHASE
        let online_start = Instant::now(); // Start timer for online phase
 
        let pika_result; // Declare pika_result
 
        // Check which party it is (P0 or P1) and run onlie phase
        if is_server {
            pika_result = pika_eval(&mut p).await;
        } else {
            pika_result = pika_eval(&mut p).await;
        }

        let online_duration = online_start.elapsed().as_secs_f32(); // Calculate online phase duration
 
        // BENCHMARKING
        let benchmarking_stats = p.netlayer.return_benchmarking().await;
 
        // Store benchmark results for this iteration
        offline_times.push(offline_time);
        online_times.push(online_duration);
        comm_rounds.push(benchmarking_stats[1] as f32 + 1.0); // add 1 round for offline
        data_received_kb.push(benchmarking_stats[2]);
    
        // Output of each party for each run
        println!("Pika Evaluation Result: {:?}", pika_result);

        // -------------- Test ------------------
        let scaled_input = (323232123 as f32 / (1 << 16) as f32) / (1 << 9) as f32;

        let target = tanh(scaled_input);

        println!("Target value: {}", target);

        // Given server and client values
        let combined = (335422031 as u64 + 987373934 as u64) % (1u64 << 32);
        let normalized_result = combined as f32 / (1u64 << 32) as f32;

        // Print the normalized result
        println!("Normalized Result: {}", normalized_result);
    }
 
    // Compute mean benchmarks
    let mean_offline_time = offline_times.iter().sum::<f32>() / iterations as f32;
    let mean_online_time = online_times.iter().sum::<f32>() / iterations as f32;
    let mean_total_time = (offline_times.iter().sum::<f32>() + online_times.iter().sum::<f32>()) / iterations as f32;
    let mean_comm_rounds = comm_rounds.iter().sum::<f32>() / iterations as f32;
    let mean_data_received_kb = data_received_kb.iter().sum::<f32>() / iterations as f32;
 
    // Print mean benchmarks
    println!("------- Mean Benchmarking Results ---------");
    println!("Offline Phase Mean Duration: {:.6} seconds", mean_offline_time);
    println!("Online Phase Mean Duration: {:.6} seconds", mean_online_time);
    println!("Total Mean Elapsed Time: {:.6} seconds", mean_total_time);
    println!("Mean Rounds of Communication: {:.3}", mean_comm_rounds);
    println!("Mean Data Received: {:.6} KB", mean_data_received_kb);
}

// Tanh: f(x) = (e^(x) - e^(-x)) / (e^(x) + e^(-x))
fn tanh(x: f32) -> f32 {
    x.tanh()
}


// Creates the offline object and calls the method to create the data shares
fn gen_offlinedata(input_bool_vectors: Vec<Vec<bool>>) -> Duration {
    let offline = BasicOffline::new();
    let start = Instant::now();
    offline.gen_data(input_bool_vectors);    
    start.elapsed() // Return the elapsed time as `Duration`
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