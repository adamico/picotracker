// PROTOTYPE — throwaway playback spike for adamico/picotracker#15. Never merged.
//
// Question: is a faithful-enough PICO-8 emulation achievable with a Rust port
// of zepto8's synth (+ fake-08's fixes) played through cpal 0.18?
//
//   cargo run -- <cart.p8|cart.p8.png> info
//   cargo run -- <cart> sfx <n>            [--wav out.wav] [--secs S]
//   cargo run -- <cart> music <pattern>    [--wav out.wav] [--secs S] [--mute 1,3]
//
// Without --wav it plays live through the default output device and prints a
// line every time a channel moves to a new note.

mod cart;
mod synth;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use synth::{Mixer, RATE};

const NOTE_NAMES: [&str; 12] = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];

fn note_str(c: &cart::Cart, sfx: i32, offset: f64) -> String {
    if sfx < 0 || offset >= 32.0 {
        return "....  .....".into();
    }
    let n = c.sfx[sfx as usize].notes[offset as usize];
    if n.volume == 0 {
        return format!("{:02x}:{:02} ...", sfx, offset as usize);
    }
    format!(
        "{:02x}:{:02} {}{}{}{}{}{}",
        sfx,
        offset as usize,
        NOTE_NAMES[n.key as usize % 12],
        n.key / 12,
        if n.custom { 'i' } else { 'w' },
        n.instrument,
        n.volume,
        n.effect
    )
}

/// Snapshot of what each channel is playing, used to print a line per change.
#[derive(PartialEq, Clone)]
struct Snap {
    pattern: i32,
    rows: [(i32, i64); 4],
}

fn snap(m: &Mixer) -> Snap {
    Snap {
        pattern: m.music.pattern,
        rows: std::array::from_fn(|i| {
            let s = &m.channels[i].main_sfx;
            (s.sfx, if s.sfx < 0 { -1 } else { s.offset.floor() as i64 })
        }),
    }
}

fn line(m: &Mixer, t: f64) -> String {
    let mut s = format!("{:7.3}s P{:>3} |", t, if m.music.pattern < 0 { "--".into() } else { format!("{:02}", m.music.pattern) });
    for ch in &m.channels {
        s += &format!(" {} |", note_str(&m.cart, ch.main_sfx.sfx, ch.main_sfx.offset));
    }
    s
}

fn info(c: &cart::Cart) {
    println!("SFX (spd, loop/LEN, filters byte, notes used, instruments):");
    for (i, s) in c.sfx.iter().enumerate() {
        let used = s.notes.iter().filter(|n| n.volume > 0).count();
        if used == 0 {
            continue;
        }
        let mut inst: Vec<String> = s.notes.iter().filter(|n| n.volume > 0)
            .map(|n| format!("{}{}", if n.custom { "i" } else { "w" }, n.instrument)).collect();
        inst.sort();
        inst.dedup();
        let lp = if s.loop_end == 0 && s.loop_start > 0 { format!("LEN {}", s.loop_start) }
                 else if s.loop_end > s.loop_start { format!("loop {}-{}", s.loop_start, s.loop_end) }
                 else { "-".into() };
        let wf = if i < 8 && s.is_waveform_instrument() { "  WAVEFORM INSTRUMENT (unsupported)" } else { "" };
        println!("  {:02} spd {:3} {:10} f={:02x} notes {:2} {}{}", i, s.speed, lp, s.filters, used, inst.join(","), wf);
    }
    println!("Patterns:");
    for (i, p) in c.song.iter().enumerate() {
        if (0..4).all(|ch| p.sfx(ch) & 0x40 != 0) {
            continue;
        }
        let chans: Vec<String> = (0..4).map(|ch| {
            let n = p.sfx(ch);
            if n & 0x40 != 0 { "--".into() } else { format!("{:02}", n) }
        }).collect();
        println!("  {:02} [{}]{}{}{}", i, chans.join(" "),
            if p.start() { " START" } else { "" }, if p.loop_back() { " LOOP" } else { "" }, if p.stop() { " STOP" } else { "" });
    }
}

fn write_wav(path: &str, samples: &[f32]) {
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&22050u32.to_le_bytes());
    out.extend_from_slice(&44100u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, out).expect("write wav");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: playback-spike <cart> info | sfx <n> | music <pattern>  [--wav out.wav] [--secs S] [--mute 1,3]");
        std::process::exit(2);
    }
    let cart = cart::load(&args[1]);
    if args[2] == "info" {
        info(&cart);
        return;
    }
    let what = args[2].clone();
    let n: i32 = args.get(3).and_then(|s| s.parse().ok()).expect("number");
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let wav = opt("--wav");
    let max_secs: f64 = opt("--secs").and_then(|s| s.parse().ok()).unwrap_or(120.0);

    let mut mixer = Mixer::new(cart);
    if let Some(m) = opt("--mute") {
        for c in m.split(',').filter_map(|c| c.parse::<usize>().ok()) {
            mixer.mute[c] = true;
        }
    }
    match what.as_str() {
        "sfx" => mixer.launch_sfx(n, 0, 0.0, 0.0, false),
        "music" => mixer.play_music(n),
        _ => panic!("sfx or music"),
    }

    if let Some(path) = wav {
        // Offline render at 22050 Hz, the rate PICO-8 itself mixes at.
        let mut out = Vec::new();
        let mut last = snap(&mixer);
        println!("{}", line(&mixer, 0.0));
        while (out.len() as f64) < max_secs * RATE && !mixer.is_silent() {
            out.push(mixer.sample());
            let s = snap(&mixer);
            if s != last {
                println!("{}", line(&mixer, out.len() as f64 / RATE));
                last = s;
            }
        }
        write_wav(&path, &out);
        println!("wrote {} ({:.2}s)", path, out.len() as f64 / RATE);
        return;
    }

    // Live playback: render at 22050 Hz and resample (linear) to the device rate.
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device");
    let config = device.default_output_config().expect("no output config");
    let dev_rate = config.sample_rate() as f64;
    let chans = config.channels() as usize;
    println!("device rate {} Hz, {} channels, format {:?}", dev_rate, chans, config.sample_format());

    let (tx, rx) = mpsc::channel::<String>();
    let step = RATE / dev_rate;
    let mut frac = 1.0f64;
    let (mut a, mut b) = (0.0f32, 0.0f32);
    let mut produced: u64 = 0;
    let mut last = snap(&mixer);
    let mut done_sent = false;
    let stream_cfg: cpal::StreamConfig = config.clone().into();
    let stream = device
        .build_output_stream(
            stream_cfg,
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(chans) {
                    frac += step;
                    while frac >= 1.0 {
                        frac -= 1.0;
                        a = b;
                        b = mixer.sample();
                        produced += 1;
                        let s = snap(&mixer);
                        if s != last {
                            let _ = tx.send(line(&mixer, produced as f64 / RATE));
                            last = s;
                        }
                    }
                    let v = a + (b - a) * frac as f32;
                    for x in frame.iter_mut() {
                        *x = v;
                    }
                }
                if !done_sent && (mixer.is_silent() || produced as f64 > max_secs * RATE) {
                    let _ = tx.send("DONE".into());
                    done_sent = true;
                }
            },
            |e| eprintln!("stream error: {e}"),
            None,
        )
        .expect("build stream (f32)");
    stream.play().expect("play");
    for msg in rx {
        if msg == "DONE" {
            break;
        }
        println!("{msg}");
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
}
