include!("share.rs");


#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let worker_id = match args[1].parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            println!("Usage: <command> -- <worker_id> <worker_num>");
            println!("Example: <command> -- 0 2");
            return;
        }
    };
    
    let worker_num = match args[2].parse::<usize>() {
        Ok(num) => num,
        Err(_) => {
            println!("Usage: <command> -- <worker_id> <worker_num>");
            println!("Example: <command> -- 0 2");
            return;
        }
    };
    
    let keyword_file = "/root/Qiyana-experiment/keyword.txt";
    println!("========================================");
    println!("👷 Worker {}/{}", worker_id, worker_num);
    println!("========================================\n");
    println!("📁 Keyword file: {}", keyword_file);

    let worker_port = WORKER_BEGIN_PORT as u32 + worker_id;
    let master_addr = format!("127.0.0.1:{}", MASTER_WORKER_PORT);
    println!("Worker {} will use port {}", worker_id, worker_port);
    println!("1. Connecting to Master at 127.0.0.1:{}...", MASTER_WORKER_PORT);
    let mut master_stream = connect_to_master(worker_id, worker_port).await;
    
    println!("2. Waiting for public parameters from Master...");
    let mut size_buf = [0u8; 8];
    if let Err(e) = master_stream.read_exact(&mut size_buf).await {
        println!("[{}] ❌ Failed to read size header: {}", master_addr, e);
        return;
    }

    let expected_size = u64::from_be_bytes(size_buf) as usize;
    println!("[{}]   Expecting {} bytes total", master_addr, expected_size);

    let mut all_data = Vec::with_capacity(expected_size);
    let mut received = 0;
    let read_start = Instant::now();

    while received < expected_size {
        let mut buffer = vec![0u8; 5 * 1024 * 1024];
        let bytes_to_read = std::cmp::min(buffer.len(), expected_size - received);
        buffer.truncate(bytes_to_read);
        
        match master_stream.read_exact(&mut buffer).await {
            Ok(_) => {
                received += buffer.len();
                all_data.extend_from_slice(&buffer);
            }
            Err(e) => {
                println!("[{}] ❌ Failed to read data chunk: {}", 
                        master_addr, e);
                return;
            }
        }
    }

    let read_time = read_start.elapsed();
    println!("[Worker {}]   ✅ Received all bytes of public parameters, took {:?}", 
        master_addr, read_time);

    match bincode::deserialize::<Vec<Vec<u8>>>(&all_data) {
        Ok(params_pack) => {
            println!("[Worker {}]   ✅ Public parameters package deserialized, {} params", 
                    worker_id, params_pack.len());
            
            if params_pack.len() != 7 {
                println!("[{}] ❌ Expected 7 parameters, got {}", master_addr, params_pack.len());
                return;
            }

            let deserialize_start = Instant::now(); 
            let seeded_bsk: SeededLweBootstrapKey<Vec<u64>> = match bincode::deserialize(&params_pack[0]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize fourier_bsk: {}", master_addr, e);
                    return;
                }
            };
            
            let seeded_ksk: SeededLweKeyswitchKey<Vec<u64>> = match bincode::deserialize(&params_pack[1]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize ksk_big_to_small: {}", master_addr, e);
                    return;
                }
            };
            
            let equal_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[2]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize equal_lut: {}", master_addr, e);
                    return;
                }
            };
            
            let and_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[3]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize and_lut: {}", master_addr, e);
                    return;
                }
            };
            
            let or_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[4]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize or_lut: {}", master_addr, e);
                    return;
                }
            };

            let one: LweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[5]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize one cipher: {}", master_addr, e);
                    return;
                }
            };

            let zero: LweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[6]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize zero cipher: {}", master_addr, e);
                    return;
                }
            };
            
            let deserialize_time = deserialize_start.elapsed();
            println!("[Worker {}]   ✅ All parameters deserialized in {:?}", 
                    worker_id, deserialize_time);
            println!("[Worker {}] 2. Server preprocessing...", worker_id);
            
            let server_init_start = Instant::now();
            let server = SubServer::new(worker_num,
                worker_id,
                seeded_bsk, 
                seeded_ksk, 
                equal_lut,
                and_lut,
                or_lut,
                one,
                zero,
                keyword_file);
            let server_init_time = server_init_start.elapsed();
            
            println!("[Worker {}]   Server initialized in {:?}", worker_id, server_init_time);
            println!("[Worker {}]   Number of documents: {}", worker_id, DOCUMENT_NUM);
            println!("[Worker {}]   ✅ Preprocessing completed", worker_id);

            let _init_elapsed = server_init_start.elapsed();

            let wait_start = Instant::now();
            let mut dots = 0;
            let status_handle = tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(1)).await;
                    print!(".");
                    let _ = std::io::stdout().flush();
                    dots += 1;
                    
                    if dots % 60 == 0 {
                        let elapsed = wait_start.elapsed();
                        println!("\n   Still waiting... ({:?} elapsed)", 
                            elapsed);
                    }
                }
            });
            println!("[Worker {}] 3. Waiting for query from Master...", worker_id);
            let mut query_size_buf = [0u8; 8];
            if let Err(e) = master_stream.read_exact(&mut query_size_buf).await {
                println!("[{}] ❌ Failed to read query size header: {}", master_addr, e);
                println!("   Wait time: {:?}", wait_start.elapsed());
                status_handle.abort();
                return;
            }

            let expected_query_size = u64::from_be_bytes(query_size_buf) as usize;
            println!("[{}]   Expecting query: {} bytes", master_addr, expected_query_size);

            let mut query_data = Vec::with_capacity(expected_query_size);
            let mut query_received = 0;
            let query_read_start = Instant::now();
            let mut query_chunk_count = 0;
            let query_chunk_size = 5 * 1024 * 1024;

            while query_received < expected_query_size {
                let remaining = expected_query_size - query_received;
                let current_chunk_size = query_chunk_size.min(remaining);
                let mut chunk_buf = vec![0; current_chunk_size];
                
                if let Err(e) = master_stream.read_exact(&mut chunk_buf).await {
                    println!("[{}] ❌ Failed to read query chunk at {} bytes: {}", 
                            master_addr, query_received, e);
                    return;
                }
                
                query_data.extend_from_slice(&chunk_buf);
                query_received += current_chunk_size;
                query_chunk_count += 1;
            }

            let query_read_time = query_read_start.elapsed();
            println!("[{}]   ✅ Received query ({} bytes) in {:?} ({} chunks, {:.2} MB/s)", 
                master_addr, query_received, query_read_time, query_chunk_count,
                (query_received as f64 / 1024.0 / 1024.0) / query_read_time.as_secs_f64());

            println!("[Worker {}]   Deserializing query...", worker_id);
            
            match bincode::deserialize::<Vec<Vec<u8>>>(&query_data) {
                Ok(query_vec) => {
                    println!("[Worker {}]   ✅ Query package deserialized, {} params", 
                            worker_id, query_vec.len());
                    
                    if query_vec.len() != 3 {
                        println!("[{}] ❌ Expected 3 query element, got {}", master_addr, params_pack.len());
                        return;
                    }
                    let deserialize_start = Instant::now();

                    let query: Vec<Vec<LweCiphertextOwned<u64>>> = match bincode::deserialize(&query_vec[0]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize query: {}", master_addr, e);
                            return;
                        }
                    };
                    
                    let querysum: Vec<LweCiphertextOwned<u64>> = match bincode::deserialize(&query_vec[1]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize querysum: {}", master_addr, e);
                            return;
                        }
                    };
                    
                    let template: String = match bincode::deserialize(&query_vec[2]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize template: {}", master_addr, e);
                            return;
                        }
                    };
                    let deserialize_time = deserialize_start.elapsed();
                    println!("[Worker {}]    ✅ All query deserialized in {:?}", 
                            worker_id, deserialize_time);

                    println!("[Worker {}] 4. Processing query...", worker_id);
                    let process_start = Instant::now();
                   
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(THREAD_NUM)
                        .build_global()
                        .unwrap();
                    let response = server.qiyana1_response(query, querysum, template);
                    println!("[Worker {}]   ✅ Query processed in {:?}", worker_id, process_start.elapsed());

                    let client_addr = format!("127.0.0.1:{}", CLIENT_RESULT_PORT);
                    println!("[Worker {}] 5. Connecting and sending to Client at {}...", worker_id, client_addr);
                    send_response_to_client(worker_id, response).await;
                    println!("   ✅ Task spawned for Worker {}", worker_id);  
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize query: {}", master_addr, e);
                    return;
                }
            }
        }
        Err(e) => {
            println!("[{}] ❌ Failed to deserialize public parameters: {}", master_addr, e);
        }
    }
}

async fn connect_to_master(worker_id: u32, worker_bind_port: u32) -> TcpStream {
    let master_addr = format!("127.0.0.1:{}", MASTER_WORKER_PORT);
    
    for attempt in 1..=5 {
        let socket = tokio::net::TcpSocket::new_v4()
            .expect("Failed to create socket");
        
        let bind_addr = format!("127.0.0.1:{}", worker_bind_port)
            .parse::<std::net::SocketAddr>()
            .expect("Failed to parse bind address");
        
        match socket.bind(bind_addr) {
            Ok(_) => {
                match socket.connect(master_addr.parse().unwrap()).await {
                    Ok(stream) => {
                        println!("   ✅ Worker  connected from port {}", worker_bind_port);
                        return stream;
                    }
                    Err(e) => {
                        println!("   ⏳ Worker {} attempt {} connect failed: {}", worker_bind_port, attempt, e);
                    }
                }
            }
            Err(e) => {
                println!("   ⏳ Worker {} attempt {} bind failed (port {}): {}", 
                         worker_id, attempt, worker_bind_port, e);
            }
        }
        
        sleep(Duration::from_secs(1)).await;
    }
    
    println!("   ⚠️ Worker {} failed to bind to port {}, using random port", 
             worker_id, worker_bind_port);

    TcpStream::connect(master_addr).await
        .expect(&format!("Worker {} failed to connect to Master", worker_id))
}

async fn send_response_to_client(worker_id: u32, response: Vec<LweCiphertextOwned<u64>>) {
    let client_addr = format!("127.0.0.1:{}", CLIENT_RESULT_PORT);

    let mut attempts = 0;
    loop {
        attempts += 1;
        println!("   🔗 [Worker {}]: Attempt {} to connect...", worker_id, attempts);
        
        match TcpStream::connect(&client_addr).await {
            Ok(mut stream) => {
                    println!("[Worker {}]  Sending response...", worker_id);
                    let response_start = Instant::now();

                    let response_bytes = bincode::serialize(&response).unwrap();
                    let response_size = response_bytes.len();
                    println!("[Worker {}]  Response size: {} bytes", worker_id, response_size);

                    let size_bytes = (response_size as u64).to_be_bytes();
                    match stream.write_all(&size_bytes).await {
                        Ok(()) => {}
                        Err(e) => {
                            println!("   ❌ Failed to send response size: {}", e);
                            return;
                        }
                    }

                    let id_bytes = (worker_id as u64).to_be_bytes();
                    match stream.write_all(&id_bytes).await {
                        Ok(()) => {}
                        Err(e) => {
                            println!("   ❌ Failed to send worker id: {}", e);
                            return;
                        }
                    }
                    
                    let chunk_size = 5 * 1024 * 1024;
                    let mut sent = 0;
                    let mut chunk_count = 0;

                    while sent < response_size {
                        let remaining = response_size - sent;
                        let current_chunk_size = chunk_size.min(remaining);
                        let chunk = &response_bytes[sent..sent + current_chunk_size];
                        
                        match stream.write_all(chunk).await {
                            Ok(()) => {
                                sent += current_chunk_size;
                                chunk_count += 1;
                            }
                            Err(e) => {
                                println!("[Worker {}] ❌ Failed to send response chunk at {} bytes: {}", 
                                        worker_id, sent, e);
                                return;
                            }
                        }
                    }

                    let send_time = response_start.elapsed();
                    println!("[{}]   ✅ Response sent in {:?} ({} chunks, {:.2} MB/s)", 
                            client_addr, send_time, chunk_count,
                            (response_size as f64 / 1024.0 / 1024.0) / send_time.as_secs_f64());

                return;
            }
            Err(e) => {
                println!("   ❌ Worker {}: Failed ({}), retrying in 1s...", worker_id, e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                if attempts > 30 {
                    println!("   💀 Worker {}: Too many failures, giving up", worker_id);
                    return;
                }
            }
        }
    }
}
