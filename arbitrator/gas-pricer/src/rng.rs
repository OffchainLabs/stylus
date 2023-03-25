// see https://www.pcg-random.org/pdf/hmc-cs-2014-0905.pdf for details
const MUL_CONST: u64 = 6364136223846793005;

pub struct Lcg64XshRR32 {
    pub state: u64,
    pub inc: u64
}

impl Lcg64XshRR32 {
    pub fn new(state: u64, inc: u64) -> Self {
        let initial_state = state.wrapping_add(inc);
        let mut gen = Lcg64XshRR32 { state:initial_state, inc};
        gen.advance();
        gen
    }

    pub fn advance(&mut self) {
        // this is just an LCG
        self.state = self.state.wrapping_mul(MUL_CONST).wrapping_add(self.inc);
    }

    pub fn output(&self) -> u32 {
        // take 5 highest bits
        let rot_amount = (self.state >> 59) as u32; 
        // next 32 bits will become output (27 bits dropped); we xorshift by 18 
        // to improve output quality
        let xorshifted = (((self.state >> 18) ^ self.state) >> 27) as u32; 
        return xorshifted.rotate_right(rot_amount);
    }
}

impl Default for Lcg64XshRR32 {
    fn default() -> Self {
        Self::new(0xcafef00dd15ea5e5, (0xa02bdbf7bb3c0a7 << 1) | 1)
    }
}