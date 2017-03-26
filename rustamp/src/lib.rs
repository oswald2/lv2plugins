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
mod delay;

extern crate libc;
extern crate lv2_raw;
extern crate num;


use lv2_raw::*;
use std::f32;
use std::f64;
use std::mem::*;
use libc::{c_char, c_void};

use delay::*;

/*
   The URI is the identifier for a plugin, and how the host associates this
   implementation in code with its description in data.  In this plugin it is
   only used once in the code, but defining the plugin URI at the top of the
   file is a good convention to follow.  If this URI does not match that used
   in the data files, the host will fail to load the plugin.
*/
static AMP_URI: &'static [u8] = b"http://example.org/rustamp\0";

/*
   In code, ports are referred to by index.  An enumeration of port indices
   should be defined for readability.
*/
enum PortIndex {
    AmpGain   = 0,
    AmpInput  = 1,
    AmpOutput = 2,
    AmpDelay  = 3,
    AmpFeedback = 4,
    AmpMaster = 5
}

impl PortIndex {

    fn from_u32(x: u32) -> Option<PortIndex> {
        match x {
            0 => Some(PortIndex::AmpGain),
            1 => Some(PortIndex::AmpInput),
            2 => Some(PortIndex::AmpOutput),
            3 => Some(PortIndex::AmpDelay),
            4 => Some(PortIndex::AmpFeedback),
            5 => Some(PortIndex::AmpMaster),
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
struct Amp{
    // Port buffers
    gain: *const f32,
    input: *const f32,
    output: *mut f32,
    delay_time: *const f32,
    delay_feedback: *const f32,
    delay_master: *const f32,
    sample_rate: u32,
    delay: Delay
}

impl Amp {
    fn new(size: usize, rate: u32) -> Amp {
        Amp { gain: (0 as *const f32), 
            input: (0 as *const f32),   
            output: (0 as *mut f32),
            delay_time: (0 as *const f32),
            delay_feedback: (0 as *const f32),
            delay_master: (0 as *const f32),
            sample_rate: rate,
            delay: Delay::new(size)
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


const PI_2: f64 = std::f64::consts::PI / 2.0;
const MAX_DELAY_TIME: u32 = 2000;

fn distortion(gain: f64, inp: f64) -> f64
{
    let x = inp * gain;
     x.atan() / PI_2
}

impl Descriptor {
    pub extern "C" fn activate(_handle: LV2Handle) {}
    pub extern "C" fn deactivate(_handle: LV2Handle) {}


    pub extern "C" fn run(handle: LV2Handle, n_samples: u32) {
        let _amp = handle as *mut Amp;

        let mut amp = unsafe { &mut *_amp };

        let n = n_samples as usize;

        let gain = unsafe { *amp.gain };
        let input = unsafe { std::slice::from_raw_parts(amp.input, n) };
        let output = unsafe { std::slice::from_raw_parts_mut(amp.output, n) };
        let delay_time = unsafe { *amp.delay_time }; 
        let delay_feedback = unsafe { *amp.delay_feedback };
        let delay_master = unsafe { *amp.delay_master };
        let ref mut delay = amp.delay;

        let coef = db_co(gain);

        // set new delay time if changed
        let delay_size = delay::sec_to_n_samples(delay_time, amp.sample_rate as u32);
        delay.set_length(delay_size);
        delay.set_vals(delay_feedback, delay_master);

        let mut idx = 0;
        for pos in input {

            let val = distortion(coef, *pos as f64);
            let valout = delay.feedbackdelay(val);

            output[idx] = valout as f32;
            idx = idx + 1;
        }
    }

    pub extern "C" fn connect_port(instance: LV2Handle,
        port: u32,
        data : *mut c_void)
    {
        let mut amp = unsafe { &mut *(instance as *mut Amp) };
        let p = PortIndex::from_u32(port);

        match p {
            Some(PortIndex::AmpGain) => amp.gain = data as *const f32,
            Some(PortIndex::AmpInput) => amp.input = data as *const f32 ,
            Some(PortIndex::AmpOutput) => amp.output = data as *mut f32 ,
            Some(PortIndex::AmpDelay) => amp.delay_time = data as *const f32 ,
            Some(PortIndex::AmpFeedback) => amp.delay_feedback = data as *const f32,
            Some(PortIndex::AmpMaster) => amp.delay_master = data as *const f32,
            None => println!("Not a valid port index: {}", port)
        }
    }

    pub extern "C" fn instantiate(_desc: *const LV2Descriptor,
        _rate: f64,
        _bundle_path: *const c_char,
        _features: *const *const LV2Feature) -> LV2Handle {

            let rate = _rate as u32;
            let size = delay::msec_to_n_samples(MAX_DELAY_TIME, rate);

            let ptr: *mut Amp =  unsafe { transmute(Box::new(Amp::new(size, rate))) };

            return ptr as LV2Handle;
    }

    pub extern "C" fn cleanup(handle: LV2Handle) {
        unsafe { 
            let _drop: Box<Amp> = transmute(((handle as *mut Amp)));
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

