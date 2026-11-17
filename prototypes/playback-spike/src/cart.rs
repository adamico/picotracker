// PROTOTYPE — throwaway. Reads the Song (0x3100) and SFX (0x3200) bytes from a
// .p8 text Cart or a .p8.png Cart. No error handling beyond "it runs".

#[derive(Clone, Copy, Default, Debug)]
pub struct Note {
    pub key: u8,        // 0..63
    pub instrument: u8, // 0..7
    pub volume: u8,     // 0..7
    pub effect: u8,     // 0..7
    pub custom: bool,   // instrument is SFX instrument 0..7
}

impl Note {
    pub fn from_word(w: u16) -> Note {
        Note {
            key: (w & 0x3f) as u8,
            instrument: ((w >> 6) & 7) as u8,
            volume: ((w >> 9) & 7) as u8,
            effect: ((w >> 12) & 7) as u8,
            custom: w & 0x8000 != 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sfx {
    pub notes: [Note; 32],
    pub filters: u8, // byte 64: editor bit + NOIZ/BUZZ bits + base-3 DETUNE/REVERB/DAMPEN
    pub speed: u8,
    pub loop_start: u8,
    pub loop_end: u8,
}

impl Default for Sfx {
    fn default() -> Self {
        Sfx { notes: [Note::default(); 32], filters: 0, speed: 16, loop_start: 0, loop_end: 0 }
    }
}

impl Sfx {
    /// Waveform-instrument flag per the community layout (byte 66 bit 7). Unverified.
    pub fn is_waveform_instrument(&self) -> bool {
        self.loop_start & 0x80 != 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pattern {
    pub bytes: [u8; 4], // raw RAM bytes: bits 0-5 SFX, bit 6 muted, bit 7 flag
}

impl Default for Pattern {
    fn default() -> Self {
        Pattern { bytes: [0x41, 0x42, 0x43, 0x44] }
    }
}

impl Pattern {
    pub fn sfx(&self, ch: usize) -> u8 { self.bytes[ch] & 0x7f }
    pub fn start(&self) -> bool { self.bytes[0] & 0x80 != 0 }
    pub fn loop_back(&self) -> bool { self.bytes[1] & 0x80 != 0 }
    pub fn stop(&self) -> bool { self.bytes[2] & 0x80 != 0 }
}

pub struct Cart {
    pub sfx: [Sfx; 64],
    pub song: [Pattern; 64],
}

pub fn load(path: &str) -> Cart {
    if path.ends_with(".png") { load_png(path) } else { load_p8(path) }
}

fn hex(s: &str) -> Vec<u8> {
    s.chars().filter_map(|c| c.to_digit(16)).map(|d| d as u8).collect()
}

fn load_p8(path: &str) -> Cart {
    let text = std::fs::read_to_string(path).expect("read cart");
    let mut cart = Cart { sfx: [Sfx::default(); 64], song: [Pattern::default(); 64] };
    let mut section = "";
    let (mut si, mut mi) = (0usize, 0usize);
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("__") && t.ends_with("__") && t.len() > 4 {
            section = if t == "__sfx__" { "sfx" } else if t == "__music__" { "music" } else { "" };
            continue;
        }
        match section {
            "sfx" if si < 64 => {
                let d = hex(t);
                if d.len() < 168 { continue; }
                let b = |i: usize| d[i * 2] << 4 | d[i * 2 + 1];
                let s = &mut cart.sfx[si];
                s.filters = b(0);
                s.speed = b(1);
                s.loop_start = b(2);
                s.loop_end = b(3);
                for n in 0..32 {
                    let o = 8 + n * 5;
                    let inst = d[o + 2];
                    s.notes[n] = Note {
                        key: (d[o] << 4 | d[o + 1]) & 0x3f,
                        instrument: inst & 7,
                        custom: inst >= 8,
                        volume: d[o + 3] & 7,
                        effect: d[o + 4] & 7,
                    };
                }
                si += 1;
            }
            "music" if mi < 64 => {
                let d = hex(t);
                if d.len() < 10 { continue; }
                let flags = d[0] << 4 | d[1];
                let mut p = Pattern { bytes: [0; 4] };
                for ch in 0..4 {
                    let c = d[2 + ch * 2] << 4 | d[3 + ch * 2];
                    p.bytes[ch] = (c & 0x7f) | (((flags >> ch) & 1) << 7);
                }
                cart.song[mi] = p;
                mi += 1;
            }
            _ => {}
        }
    }
    cart
}

fn load_png(path: &str) -> Cart {
    let mut dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).expect("open png")));
    dec.set_transformations(png::Transformations::EXPAND);
    let mut reader = dec.read_info().expect("png info");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    assert_eq!((info.width, info.height), (160, 205), "not a PICO-8 cart png");
    let chans = info.color_type.samples();
    let rom: Vec<u8> = buf[..info.buffer_size()]
        .chunks(chans)
        .map(|p| {
            let (r, g, b) = (p[0], p[1], p[2]);
            let a = if chans == 4 { p[3] } else { 255 };
            ((a & 3) << 6) | ((r & 3) << 4) | ((g & 3) << 2) | (b & 3)
        })
        .collect();
    let mut cart = Cart { sfx: [Sfx::default(); 64], song: [Pattern::default(); 64] };
    for p in 0..64 {
        let o = 0x3100 + p * 4;
        cart.song[p].bytes.copy_from_slice(&rom[o..o + 4]);
    }
    for i in 0..64 {
        let o = 0x3200 + i * 68;
        let s = &mut cart.sfx[i];
        for n in 0..32 {
            s.notes[n] = Note::from_word(u16::from_le_bytes([rom[o + n * 2], rom[o + n * 2 + 1]]));
        }
        s.filters = rom[o + 64];
        s.speed = rom[o + 65];
        s.loop_start = rom[o + 66];
        s.loop_end = rom[o + 67];
    }
    cart
}
