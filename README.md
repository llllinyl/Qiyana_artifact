# Qiyana

This is an Rust implementation of the Qiyana scheme, introduiced in "Qiyana: Towards Boolean-Aware System for Oblivious Document Ranking and Retrieval".

# Build
To build and run this code, we need some environmental needs and adjustments.

1. Run `sudo apt-get update && sudo apt-get install -y build-essential` to ensure an up-to-date environment.
2. Our code is under Rust, so we need to install Rust compiler by `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` and `sudo apt install cargo`.

# Project Structure
The project consists of three main protocol implementations:

1. **Baseline** - Baseline implementation

2. **Qiyana** - Enhanced system with optimal communication

3. **Qiyana-wosel** - A trade-off for optimal latency

Each implementation includes:

1. A simulation mode for testing and benchmarking

2. A client-server interactive implementation for real-world usage

# Database
We use the English Wikipedia document dump of 1 December 2025 (`https://dumps.wikimedia.org/enwiki/20251201/enwiki-20251201-pages-articles-multistream.xml.bz2`) as our source corpus. All articles are first extracted using the open-source WikiExtractor tool (`https://github.com/attardi/wikiextractor`). From this set, we randomly select a subset of documents subject to file size constraints using `python extract_docs.py <wiki_folder> <output_folder> --start-index <start_index>` and `python random_select_with_size_range.py <input_folder> <output_folder> --total-files <input_number> --select-count <output_number> --min-size <min_size> --max-size <max_size> --seed 42 --verify`. Finally, we generate keyword sets for the selected documents using three preprocessing scripts: `select_keywords_qiyana.py`, `tfidf_processor.py`, and `find.py`.

# Running Tests
## Protocol Simulations
Run simulation tests for each protocol implementation:
```
# Baseline simulation
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture baseline::baseline_sim::test_baseline_simulate

# Qiyana simulation
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture qiyana::qiyana_sim::test_qiyana_simulate

(Compress rank ciphertext vector)
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture qiyana::qiyana_sim::test_qiyana_zip_simulate

# QiyanaWosel simulation
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture qiyanawosel::qiyanawosel_sim::test_qiyanawosel_simulate
```

## Utility Modules
Test the supporting modules:
```
# Bloom Filter tests
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture utils::bloomfilter

# Decomposition tests
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture utils::decomposition
```

# Running Client-Server Interactive Mode
## Baseline Protocol
1. Start the server:
```
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin baseline_server
```
2. Start the client:
```
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin baseline_client
```

## Qiyana Protocol
1. Start the server:
```
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyana_server
```
2. Start the client:
```
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyana_client
```

To compress a rank ciphertext vector, modify lines 93, 211 and 215 in `qiyana_client.rs`, as well as the comments on lines 287–295 and 313 in `qiyana_server.rs`.

## Qiyana-wosel Protocol
1. Start the server:
```
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawosel_server
```
2. Start the client:
```
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawosel_client
```

## Other
Also, we provide direct shell script invocation, with results saved to two text files in the same directory.
```
cd src/<folder>
chmod +x run_single.sh
./run_single.sh
```

# Running Client-(Master-Workers) Interactive Mode
We provide a shell script to support easy execution of the entire system. For example, you can run the following code to test the performance with 4 workers. If you wish to adjust the number of documents, ports, threads, etc., please modify the settings in the `share.rs` file.
```
cd src/<folder>
chmod +x run_distribute.sh
WORKER_NUM=4 ./run_distribute.sh
```

When using Qiyana, you have the option of providing an additional parameter (`MODE=0/1` to represent standard/compressed communication):
```
WORKER_NUM=2 MODE=1 ./run_distribute.sh
```

# Performance Optimization
The RUSTFLAGS="-C target-cpu=native" flag enables CPU-specific optimizations for maximum performance. This is particularly important for cryptographic operations and large dataset processing.

# Citation
To cite Qiyana in academic papers, please use the following entry:
```bibtex
```