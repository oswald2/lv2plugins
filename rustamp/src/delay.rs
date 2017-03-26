

pub struct Delay {
    buffer : Vec<f64>,
    index : usize,
    length : usize,
    feedback : f64,
    outlevel : f64
}

pub fn msec_to_n_samples(time: u32, sample_rate: u32) -> usize {
    let s = time / 1000 * sample_rate;
    s as usize
}

pub fn sec_to_n_samples(time: f32, sample_rate: u32) -> usize {
    let s = time * sample_rate as f32;
    s as usize
}


impl Delay {

    pub fn new(size : usize) -> Delay {
        Delay {buffer : vec![0.0; size], 
            index : 0, 
            length : 0,
            feedback : 0.5,
            outlevel : 1.0}
    }

    pub fn set_length(&mut self, new_length : usize) -> () {
        if new_length != self.length {
            let len = if new_length > self.buffer.len() { 
                            self.buffer.len() 
                        }
                        else {
                            new_length
                        };
            self.length = len;
        }
    }

    pub fn set_vals(&mut self, new_feedback: f32, new_master: f32) -> () {
        let feed = if new_feedback > 1.0 { 1.0 } else if new_feedback < -1.0 { -1.0 } else { new_feedback };
        self.feedback = feed as f64;

        let master = if new_master < 0.0 { 0.0 } else { new_master };
        self.outlevel = master as f64;
    }

    fn incr(&mut self) -> () {
        self.index = self.index + 1;
        if self.index >= self.buffer.len() || self.index >= self.length {
            self.index = 0;
        }
    }

    pub fn delayline(&mut self, x : f64) -> f64 {
        let y = self.buffer[self.index];
        self.buffer[self.index] = x;
        self.incr();
        y * self.outlevel
    }

    pub fn feedbackdelay(&mut self, x: f64) -> f64 {
        let d = self.buffer[self.index];
        let y = x - self.feedback * d;

        self.buffer[self.index] = y;
        self.incr();
        y * self.outlevel
    }

}