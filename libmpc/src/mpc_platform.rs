use tokio::{
    io::{AsyncWriteExt, AsyncReadExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    //task;
};
use fss::RingElm;
use fss::Group;
//use async_trait::async_trait;
use std::time::Instant;
use std::time;

// #[derive(Clone)]
pub struct NetInterface{
    //pub listener: TcpListener,
    pub is_server: bool,
    pub reader: OwnedReadHalf,
    pub writer: OwnedWriteHalf,
    pub received:usize,
    pub rounds_occured:u32,
    pub timer:Instant
}

impl NetInterface{
    pub async fn new(isserver: bool, addr: &str)->NetInterface{
        if isserver{
            let listner = TcpListener::bind(addr).await.unwrap();
            println!("***Start Listening ......***");
            let (c, caddr) = listner.accept().await.unwrap();
            println!("Accept from {:?}", caddr);
            let (r, w) = c.into_split();

            NetInterface{
                is_server: true,
                reader: r,
                writer: w,
                received:  0usize,
                rounds_occured:  0u32,
                timer: Instant::now()
            }
        }
        else{
            std::thread::sleep(time::Duration::from_millis(15));
            let s = TcpStream::connect(addr).await.unwrap();
            let (r, w) = s.into_split();
            println!("Connect to {} success.", addr);
            NetInterface{
                is_server: false,
                reader: r,
                writer: w,
                received:  0usize,
                rounds_occured:  0u32,
                timer: Instant::now()
            }
        }
    }
    
    pub async fn reset_timer(&mut self){
        self.timer = Instant::now();
        self.received = 0usize;
        self.rounds_occured = 0u32;
    }

    pub async fn return_benchmarking(&mut self)-> Vec<f32> {
        //TODO imlpement the function that returns:
        // 1. The time elapsed for the computation
        // 2. The rounds of communication occured
        // 3. The amount of data the party received
    }

    pub async fn exchange_a_bool(&mut self, msg: bool)->bool{
        let mut buf: [u8; 1] = [0; 1];
        
        let mut x_msg: Vec<u8> = Vec::new();
        if msg{
            x_msg.push(0x1u8);
        } // convert the bool vec to u8 vec such that the message can be convoyed in the channel
        else{
            x_msg.push(0x0u8);
        }
        let xmsg_len = 1;
        
        if let Err(err) = self.writer.write_all(&x_msg.as_slice()).await{
            eprintln!("Write to partner failed:{}", err);
            std::process::exit(-1);
        }
        else{
            // println!("Write to partner {} bytes.", xmsg_len);
        } // send message to the partner

        match  self.reader.read_exact(&mut buf[0..xmsg_len]).await{
            Err(e) => {
                eprintln!("read from client error: {}", e);
                std::process::exit(-1);
            }
            Ok(0) => {
                println!("client closed.");
                std::process::exit(-1);
            }  
            Ok(n) => {
                self.received+=xmsg_len;
                assert_eq!(n, xmsg_len);
                // println!("Receive {} bytes from partner.", n);
            }        
        }

        self.rounds_occured+=1;

        let mut r = msg; //save the msg
        if buf[0] == 1{
            r = !r;
        }

        r
    }

    pub async fn exchange_bool_vec(&mut self, msg: Vec<bool>)->Vec<bool>{
        // let x_msg: Vec<u8> = msg.clone().iter().map(|x| if *x == true {1} else {0}).collect(); // convert the bool vec to u8 vec such that the message can be convoyed in the channel.
         fn bool_vec_to_u8_vec(bool_vec: &[bool]) -> Vec<u8> {
            let mut u8_vec = vec![];
            let mut accumulator = 0u8;
            let mut bit_index = 0;

            for &b in bool_vec {
                if b {
                    accumulator |= 1 << bit_index;
                }
                bit_index += 1;

                if bit_index >= 8 {
                    u8_vec.push(accumulator);
                    accumulator = 0;
                    bit_index = 0;
                }
            }

            // Push the remaining accumulator value if there are any "left-over" bits
            if bit_index > 0 {
                u8_vec.push(accumulator);
            }

            u8_vec
        }

        fn u8_vec_to_bool_vec(u8_vec: &[u8], original_length: usize) -> Vec<bool> {
            let mut bool_vec = Vec::with_capacity(original_length);
            for &byte in u8_vec {
                for bit_index in 0..8 {
                    if bool_vec.len() == original_length {
                        break;
                    }
                    let mask = 1 << bit_index;
                    bool_vec.push(byte & mask != 0);
                }
            }
            bool_vec
        }

        let x_msg = bool_vec_to_u8_vec(&msg);
        let xmsg_len = x_msg.len();
        let mut buf: Vec<u8> = vec![0; xmsg_len];

        // println!("current x_msg len is: {:?}", xmsg_len);

        const MAX_MSG_SIZE:usize = 256000; //250KB
        let slices:usize = (xmsg_len+MAX_MSG_SIZE-1)/MAX_MSG_SIZE;

        for i in 0..slices{
            let start_index:usize = i*MAX_MSG_SIZE;
            let end_index:usize = if i==slices-1{xmsg_len}else{(i+1)*MAX_MSG_SIZE };
            let expect_buff_size = end_index - start_index;

            let cur_slice: &[u8] = &x_msg[start_index..end_index];

            if let Err(err) = self.writer.write_all(&cur_slice).await{
                eprintln!("Write to partner failed:{}", err);
                std::process::exit(-1);
            }
            else{
            //println!("Write to partner {} bytes.", xmsg_len);
            } //send message to the partner

            match  self.reader.read_exact(&mut buf[start_index..end_index]).await{
                Err(e) => {
                    eprintln!("read from client error: {}", e);
                    std::process::exit(-1);
                }
                Ok(0) => {
                    println!("client closed.");
                    std::process::exit(-1);
                }  
                Ok(n) => {
                    self.received += expect_buff_size;
                    assert_eq!(n, expect_buff_size);
                    // println!("Receive {} bytes from partner.", n);
                }        
            }
        }

        self.rounds_occured+=1;

        let decoded_bool_vec = u8_vec_to_bool_vec(&buf, msg.len());

        decoded_bool_vec
    }

    pub async fn exchange_ring_vec(&mut self, msg: Vec<RingElm>) -> Vec<RingElm>{
        let mut x_msg: Vec<u8> = Vec::<u8>::new();
        for e in &msg{
            x_msg.append(&mut e.to_u32().unwrap().to_be_bytes().to_vec());
        }//convert u32 stream to u8 stream

        let xmsg_len = x_msg.len();
        let mut buf: Vec<u8> = vec![0; xmsg_len];

        if let Err(err) = self.writer.write_all(&x_msg.as_slice()).await{
            eprintln!("Write to partner failed:{}", err);
            std::process::exit(-1);
        }
        else{
            // println!("Write to partner {} bytes.", xmsg_len);
        } // send message to the partner

        match  self.reader.read_exact(&mut buf[0..xmsg_len]).await{
            Err(e) => {
                eprintln!("read from client error: {}", e);
                std::process::exit(-1);
            }
            Ok(0) => {
                println!("client closed.");
                std::process::exit(-1);
            }     
            Ok(n) => {
                self.received+=xmsg_len;
                assert_eq!(n, xmsg_len);
                // println!("Receive {} bytes from partner.", n);
            }        
        }
        self.rounds_occured+=1;

        let mut r: Vec<RingElm> = msg;
        for i in 0..xmsg_len/4{
            let mut ybuf: [u8; 4]= [0; 4];
            for j in 0..4{
                ybuf[j] = buf[i*4+j];
            }
            let e = RingElm::from(u32::from_be_bytes(ybuf));
            r[i].add(&e);
        }
        r
    }

    pub async fn exchange_byte_vec(&mut self, messages: &Vec<Vec<u8>>) -> Vec<Vec<u8>>{
        let mut return_vec: Vec<Vec<u8>> = Vec::new();

        for i in 0..messages.len() {
            let msg = &messages[i];
            let msg_len = msg.len();

            let mut buf: Vec<u8> = vec![0; msg_len];
            if let Err(err) = self.writer.write_all(&msg.as_slice()).await{
                eprintln!("Write to partner failed:{}", err);
                std::process::exit(-1);
            }
            else{
                // println!("Write to partner {} bytes.", xmsg_len);
            } // send message to the partner

            match  self.reader.read_exact(&mut buf[0..msg_len]).await{
                Err(e) => {
                    eprintln!("read from client error: {}", e);
                    std::process::exit(-1);
                }
                Ok(0) => {
                    println!("client closed.");
                    std::process::exit(-1);
                }     
                Ok(n) => {
                    self.received+=msg_len;
                    assert_eq!(n, msg_len);
                    // println!("Receive {} bytes from partner.", n);
                }        
            }
            return_vec.push(buf);   
        }
        self.rounds_occured +=1;
        return_vec
    }

    pub async fn exchange_u16_vec(&mut self, msg: Vec<u16>) -> Vec<u16>{
        let mut x_msg: Vec<u8> = Vec::<u8>::new();
        for e in &msg{
            x_msg.append(&mut e.to_be_bytes().to_vec());
        }//convert u32 stream to u8 stream

        let xmsg_len = x_msg.len();
        let mut buf: Vec<u8> = vec![0; xmsg_len];
        if let Err(err) = self.writer.write_all(&x_msg.as_slice()).await{
            eprintln!("Write to partner failed:{}", err);
            std::process::exit(-1);
        }
        else{
            // println!("Write to partner {} bytes.", xmsg_len);
        } // send message to the partner

        match  self.reader.read_exact(&mut buf[0..xmsg_len]).await{
            Err(e) => {
                eprintln!("read from client error: {}", e);
                std::process::exit(-1);
            }
            Ok(0) => {
                println!("client closed.");
                std::process::exit(-1);
            }     
            Ok(n) => {
                self.received+=xmsg_len;
                assert_eq!(n, xmsg_len);
                // println!("Receive {} bytes from partner.", n);
            }        
        }
        self.rounds_occured+=1;

        let mut r: Vec<u16> = msg;
        for i in 0..xmsg_len/2{
            let mut ybuf: [u8; 2]= [0; 2];
            for j in 0..2{
                ybuf[j] = buf[i*2+j];
            }
            let e = u16::from_be_bytes(ybuf);
            r[i] = r[i].wrapping_add(e);
        }
        r
    }

}