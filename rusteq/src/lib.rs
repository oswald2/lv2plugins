#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}


/*
  Copyright 2006-2016 David Robillard <d@drobilla.net>
  Copyright 2006 Steve Harris <steve@plugin.org.uk>

  Permission to use, copy, modify, and/or distribute this software for any
  purpose with or without fee is hereby granted, provided that the above
  copyright notice and this permission notice appear in all copies.

  THIS SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
  WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
  MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
  ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
  WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
  ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
  OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
*/

/*
   LV2 headers are based on the URI of the specification they come from, so a
   consistent convention can be used even for unofficial extensions.  The URI
   of the core LV2 specification is <http://lv2plug.in/ns/lv2core>, by
   replacing `http:/` with `lv2` any header in the specification bundle can be
   included, in this case `lv2.h`.
*/
mod analog_eq;

extern crate libc;
extern crate lv2_raw;
extern crate num;


use lv2_raw::*;
use std::f32;
use std::f64;
use std::mem::*;
use libc::{c_char, c_void};

use analog_eq::*;

/*
   The URI is the identifier for a plugin, and how the host associates this
   implementation in code with its description in data.  In this plugin it is
   only used once in the code, but defining the plugin URI at the top of the
   file is a good convention to follow.  If this URI does not match that used
   in the data files, the host will fail to load the plugin.
*/
static AMP_URI: &'static [u8] = b"http://example.org/rusteq\0";

/*
   In code, ports are referred to by index.  An enumeration of port indices
   should be defined for readability.
*/
enum PortIndex {
    EQInputL  = 0,
    EQInputR  = 1,
    EQOutputL = 2,
    EQOutputR = 3,
    EQType = 4,
    EQFreq = 5,
    EQQ = 6,
    EQStages = 7,
    EQGain = 8
}

impl PortIndex {

    fn from_u32(x: u32) -> Option<PortIndex> {
        match x {
            0 => Some(PortIndex::EQInputL),
            1 => Some(PortIndex::EQInputR),
            2 => Some(PortIndex::EQOutputL),
            3 => Some(PortIndex::EQOutputR),
            4 => Some(PortIndex::EQType),
            5 => Some(PortIndex::EQFreq),
            6 => Some(PortIndex::EQQ),
            7 => Some(PortIndex::EQStages),
            8 => Some(PortIndex::EQGain),
            _ => None
        }
    }
}

/*
   Every plugin defines a private structure for the plugin instance.  All data
   associated with a plugin instance is stored here, and is available to
   every instance method.  In this simple plugin, only port buffers need to be
   stored, since there is no additional instance data.
*/
#[repr(C)]
struct EQ{
    // Port buffers
    input_l: *const f32,
    input_r: *const f32,
    output_l: *mut f32,
    output_r: *mut f32,
    ftype: *const f32,
    freq: *const f32,
    q: *const f32,
    stages: *const f32,
    gain: *const f32,
    ftype_c: f32,
    freq_c: f32,
    q_c: f32,
    stages_c: f32,
    gain_c: f32,
    filter_l: AnalogFilter,
    filter_r: AnalogFilter
}

impl EQ {
    fn new(ftype: &FilterType, 
        ffreq: f32,
        fq: f32,
        fstages: u8,
        srate: u32) -> EQ {
        EQ { 
            input_l: (0 as *const f32),   
            input_r: (0 as *const f32),   
            output_l: (0 as *mut f32),
            output_r: (0 as *mut f32),
            ftype: (0 as *const f32),
            freq: (0 as *const f32),
            q: (0 as *const f32),
            stages: (0 as *const f32),
            gain: (0 as *const f32),
            ftype_c: ftype.to_u32() as f32,
            freq_c: ffreq,
            q_c: fq,
            stages_c: fstages as f32,
            gain_c: 1.0,
            filter_l: AnalogFilter::new(ftype, ffreq, fq, fstages, srate),
            filter_r: AnalogFilter::new(ftype, ffreq, fq, fstages, srate)
        }
    }
}


/* Define a macro for converting a gain in dB to a coefficient. */
//#define DB_CO(g) ((g) > -90.0f ? powf(10.0f, (g) * 0.05f) : 0.0f)

fn db_co(g: f32) -> f64 {
    if g > -90.0_f32 {
        let base = 10.0_f64;
        base.powf((g as f64) * 0.05_f64)
    }
    else {
        0.0_f64
    }
}

struct Descriptor(LV2Descriptor);


impl Descriptor {
    pub extern "C" fn activate(_handle: LV2Handle) {}
    pub extern "C" fn deactivate(_handle: LV2Handle) {}


    pub extern "C" fn run(handle: LV2Handle, n_seqlen: u32) {
        let _eq = handle as *mut EQ;

        let mut eq = unsafe { &mut *_eq };

        let n = n_seqlen as usize;

        let input_l = unsafe { std::slice::from_raw_parts(eq.input_l, n) };
        let input_r = unsafe { std::slice::from_raw_parts(eq.input_r, n) };
        let output_l = unsafe { std::slice::from_raw_parts_mut(eq.output_l, n) };
        let output_r = unsafe { std::slice::from_raw_parts_mut(eq.output_r, n) };
        let ftype = unsafe { *eq.ftype }; 
        let freq = unsafe { *eq.freq };
        let q = unsafe { *eq.q };
        let stages = unsafe { *eq.stages };
        let gain = unsafe { *eq.gain };
        let ref mut filter_l = eq.filter_l;
        let ref mut filter_r = eq.filter_r;

        if ftype != eq.ftype_c || freq != eq.freq_c || q != eq.q_c 
            || stages != eq.stages_c || gain != eq.gain_c {

            eq.ftype_c = ftype;
            eq.freq_c = freq;
            eq.q_c = q;
            eq.stages_c = stages;
            eq.gain_c = gain;

            filter_l.set_values(ftype, freq, q, stages, db_co(gain));
            filter_r.set_values(ftype, freq, q, stages, db_co(gain));
        }

        filter_l.filterout(input_l, output_l);
        filter_r.filterout(input_r, output_r);
    }

    pub extern "C" fn connect_port(instance: LV2Handle,
        port: u32,
        data : *mut c_void)
    {
        let mut eq = unsafe { &mut *(instance as *mut EQ) };
        let p = PortIndex::from_u32(port);

        println!("connect_port: {}", port);

        match p {
            Some(PortIndex::EQInputL) => eq.input_l = data as *const f32,
            Some(PortIndex::EQInputR) => eq.input_r = data as *const f32 ,
            Some(PortIndex::EQOutputL) => eq.output_l = data as *mut f32 ,
            Some(PortIndex::EQOutputR) => eq.output_r = data as *mut f32 ,
            Some(PortIndex::EQType) => eq.ftype = data as *const f32,
            Some(PortIndex::EQFreq) => eq.freq = data as *const f32,
            Some(PortIndex::EQQ) => eq.q = data as *const f32,
            Some(PortIndex::EQStages) => eq.stages = data as *const f32,
            Some(PortIndex::EQGain) => eq.gain = data as *const f32,
            None => println!("Not a valid port index: {}", port)
        }
    }


    pub extern "C" fn instantiate(_desc: *const LV2Descriptor,
        _rate: f64,
        _bundle_path: *const c_char,
        _features: *const *const LV2Feature) -> LV2Handle {

            let rate = _rate as u32;

            let ptr: *mut EQ =  unsafe { transmute(Box::new(EQ::new(
                &FilterType::LPF1,
                1000.0,
                1.0,
                1,
                rate))) };

            return ptr as LV2Handle;
    }

    pub extern "C" fn cleanup(handle: LV2Handle) {
        unsafe { 
            let _drop: Box<EQ> = transmute(((handle as *mut EQ)));
        }
    }

    pub extern "C" fn extension_data(_uri: *const u8) -> *const c_void {
        return 0 as *const c_void
    }
}

static mut DESCRIPTOR: LV2Descriptor = LV2Descriptor {
    uri: 0 as *const c_char,
    instantiate: Descriptor::instantiate,
    connect_port: Descriptor::connect_port,
    activate: Some(Descriptor::activate),
    run: Descriptor::run,
    deactivate: Some(Descriptor::deactivate),
    cleanup: Descriptor::cleanup,
    extension_data: Descriptor::extension_data
};

/*
   The `lv2_descriptor()` function is the entry point to the plugin library.  The
   host will load the library and call this function repeatedly with increasing
   indices to find all the plugins defined in the library.  The index is not an
   indentifier, the URI of the returned descriptor is used to determine the
   identify of the plugin.

   This method is in the ``discovery'' threading class, so no other functions
   or methods in this plugin library will be called concurrently with it.
*/

#[no_mangle]
pub extern "C" fn lv2_descriptor(index: u32) -> *const LV2Descriptor {
    match index {
        0 => unsafe {
            DESCRIPTOR.uri = AMP_URI.as_ptr() as *const c_char;
            return &DESCRIPTOR as *const LV2Descriptor },
        _ => return 0 as *const LV2Descriptor
    }
}

