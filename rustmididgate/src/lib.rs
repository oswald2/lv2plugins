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


/*
   The URI is the identifier for a plugin, and how the host associates this
   implementation in code with its description in data.  In this plugin it is
   only used once in the code, but defining the plugin URI at the top of the
   file is a good convention to follow.  If this URI does not match that used
   in the data files, the host will fail to load the plugin.
*/
static MIDIGATE_URI: &'static [u8] = b"http://example.org/rustmidigate\0";

/*
   In code, ports are referred to by index.  An enumeration of port indices
   should be defined for readability.
*/
enum PortIndex {
    MGControl  = 0,
    MGIn  = 1,
    MGOut = 2
}

impl PortIndex {

    fn from_u32(x: u32) -> Option<PortIndex> {
        match x {
            0 => Some(PortIndex::MGControl),
            1 => Some(PortIndex::MGIn),
            2 => Some(PortIndex::MGOut),
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
struct MidiGate {
    // Port buffers
    control: *mut LV2_Atom_Sequence,
    input: *const f32,
    output: *mut f32,

    map: *const LV2UridMap,

    midi_event: LV2Urid,

    n_active_notes: u32,
    program: u32,
}

impl MidiGate {
    fn new(m: *const LV2UridMap, event: &LV2Urid) -> MidiGate {
        MidiGate { 
            control: (0 as *mut LV2_Atom_Sequence),
            input: (0 as *const f32),   
            output: (0 as *mut f32),

            map: m,

            midi_event: *event,
            n_active_notes: 0,
            program: 0
        }
    }

    fn write_output(&mut self, offset: isize, len: usize) -> () {
        let active = if self.program == 0 { self.n_active_notes > 0 } 
                        else { self.n_active_notes == 0 };

        let input = unsafe { std::slice::from_raw_parts(self.input.offset(offset), len) };
        let output = unsafe { std::slice::from_raw_parts_mut(self.output.offset(offset), len) };

        if active {
            output.copy_from_slice(input);
        } else {
            for out in output {
                *out = 0.0;
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
        let mut gate = unsafe { &mut *(_handle as *mut MidiGate) };

        gate.n_active_notes = 0;
        gate.program = 0;
    }


    pub extern "C" fn deactivate(_handle: LV2Handle) {}


    pub extern "C" fn run(_handle: LV2Handle, sample_count: u32) {
        let gate = unsafe { &mut *(_handle as *mut MidiGate) };
        let mut offset = 0;

        unsafe {
            let f = |it: *const Lv2AtomEvent| { 
                        if (*it).body.mytype == gate.midi_event {
                            let msg_raw = it.offset(1) as *const u8;
                            let msg = std::slice::from_raw_parts(msg_raw, (*it).body.size as usize);
                            match lv2_midi_message_type(msg) {
                                LV2_Midi_Message_Type::LV2_MIDI_MSG_NOTE_ON => { 
                                        gate.n_active_notes += 1;
                                        println!("NOTE_ON");
                                    },
                                LV2_Midi_Message_Type::LV2_MIDI_MSG_NOTE_OFF => {
                                        gate.n_active_notes -= 1;
                                        println!("NOTE_OFF");
                                    },
                                LV2_Midi_Message_Type::LV2_MIDI_MSG_PGM_CHANGE => {
                                        println!("PGM_CHG");
                                        if (msg[1] == 0) || (msg[1] == 1) { gate.program = msg[1] as u32; }
                                    },
                                _ => return
                            }   
                        }

                        let frames = (*it).time_as_frames();
                        gate.write_output(offset, frames as usize - offset as usize);
                        offset = frames as isize;
                    };

            let ref mut control = *gate.control;
            control.foreach(f);
        }
        gate.write_output(offset, sample_count as usize - offset as usize);
    }            


    pub extern "C" fn connect_port(instance: LV2Handle,
        port: u32,
        data : *mut c_void)
    {
        let mut gate = unsafe { &mut *(instance as *mut MidiGate) };
        let p = PortIndex::from_u32(port);

        println!("connect_port: {}", port);

        match p {
            Some(PortIndex::MGControl) => gate.control = data as *mut LV2_Atom_Sequence,
            Some(PortIndex::MGIn) => gate.input = data as *const f32 ,
            Some(PortIndex::MGOut) => gate.output = data as *mut f32 ,
            None => println!("Not a valid port index: {}", port)
        }
    }


    pub extern "C" fn instantiate(_desc: *const LV2Descriptor,
        _rate: f64,
        _bundle_path: *const c_char,
        _features: *const *const LV2Feature) -> LV2Handle {

            let ptr: *mut MidiGate;

            unsafe {
                let mut map = 0 as *const LV2UridMap;
                let mut i = 0;
                let mut feature = *_features;
                let nul = 0 as *const LV2Feature;

                while feature != nul {
                    let f = CStr::from_ptr((*feature).uri).to_string_lossy().into_owned();
                    
                    println!("URI: {}", f);

                    if f == LV2_URID__MAP {
                        map = (*feature).data as *const LV2UridMap;

                        println!("Found map: {}", f);

                        break;
                    }

                    feature = *_features.offset(i);
                    i += 1;
                }

                if map.is_null() {
                    ptr = 0 as *mut MidiGate;
                } else {
                    let f = (*map).map;
                    let ev = f((*map).handle, LV2_MIDI__MIDIEVENT.as_ptr() as *const c_char);

                    println!("map is: {}", ev);

                    ptr = transmute(Box::new(MidiGate::new(map, &ev)));
                }

            }

            return ptr as LV2Handle;
    }

    pub extern "C" fn cleanup(handle: LV2Handle) {
        unsafe { 
            let _drop: Box<MidiGate> = transmute(((handle as *mut MidiGate)));
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

