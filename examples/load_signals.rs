// Copyright 2023 The Regents of the University of California
// released under BSD 3-Clause License
// author: Kevin Laeufer <laeufer@berkeley.edu>

use bytesize::ByteSize;
use clap::Parser;
use waveform::*;

#[derive(Parser, Debug)]
#[command(name = "loadfst")]
#[command(author = "Kevin Laeufer <laeufer@berkeley.edu>")]
#[command(version)]
#[command(about = "Loads a FST file into a representation suitable for fast access.", long_about = None)]
struct Args {
    #[arg(value_name = "FSTFILE", index = 1)]
    filename: String,
}

fn print_size_of_full_vs_reduced_names(hierarchy: &Hierarchy) {
    let total_num_elements = hierarchy.iter_vars().len() + hierarchy.iter_scopes().len();
    let reduced_size = hierarchy
        .iter_scopes()
        .map(|s| s.name(hierarchy).bytes().len())
        .sum::<usize>()
        + hierarchy
            .iter_vars()
            .map(|v| v.name(hierarchy).bytes().len())
            .sum::<usize>();
    // to compute full names efficiently, we do need to save a 16-bit parent pointer which takes some space
    let parent_overhead = std::mem::size_of::<u16>() * total_num_elements;
    let full_size = hierarchy
        .iter_scopes()
        .map(|s| s.full_name(hierarchy).bytes().len())
        .sum::<usize>()
        + hierarchy
            .iter_vars()
            .map(|v| v.full_name(hierarchy).bytes().len())
            .sum::<usize>();
    let string_overhead = std::mem::size_of::<String>() * total_num_elements;

    println!("Full vs. partial strings. (Ignoring interning)");
    println!(
        "Saving only the local names uses {}.",
        ByteSize::b((reduced_size + string_overhead) as u64)
    );
    println!(
        "Saving full names would use {}.",
        ByteSize::b((full_size + string_overhead) as u64)
    );
    println!(
        "We saved {}. (actual saving is larger because of interning)",
        ByteSize::b((full_size - reduced_size) as u64)
    )
}

fn main() {
    let args = Args::parse();
    let ext = args.filename.split('.').last().unwrap();
    let start = std::time::Instant::now();
    let mut wave = match ext {
        "fst" => waveform::fst::read(&args.filename),
        "vcd" => waveform::vcd::read(&args.filename).expect("Failed to load VCD."),
        other => panic!("Unsupported file extension: {other}"),
    };
    let load_duration = start.elapsed();
    println!("It took {:?} to load {}", load_duration, args.filename);

    println!(
        "The hierarchy takes up at least {} of memory.",
        ByteSize::b(wave.hierarchy().size_in_memory() as u64)
    );
    print_size_of_full_vs_reduced_names(wave.hierarchy());

    // load every signal individually
    let mut signal_load_times = Vec::new();
    let mut signal_sizes = Vec::new();
    let signal_load_start = std::time::Instant::now();
    for var in wave.hierarchy().get_unique_signals_vars().iter().flatten() {
        let signal_name: String = var.full_name(wave.hierarchy());
        let ids = [var.signal_idx(); 1];
        let start = std::time::Instant::now();
        wave.load_signals(&ids);
        let load_time = start.elapsed();
        let bytes_in_mem = wave.get_signal_size_in_memory(var.signal_idx()).unwrap();
        signal_load_times.push(load_time);
        signal_sizes.push(bytes_in_mem);
    }
    let signal_load_total_duration = signal_load_start.elapsed();
    println!(
        "It took {:?} to load all signals. (and drop them)",
        signal_load_total_duration
    );

    let average_signal_load_time =
        signal_load_times.iter().sum::<std::time::Duration>() / signal_load_times.len() as u32;
    let max_signal_load_time = signal_load_times.iter().max().unwrap();
    let min_signal_load_time = signal_load_times.iter().min().unwrap();
    println!(
        "Loading a signal takes: {:?}..{:?} (avg. {:?})",
        min_signal_load_time, max_signal_load_time, average_signal_load_time
    );

    let average_signal_size = signal_sizes.iter().sum::<usize>() / signal_sizes.len();
    let max_signal_size = *signal_sizes.iter().max().unwrap();
    let min_signal_size = *signal_sizes.iter().min().unwrap();
    println!(
        "Signal take up {}..{} (avg. {})",
        ByteSize::b(min_signal_size as u64),
        ByteSize::b(max_signal_size as u64),
        ByteSize::b(average_signal_size as u64)
    )
}
