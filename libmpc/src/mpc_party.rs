use crate::mpc_platform::*;
pub struct MPCParty<T>{
    // offlinedata: BitMaxOffline,
    pub offlinedata: T,
    
    pub m: usize, //The number of share numbers
    pub n: usize, //The length of a shared element

    pub netlayer: NetInterface
}

impl<T>  MPCParty<T>{
    pub fn new(data: T, netinterface:  NetInterface)->Self{
        MPCParty { offlinedata: data, m: 0, n: 0, netlayer: netinterface}
    }
    pub fn setup(&mut self, input_size: usize, input_bits: usize){
        self.m = input_size;
        self.n = input_bits;
    }
}