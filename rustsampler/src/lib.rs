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

extern crate libc;
extern crate lv2_raw;
extern crate num;


use lv2_raw::*;
use std::f32;
use std::f64;
use std::mem::*;
use libc::{c_char, c_void};
use std::ffi::*;
use std::f64::consts::PI;

/*
   The URI is the identifier for a plugin, and how the host associates this
   implementation in code with its description in data.  In this plugin it is
   only used once in the code, but defining the plugin URI at the top of the
   file is a good convention to follow.  If this URI does not match that used
   in the data files, the host will fail to load the plugin.
*/
static SAMPLER_URI: &'static [u8] = b"http://example.org/rustsampler\0";
static SAMPLER__SAMPLE: &'static [u8] = b"http://example.org/rustsampler#sample\0";
static SAMPLER__APPLY_SAMPLE: &'static [u8] = b"http://example.org/rustsampler#applySample\0";
static SAMPLER__FREE_SAMPLE: &'static [u8] = b"http://example.org/rustsampler#freeSample\0";


struct SamplerURIs {
    atom_float: LV2Urid, 
    atom_path: LV2Urid,
    atom_resource: LV2Urid,
    atom_sequence: LV2Urid,
    atom_urid: LV2Urid,
    atom_event_transfer: LV2Urid,
    eg_apply_sample: LV2Urid,
    eg_sample: LV2Urid,
    eg_freeSample: LV2Urid,
    midi_event: LV2Urid,
    param_gain: LV2Urid,
    patch_get: LV2Urid,
    patch_set: LV2Urid,
    patch_property: LV2Urid,
    patch_value: LV2Urid
}

impl SamplerURIs {

    fn new() -> SamplerURIs {
        SamplerURIs {
            atom_float: 0,
            atom_path: 0,
            atom_resource: 0,
            atom_sequence: 0,
            atom_urid: 0,
            atom_event_transfer: 0,
            eg_apply_sample: 0,
            eg_sample: 0,
            eg_freeSample: 0,
            midi_event: 0,
            param_gain: 0,
            patch_get: 0,
            patch_set: 0,
            patch_property: 0,
            patch_value: 0
        }
    }

    fn map_sampler_uris(map: *const LV2UridMap) -> SamplerURIs {

        let f = (*map).map;

        SamplerURIs {
            atom_float: f((*map).handle, LV2_ATOM__FLOAT.as_ptr() as *const c_char),
            atom_path: f((*map).handle, LV2_ATOM__PATH.as_ptr() as *const c_char),
            atom_resource: f((*map).handle, LV2_ATOM__RESOURCE.as_ptr() as *const c_char),
            atom_sequence: f((*map).handle, LV2_ATOM__SEQUENCE.as_ptr() as *const c_char),
            atom_urid: f((*map).handle, LV2_ATOM__URID.as_ptr() as *const c_char),
            atom_event_transfer: f((*map).handle, LV2_ATOM__EVENTTRANSFER.as_ptr() as *const c_char),
            eg_apply_sample: f((*map).handle, SAMPLER__APPLY_SAMPLE.as_ptr() as *const c_char),
            eg_sample: f((*map).handle, SAMPLER__APPLY_SAMPLE.as_ptr() as *const c_char),
            eg_apply_sample: f((*map).handle, SAMPLER__SAMPLE.as_ptr() as *const c_char),
            eg_freeSample: f((*map).handle, SAMPLER__FREE_SAMPLE.as_ptr() as *const c_char),
            midi_event: f((*map).handle, LV2_MIDI__MIDIEVENT.as_ptr() as *const c_char),
            param_gain: f((*map).handle, LV2_PARAMETERS__GAIN.as_ptr() as *const c_char),
            patch_get: f((*map).handle, LV2_PATCH__GET.as_ptr() as *const c_char),
            patch_set: f((*map).handle, LV2_PATCH__SET.as_ptr() as *const c_char),
            patch_property: f((*map).handle, LV2_PATCH__PROPERTY.as_ptr() as *const c_char),
            patch_value: f((*map).handle, LV2_PATCH__VALUE.as_ptr() as *const c_char),
        }
    }

}


/*
   In code, ports are referred to by index.  An enumeration of port indices
   should be defined for readability.
*/


enum PortIndex {
    SamplerControl  = 0,
    SamplerNotify = 1,
    SamplerOut = 2
}

impl PortIndex {

    fn from_u32(x: u32) -> Option<PortIndex> {
        match x {
            0 => Some(PortIndex::SamplerControl),
            1 => Some(PortIndex::SamplerNotify),
            2 => Some(PortIndex::SamplerOut),
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
struct Ports {
    control: *mut LV2AtomSequence,
    output: *mut f32
}


#[repr(C)]
struct Sampler {
    map: *const LV2UridMap,
    schedule: *const LV2WorkerSchedule,
    uris: SamplerURIs,

    ports: Ports,
    
    rate: f64,
    bpm: f64,
    speed: f64,

    elapsed_len: u32,

    state: State,

    wave: Vec<f32>,
    wave_offset: usize,

    attack_len: u32,
    decay_len: u32
}



impl Metro {
    fn new(m: *const LV2UridMap, 
           u: MetroURIs, 
           rate: f64, 
           bpm: f64,
           wav: Vec<f32>) -> Metro {
        Metro { 
            map: m,

            uris: u,

            ports: Ports {
                control: (0 as *mut LV2AtomSequence),
                output: (0 as *mut f32) },

            rate: rate,
            bpm: bpm,
            speed: 0.0,

            elapsed_len: 0,
            wave_offset: 0,

            state: State::StateOff,

            wave: wav,

            attack_len: (rate * 0.005) as u32,
            decay_len: (rate * 0.075) as u32
        }
    }

    pub fn play(&mut self, begin: u32, end: u32) -> () {
        let frames_per_beat = (60.0 / self.bpm * self.rate) as u32;

        let mut out = unsafe { std::slice::from_raw_parts_mut(self.ports.output.offset(begin as isize), 
                        (end - begin) as usize) };

        if self.speed == 0.0 {
            for it in &mut out[..] {
                *it = 0.0;
                return;
            }
        }

        for it in &mut out[..] {
            match self.state {
                State::StateAttack => {
                    *it = self.wave[self.wave_offset] * (self.elapsed_len as f32) 
                            / (self.attack_len as f32);
                    if self.elapsed_len >= self.attack_len {
                        self.state = State::StateDecay;
                    }
                },
                State::StateDecay => {
                    let d = (self.elapsed_len as f32 - self.attack_len as f32) 
                            / self.decay_len as f32;
                    *it = self.wave[self.wave_offset] * (1.0 - d);
                    if self.elapsed_len >= (self.attack_len + self.decay_len) {
                        self.state = State::StateOff;
                    }
                },
                State::StateOff => *it = 0.0,
            }

            self.wave_offset = (self.wave_offset + 1) % self.wave.len();

            self.elapsed_len += 1;
            if self.elapsed_len == frames_per_beat {
                self.state = State::StateAttack;
                self.elapsed_len = 0;
            }
        }
    }


    pub fn update_position(&mut self, obj: *mut LV2AtomObject) -> () {
        let uris = &self.uris;


        let mut beat: *mut LV2Atom = 0 as *mut LV2Atom;
        let mut bpm: *mut LV2Atom = 0 as *mut LV2Atom;
        let mut speed: *mut LV2Atom = 0 as *mut LV2Atom;

        let descr = [ObjectHelper{key: uris.time_bar_beat, atom: &mut beat},
            ObjectHelper{key: uris.time_beats_per_minute, atom: &mut bpm},
            ObjectHelper{key: uris.time_speed, atom: &mut speed}];

        unsafe { 
            
            lv2_atom_object_get(obj, &descr[..]);

            if !bpm.is_null() && (*bpm).mytype == uris.atom_float {
                self.bpm = (*(bpm as *const LV2AtomFloat)).body as f64;
            }
            if !speed.is_null() && (*speed).mytype == uris.atom_float {
                self.speed = (*(speed as *const LV2AtomFloat)).body as f64;
            }
            if !beat.is_null() && (*beat).mytype == uris.atom_float {
                let frames_per_beat = 60.0 / self.bpm * self.rate;
                let bar_beats = (*(beat as *const LV2AtomFloat)).body as f64;
                let beat_beats = bar_beats - bar_beats.floor();

                self.elapsed_len = (beat_beats * frames_per_beat) as u32;

                if self.elapsed_len < self.attack_len {
                    self.state = State::StateAttack;
                } else if self.elapsed_len < (self.attack_len + self.decay_len) {
                    self.state = State::StateDecay;
                } else {
                    self.state = State::StateOff;
                }

            }
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
    pub extern "C" fn activate(_handle: LV2Handle) {
        let mut metro = unsafe { &mut *(_handle as *mut Metro) };

        metro.elapsed_len = 0;
        metro.wave_offset = 0;
        metro.state = State::StateOff;
    }


    pub extern "C" fn deactivate(_handle: LV2Handle) {}


    pub extern "C" fn run(_handle: LV2Handle, sample_count: u32) {
        let metro = unsafe { &mut *(_handle as *mut Metro) };

        let inp = metro.ports.control;
        let mut last_t = 0;

        unsafe {
        
            let mut ev = lv2_atom_sequence_begin(&(*inp).body);

            while !lv2_atom_sequence_is_end(&(*inp).body, (*inp).atom.size, ev) {


                metro.play(last_t, (*ev).time_as_frames() as u32);

                if ((*ev).body.mytype == metro.uris.atom_object) ||
                    ((*ev).body.mytype == metro.uris.atom_blank) {

                    let addr: *mut LV2Atom = &mut ((*ev).body);
                    let obj = addr as *mut LV2AtomObject;

                    if (*obj).body.otype == metro.uris.time_position {
                        metro.update_position(obj);
                    }
                }
                last_t = (*ev).time_as_frames() as u32;

                ev = lv2_atom_sequence_next(ev);
            }
        }

        metro.play(last_t, sample_count);
    }            


    pub extern "C" fn connect_port(instance: LV2Handle,
        port: u32,
        data : *mut c_void)
    {
        let mut metro = unsafe { &mut *(instance as *mut Metro) };
        let p = PortIndex::from_u32(port);

        println!("connect_port: {}", port);

        match p {
            Some(PortIndex::MetroControl) => metro.ports.control = data as *mut LV2AtomSequence,
            Some(PortIndex::MetroOut) => metro.ports.output = data as *mut f32 ,
            None => println!("Not a valid port index: {}", port)
        }
    }



    pub extern "C" fn instantiate(_desc: *const LV2Descriptor,
        _rate: f64,
        _bundle_path: *const c_char,
        _features: *const *const LV2Feature) -> LV2Handle {

            let ptr: *mut Metro;

            unsafe {
                let mut map = 0 as *const LV2UridMap;
                let mut i = 0;
                let mut feature = *_features;
                let nul = 0 as *const LV2Feature;

                while feature != nul {
                    let f = CStr::from_ptr((*feature).uri).to_string_lossy().into_owned();
                    if f == LV2_URID__MAP {
                        map = (*feature).data as *const LV2UridMap;
                        break;
                    }

                    feature = *_features.offset(i);
                    i += 1;
                }

                if map.is_null() {
                    ptr = 0 as *mut Metro;
                } else {
                    let f = (*map).map;

                    let uris = MetroURIs {
                            atom_blank: f((*map).handle, LV2_ATOM__BLANK.as_ptr() as *const c_char),
                            atom_float: f((*map).handle, LV2_ATOM__FLOAT.as_ptr() as *const c_char),
                            atom_object: f((*map).handle, LV2_ATOM__OBJECT.as_ptr() as *const c_char),
                            atom_path: f((*map).handle, LV2_ATOM__PATH.as_ptr() as *const c_char),
                            atom_resource: f((*map).handle, LV2_ATOM__RESOURCE.as_ptr() as *const c_char),
                            atom_sequence: f((*map).handle, LV2_ATOM__SEQUENCE.as_ptr() as *const c_char),
                            time_position: f((*map).handle, LV2_TIME__POSITION.as_ptr() as *const c_char),
                            time_bar_beat: f((*map).handle, LV2_TIME__BARBEAT.as_ptr() as *const c_char),
                            time_beats_per_minute: f((*map).handle, LV2_TIME__BEATSPERMINUTE.as_ptr() as *const c_char),
                            time_speed: f((*map).handle, LV2_TIME__SPEED.as_ptr() as *const c_char)
                        };

                    let freq = 440.0 * 2.0;
                    let amp = 0.5;

                    let mut data = Vec::new();
                    data.set_len((_rate / freq) as usize);

                    let mut i = 0;
                    for it in &mut data {
                        *it = (((i as f64) * 2.0 * PI * freq / _rate).sin() * amp) as f32;
                        i += 1;
                    }

                    ptr = transmute(Box::new(Metro::new(map, 
                                uris, 
                                _rate, 
                                120.0,
                                data
                                )));
                }

            }

            return ptr as LV2Handle;
    }

    pub extern "C" fn cleanup(handle: LV2Handle) {
        unsafe { 
            let _drop: Box<Metro> = transmute(((handle as *mut Metro)));
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
            DESCRIPTOR.uri = MIDIGATE_URI.as_ptr() as *const c_char;
            return &DESCRIPTOR as *const LV2Descriptor },
        _ => return 0 as *const LV2Descriptor
    }
}

