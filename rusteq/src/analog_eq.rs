

// ZynaddSubFx filters
use std::f64::consts::*;


#[derive(Clone, Copy, Debug)]
pub struct Coeff {
    c: [f64; 3],
    d: [f64; 3]
}

#[derive(Clone, Copy, Debug)]
pub struct FStage {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64
}



pub const MAX_FILTER_STAGES: u8 = 5;


#[derive(Clone, Copy, Debug)]
pub enum FilterType {
    LPF1,
    HPF1,
    LPF2,
    HPF2,
    BPF2,
    NOTCH2,
    PEAK2,
    LoShelf,
    HiShelf
}

impl FilterType {

    pub fn from_u32(x: u32) -> FilterType {
        match x {
            0 => FilterType::LPF1,
            1 => FilterType::HPF1,
            2 => FilterType::LPF2,
            3 => FilterType::HPF2,
            4 => FilterType::BPF2,
            5 => FilterType::NOTCH2,
            6 => FilterType::PEAK2,
            7 => FilterType::LoShelf,
            8 => FilterType::HiShelf,
            _ => FilterType::LPF1
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            FilterType::LPF1 => 0,
            FilterType::HPF1 => 1,
            FilterType::LPF2 => 2,
            FilterType::HPF2 => 3,
            FilterType::BPF2 => 4,
            FilterType::NOTCH2 => 5,
            FilterType::PEAK2 => 6,
            FilterType::LoShelf => 7,
            FilterType::HiShelf => 8
        }
    }
}



#[warn(non_snake_case)]
pub fn db_2_rap(db: f64) -> f64 {
    db.exp() * LN_10 / 20.0
}

#[warn(non_snake_case)]
pub fn rap_2_db(rap: f64) -> f64 {
    (20.0 * rap.ln()) / LN_10
}

#[derive(Debug)]
pub struct AnalogFilter {
    coeff: Coeff,
    old_coeff: Coeff,

    history: [FStage; (MAX_FILTER_STAGES + 1) as usize],
    old_history: [FStage; (MAX_FILTER_STAGES + 1) as usize],

    filter_type: FilterType,
    stages: u8,
    freq: f64,
    q: f64,
    gain: f64,

    order: u32,

    needs_interpolation: bool,
    first_time: bool,

    above_nq: bool,
    oldabove_nq: bool,

    samplerate: u32,
    outgain: f64,

    samplerate_f: f64,
    halfsamplerate_f: f64
}


impl AnalogFilter {

    pub fn new(ftype: &FilterType, 
        ffreq: f32,
        fq: f32,
        fstages: u8,
        srate: u32) -> AnalogFilter {
            
            let st = if fstages > MAX_FILTER_STAGES { MAX_FILTER_STAGES } else { fstages }; 
            
            let mut f = AnalogFilter {
                outgain: 1.0,
                samplerate: srate,
                samplerate_f: srate as f64,

                halfsamplerate_f: (srate as f64) / 2.0,

                coeff: Coeff { c: [0.0; 3], d: [0.0; 3] },
                old_coeff: Coeff { c: [0.0; 3], d: [0.0; 3] },

                history: [FStage {x1: 0.0, x2:  0.0, y1: 0.0, y2: 0.0 }; (MAX_FILTER_STAGES + 1) as usize],
                old_history: [FStage {x1: 0.0, x2:  0.0, y1: 0.0, y2: 0.0 }; (MAX_FILTER_STAGES + 1) as usize],

                filter_type: *ftype,
                stages: st,
                freq: ffreq as f64,
                q: fq as f64,
                gain: 1.0,

                order: 1,

                needs_interpolation: false,
                first_time: true,

                above_nq: false,
                oldabove_nq: false

            };

            f.setfreq(ffreq);
            f
        }


    pub fn setfreq(&mut self, freq: f32) -> () {
        let frequency = if freq < 0.1 { 0.1_f64 } else { freq as f64 };

        let mut rap = self.freq / frequency;
        if rap < 1.0 { rap = 1.0 / rap }

        self.oldabove_nq = self.above_nq;
        self.above_nq = frequency > (self.halfsamplerate_f - 500.0);

        let nyquistthresh = self.above_nq ^ self.oldabove_nq;

        if rap > 3.0 || nyquistthresh {
            self.old_coeff = self.coeff;
            
            self.old_history = self.history;
            if !self.first_time { self.needs_interpolation = true };
        }

        self.freq = frequency;
        self.computefiltercoefs();
        self.first_time = false;
    }

    pub fn computefiltercoefs(&mut self) -> () {
        println!("computefiltercoefs: {:?}", self.filter_type);

        let (coeff, order) = AnalogFilter::compute_coeff(self.filter_type, self.freq, self.q, self.stages,
            self.gain, self.samplerate_f);

        self.coeff = coeff;
        self.order = order;
    }


    pub fn compute_coeff(ftype: FilterType,
        cutoff: f64,
        fq: f64,
        stages: u8,
        gain: f64,
        fs: f64) -> (Coeff, u32) {

        println!("computefiltercoefs: {:?}", ftype);

        let mut coeff = Coeff { c: [0.0; 3], d: [0.0; 3] };
        let mut zerocoefs = false;
        let mut q = fq;
        let order;

        let samplerate_f = fs;
        let halfsamplerate_f = fs / 2.0;

        let mut freq = cutoff;

        if freq > (halfsamplerate_f - 500.0) {
            freq = halfsamplerate_f - 500.0;
            zerocoefs = true;
        }

        if freq < 0.1 {
            freq = 0.1;
        }

        if q < 0.0 {

            q = 0.0;
        }

        let mut tmpq;
        let tmpgain;

        if stages == 0 {
            tmpq = q;
            tmpgain = gain;
        } else {
            let fact = 1.0 / ((stages + 1) as f64);
            tmpq = if q > 1.0 { 
                    q.powf(fact)
                } else {
                    q
                };
            tmpgain = gain.powf(fact);
        }

        let omega = 2.0 * PI * freq / samplerate_f;
        let sn = omega.sin();
        let cs = omega.cos();

        let alpha;
        let beta;
        let tmp;
        let tgp1;
        let tgm1;

        match ftype {
            FilterType::LPF1 => {
                if !zerocoefs {
                    tmp = (-2.0 * PI * freq / samplerate_f).exp();
                } else {
                    tmp = 0.0;
                }
                coeff.c[0] = 1.0 - tmp;
                coeff.c[1] = 0.0;
                coeff.c[2] = 0.0;
                coeff.d[1] = tmp;
                coeff.d[2] = 0.0;
                order = 1;

                println!("LPF1 coeffs: {:?}", coeff);
            }
            FilterType::HPF1 => {
                if !zerocoefs {
                    tmp = (-2.0 * PI * freq / samplerate_f).exp();
                } else {
                    tmp = 0.0;
                }
                coeff.c[0] = (1.0 + tmp) / 2.0;
                coeff.c[1] = -(1.0 + tmp) / 2.0;
                coeff.c[2] = 0.0;
                coeff.d[1] = tmp;
                coeff.d[2] = 0.0;
                order = 1;

                println!("HPF1 coeffs: {:?}", coeff);
                
            }
            FilterType::LPF2 => {
                if !zerocoefs {
                    alpha = sn / (2.0 * tmpq);
                    tmp = 1.0 + alpha;
                    coeff.c[1] = (1.0 - cs) / tmp;
                    coeff.c[0] = coeff.c[1] / 2.0;
                    coeff.c[2] = coeff.c[1] / 2.0;
                    coeff.d[1] = -2.0 * cs / tmp * -1.0;
                    coeff.d[2] = (1.0 - alpha) / tmp * -1.0;
                } else {
                    coeff.c[0] = 1.0;
                    // other coeffs are zero initalised                        
                }
                order = 2;

                println!("LPF2 coeffs: {:?}", coeff);
            }
            FilterType::HPF2 => {
                if !zerocoefs {
                    alpha = sn / (2.0 * tmpq);
                    tmp = 1.0 + alpha;
                    coeff.c[0] = (1.0 + cs) / 2.0 / tmp;
                    coeff.c[1] = -(1.0 + cs) / tmp;
                    coeff.c[2] = (1.0 + cs) / 2.0 / tmp;
                    coeff.d[1] = -2.0 * cs / tmp * -1.0;
                    coeff.d[2] = (1.0 - alpha) / tmp * -1.0;
                } 
                order = 2;
            }
            FilterType::BPF2 => {
                if !zerocoefs {
                    alpha = sn / (2.0 * tmpq);
                    tmp = 1.0 + alpha;
                    coeff.c[0] = alpha / tmp * (tmpq + 1.0).sqrt();
                    coeff.c[1] = 0.0;
                    coeff.c[2] = -alpha / tmp * (tmpq + 1.0).sqrt();
                    coeff.d[1] = -2.0 * cs / tmp * -1.0;
                    coeff.d[2] = (1.0 - alpha) / tmp * -1.0;
                }
                order = 2;
            }
            FilterType::NOTCH2 => {
                if !zerocoefs {
                    alpha = sn / (2.0 * tmpq.sqrt());
                    tmp = 1.0 + alpha; 
                    coeff.c[0] = 1.0 / tmp;
                    coeff.c[1] = -2.0 * cs / tmp;
                    coeff.c[2] = 1.0 / tmp;
                    coeff.d[1] = -2.0 * cs / tmp * -1.0;
                    coeff.d[2] = (1.0 - alpha) / tmp * -1.0
                } else {
                    coeff.c[0] = 1.0
                }
                order = 2;
            }
            FilterType::PEAK2 => {
                if !zerocoefs {
                    tmpq *= 3.0;
                    alpha = sn / (2.0 * tmpq);
                    tmp = 1.0 + alpha / tmpgain;
                    coeff.c[0] = (1.0 + alpha * tmpgain) / tmp;
                    coeff.c[1] = (-2.0 * cs) / tmp;
                    coeff.c[2] = (1.0 - alpha * tmpgain) / tmp;
                    coeff.d[1] = -2.0 * cs / tmp * -1.0;
                    coeff.d[2] = (1.0 - alpha / tmpgain) / tmp * -1.0;
                } else {
                    coeff.c[0] = 1.0;
                }
                order = 2;
            }
            FilterType::LoShelf => {
                if !zerocoefs {
                    tmpq = tmpq.sqrt();
                    beta = tmpgain.sqrt() / tmpq;
                    tgp1 = tmpgain + 1.0;
                    tgm1 = tmpgain - 1.0;
                    tmp = tgp1 + tgm1 * cs + beta * sn;

                    coeff.c[0] = tmpgain * (tgp1 - tgm1 * cs + beta * sn) / tmp;
                    coeff.c[1] = 2.0 * tmpgain * (tgm1 - tgp1 * cs) / tmp;
                    coeff.c[2] = tmpgain * (tgp1 - tgm1 * cs - beta * sn) / tmp;
                    coeff.d[1] = -2.0 * (tgm1 + tgp1 * cs) / tmp * -1.0;
                    coeff.d[2] = (tgp1 + tgm1 * cs - beta * sn) / tmp * -1.0;
                } else {
                    coeff.c[0] = tmpgain;
                }
                order = 2;
            }
            FilterType::HiShelf => {
                if !zerocoefs {
                    tmpq = tmpq.sqrt();
                    beta = tmpgain.sqrt() / tmpq;
                    tgp1 = tmpgain + 1.0;
                    tgm1 = tmpgain - 1.0;
                    tmp = tgp1 - tgm1 * cs + beta * sn;

                    coeff.c[0] = tmpgain * (tgp1 + tgm1 * cs + beta * sn) / tmp;
                    coeff.c[1] = -2.0 * tmpgain * (tgm1 + tgp1 * cs) / tmp;
                    coeff.c[2] = tmpgain * (tgp1 + tgm1 * cs - beta * sn) / tmp;
                    coeff.d[1] = 2.0 * (tgm1 - tgp1 * cs) / tmp * -1.0;
                    coeff.d[2] = (tgp1 - tgm1 * cs - beta * sn) / tmp * -1.0;
                } else {
                    coeff.c[0] = 1.0;
                }
                order = 2;
            }
        }
        
        (coeff, order)
    }

    pub fn set_q(&mut self, q: f32) -> () {
        self.q = q as f64;
        self.computefiltercoefs();
    }

    pub fn set_type(&mut self, ftype: &FilterType) -> () {
        self.filter_type = *ftype;
        self.computefiltercoefs();
    }

    pub fn set_gain(&mut self, gain: f32) -> () {
        self.gain = db_2_rap(gain as f64);
        self.computefiltercoefs();
    }

    pub fn set_stages(&mut self, stages: u8) -> () {
        let _stages = if stages >= MAX_FILTER_STAGES { MAX_FILTER_STAGES - 1 } else { stages };
        if self.stages != _stages {
            self.stages = _stages;
            self.cleanup();
            self.computefiltercoefs();
        }
    }    

    pub fn cleanup(&mut self) -> () {

        self.history = [FStage {x1: 0.0, x2:  0.0, y1: 0.0, y2: 0.0 }; (MAX_FILTER_STAGES + 1) as usize];
        self.old_history = [FStage {x1: 0.0, x2:  0.0, y1: 0.0, y2: 0.0 }; (MAX_FILTER_STAGES + 1) as usize];
        self.needs_interpolation = false;
    }

    pub fn biquad_filter_a(coeff: &[f64; 5], src: f64, work: &mut [f64; 4]) -> f64 {
        work[3] = src * coeff[0]
            + work[0] * coeff[1]
            + work[1] * coeff[2]
            + work[2] * coeff[3]
            + work[3] * coeff[4];
        work[1] = src;
        work[3]
    }

    pub fn biquad_filter_b(coeff: &[f64; 5], src: f64, work: &mut [f64; 4]) -> f64 {
        work[2] = src * coeff[0]
            + work[1] * coeff[1]
            + work[0] * coeff[2]
            + work[3] * coeff[3]
            + work[2] * coeff[4];
        work[0] = src;
        work[2]
    }

    pub fn singlefilterout(input: &[f32], output: &mut [f32], 
                        hist: &mut FStage, coeff: &Coeff, order: &u32) -> () {
        
        assert!((input.len() % 8) == 0);

        if *order == 1 {
            for i in 0..input.len() {
                let ii = input[i] as f64;
                let y0 = ii * coeff.c[0] + hist.x1 * coeff.c[1] + hist.y1 * coeff.d[1];
                hist.y1 = y0;
                hist.x1 = ii;
                output[i] = y0 as f32;
            }
        } else if *order == 2 {
            let coeff = [coeff.c[0], coeff.c[1], coeff.c[2], coeff.d[1], coeff.d[2]];
            let mut work = [hist.x1, hist.x2, hist.y1, hist.y2];

            let mut i = 0;
            while i < input.len() {
                output[i + 0] = AnalogFilter::biquad_filter_a(&coeff, input[i + 0] as f64, &mut work) as f32;
                output[i + 1] = AnalogFilter::biquad_filter_b(&coeff, input[i + 1] as f64, &mut work) as f32;
                output[i + 2] = AnalogFilter::biquad_filter_a(&coeff, input[i + 2] as f64, &mut work) as f32;
                output[i + 3] = AnalogFilter::biquad_filter_b(&coeff, input[i + 3] as f64, &mut work) as f32;
                output[i + 4] = AnalogFilter::biquad_filter_a(&coeff, input[i + 4] as f64, &mut work) as f32;
                output[i + 5] = AnalogFilter::biquad_filter_b(&coeff, input[i + 5] as f64, &mut work) as f32;
                output[i + 6] = AnalogFilter::biquad_filter_a(&coeff, input[i + 6] as f64, &mut work) as f32;
                output[i + 7] = AnalogFilter::biquad_filter_b(&coeff, input[i + 7] as f64, &mut work) as f32;
                i += 8;
            }
            hist.x1 = work[0];
            hist.x2 = work[1];
            hist.y1 = work[2];
            hist.y2 = work[3];
        } 
    }


    pub fn filterout(&mut self, input: &[f32], output: &mut [f32]) -> () {
        for i in 0..(self.stages + 1) {
            AnalogFilter::singlefilterout(input, output, &mut self.history[i as usize], 
                &self.coeff, &self.order);
        }

        if self.needs_interpolation {
            let mut ismp = vec![0.0; output.len()];

            for i in 0..(self.stages + 1) {
                AnalogFilter::singlefilterout(input, &mut ismp[..], &mut self.old_history[i as usize],
                    &self.old_coeff, &self.order);
            }

            let len = ismp.len();
            let len_f = len as f64;
            for i in 0..len {
                let x = i as f64 / len_f;
                let isi = ismp[i] as f64;
                let ii = input[i] as f64;
                let val = isi * (1.0 - x) + ii * x;
                output[i] = val as f32;
                
            }

            self.needs_interpolation = false;
        }
    }

    pub fn h(&mut self, freq: f64) -> f64 {
        let fr = freq / self.samplerate_f * PI * 2.0;
        let mut x = self.coeff.c[0];
        let mut y = 0.0;

        for n in 1..3 {
            let t = n as f64 * fr;
            x += t.cos() * self.coeff.c[n];
            y -= t.sin() * self.coeff.c[n];
        }

        let mut h = x * x + y * y;
        x = 1.0;
        y = 0.0;

        for n in 1..3 {
            let t = n as f64 * fr;
            x -= t.cos() * self.coeff.d[n];
            y += t.sin() * self.coeff.d[n];
        }
        h = h/ (x * x + y * y);
        h.powf((self.stages as f64 + 1.0 ) / 2.0)
    }


    pub fn set_values(&mut self, ftype: f32, freq: f32, q: f32, stages: f32, gain: f64) -> () {
        
        println!("set_values: {:?}", self);

        let typ = FilterType::from_u32(ftype as u32);

    

        self.q = q as f64;
        self.filter_type = typ;
        self.gain = db_2_rap(gain as f64);

        let _stages = if stages as u8 >= MAX_FILTER_STAGES { MAX_FILTER_STAGES - 1 } else { stages as u8 };
        if self.stages != _stages {
            self.stages = _stages;
            self.cleanup();
        }

        let frequency = if freq < 0.1 { 0.1_f64 } else { freq as f64 };

        let mut rap = self.freq / frequency;
        if rap < 1.0 { rap = 1.0 / rap }

        self.oldabove_nq = self.above_nq;
        self.above_nq = frequency > (self.halfsamplerate_f - 500.0);

        let nyquistthresh = self.above_nq ^ self.oldabove_nq;

        if rap > 3.0 || nyquistthresh {
            self.old_coeff = self.coeff;
            
            self.old_history = self.history;
            if !self.first_time { self.needs_interpolation = true };
        }

        self.freq = frequency;
        self.first_time = false;
        
        println!("Before compute: {:?}", self);

        self.computefiltercoefs();

        println!("After compute: {:?}", self);

    }

}