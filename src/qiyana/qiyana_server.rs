include!("qiyana_sim.rs");
use std::error::Error;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:9999";
    let keyword_file = "/root/Qiyana-experiment/keyword.txt";
    let tfidf_path = "/root/Qiyana-experiment/tf-idf.txt";
    
    println!("========================================");
    println!("🖥️  Qiyana Server");
    println!("========================================\n");
    
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("✅ Server started successfully");
    println!("📍 Listening on: {}", addr);
    println!("📁 Keyword file: {}", keyword_file);
    println!("📁 TF-IDF file: {}", tfidf_path);
    
    println!("\nWaiting for ONE client connection...");
    
    let (socket, client_addr) = listener.accept().await.unwrap();
    println!("\n📡 Client connected: {}", client_addr);
    println!("This server will accept only ONE client and then exit.");
    
    let _ = handle_client(socket, client_addr, keyword_file, tfidf_path).await;
    
    println!("🔴 Server shutting down...");
    println!("Goodbye!");
}

async fn handle_client(
    mut socket: TcpStream, 
    client_addr: std::net::SocketAddr,
    keyword_file: &str,
    tfidf_path: &str
) -> Result<(), Box<dyn Error>> {
    println!("[{}] 1. Receiving public parameters...", client_addr);

    let mut size_buf = [0u8; 8];
    if let Err(e) = socket.read_exact(&mut size_buf).await {
        println!("[{}] ❌ Failed to read size header: {}", client_addr, e);
        return Ok(());
    }

    let expected_size = u64::from_be_bytes(size_buf) as usize;
    println!("[{}]   Expecting {} bytes total", client_addr, expected_size);

    let mut all_data = Vec::with_capacity(expected_size);
    let mut received = 0;
    let read_start = Instant::now();
    let mut chunk_count = 0;

    while received < expected_size {
        let mut buffer = vec![0u8; 5 * 1024 * 1024];
        let bytes_to_read = std::cmp::min(buffer.len(), expected_size - received);
        buffer.truncate(bytes_to_read);
        
        match socket.read_exact(&mut buffer).await {
            Ok(_) => {
                received += buffer.len();
                all_data.extend_from_slice(&buffer);
                chunk_count += 1;
            }
            Err(e) => {
                println!("[{}] ❌ Failed to read data chunk {}: {}", 
                        client_addr, chunk_count, e);
                return Ok(());
            }
        }
    }

    let read_time = read_start.elapsed();
    println!("[{}]   ✅ Received all {} bytes in {} chunks, took {:?}", 
            client_addr, received, chunk_count, read_time);

    match bincode::deserialize::<Vec<Vec<u8>>>(&all_data) {
        Ok(params_pack) => {
            println!("[{}]   ✅ Public parameters package deserialized, {} params", 
                    client_addr, params_pack.len());
            
            if params_pack.len() != 9 {
                println!("[{}] ❌ Expected 9 parameters, got {}", client_addr, params_pack.len());
                return Ok(());
            }

            let deserialize_start = Instant::now();
            
            let fourier_bsk: SeededLweBootstrapKey<Vec<u64>> = match bincode::deserialize(&params_pack[0]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize fourier_bsk: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let ksk_big_to_small: SeededLweKeyswitchKey<Vec<u64>> = match bincode::deserialize(&params_pack[1]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize ksk_big_to_small: {}", client_addr, e);
                    return Ok(());
                }
            };

            let packing_key: SeededLwePackingKeyswitchKey<Vec<u64>> = match bincode::deserialize(&params_pack[2]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize ksk_big_to_small: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let equal_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[3]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize equal_lut: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let and_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[4]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize and_lut: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let or_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[5]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize or_lut: {}", client_addr, e);
                    return Ok(());
                }
            };

            let mul_lut: GlweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[6]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize or_lut: {}", client_addr, e);
                    return Ok(());
                }
            };

            let one: LweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[7]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize one cipher: {}", client_addr, e);
                    return Ok(());
                }
            };

            let zero: LweCiphertextOwned<u64> = match bincode::deserialize(&params_pack[8]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize zero cipher: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let deserialize_time = deserialize_start.elapsed();
            println!("[{}]   ✅ All parameters deserialized in {:?}", 
                    client_addr, deserialize_time);

            println!("[{}] 2. Server preprocessing for 10 seconds...", client_addr);
            
            let server_init_start = Instant::now();
            let server = Server::new(fourier_bsk, 
                ksk_big_to_small, 
                packing_key,
                equal_lut,
                and_lut,
                or_lut,
                mul_lut,
                one,
                zero,
                keyword_file,
                tfidf_path);
            let server_init_time = server_init_start.elapsed();
            
            println!("[{}]   Server initialized in {:?}", client_addr, server_init_time);
            println!("[{}]   Number of documents: {}", client_addr, DOCUMENT_NUM);
            
            let init_elapsed = server_init_start.elapsed();
            let remaining_wait = if init_elapsed < Duration::from_secs(10) {
                Duration::from_secs(10) - init_elapsed
            } else {
                Duration::from_secs(0)
            };

            if remaining_wait > Duration::from_secs(0) {
                sleep(Duration::from_secs(remaining_wait.as_secs())).await;
            }
            println!("[{}]   ✅ Preprocessing completed", client_addr);

            println!("[{}] 3. Waiting for query...", client_addr);
            let mut query_size_buf = [0u8; 8];
            if let Err(e) = socket.read_exact(&mut query_size_buf).await {
                println!("[{}] ❌ Failed to read query size header: {}", client_addr, e);
                return Ok(());
            }

            let expected_query_size = u64::from_be_bytes(query_size_buf) as usize;
            println!("[{}]   Expecting query: {} bytes", client_addr, expected_query_size);

            let mut query_data = Vec::with_capacity(expected_query_size);
            let mut query_received = 0;
            let query_read_start = Instant::now();
            let mut query_chunk_count = 0;
            let query_chunk_size = 5 * 1024 * 1024;

            while query_received < expected_query_size {
                let remaining = expected_query_size - query_received;
                let current_chunk_size = query_chunk_size.min(remaining);
                let mut chunk_buf = vec![0; current_chunk_size];
                
                if let Err(e) = socket.read_exact(&mut chunk_buf).await {
                    println!("[{}] ❌ Failed to read query chunk at {} bytes: {}", 
                            client_addr, query_received, e);
                    return Ok(());
                }
                
                query_data.extend_from_slice(&chunk_buf);
                query_received += current_chunk_size;
                query_chunk_count += 1;
            }

            let query_read_time = query_read_start.elapsed();
            println!("[{}]   ✅ Received query ({} bytes) in {:?} ({} chunks, {:.2} MB/s)", 
                client_addr, query_received, query_read_time, query_chunk_count,
                (query_received as f64 / 1024.0 / 1024.0) / query_read_time.as_secs_f64());

            println!("[{}]   Deserializing query...", client_addr);
            
            match bincode::deserialize::<Vec<Vec<u8>>>(&query_data) {
                Ok(query_vec) => {
                    println!("[{}]   ✅ Query package deserialized, {} params", 
                            client_addr, query_vec.len());
                    
                    if query_vec.len() != 4 {
                        println!("[{}] ❌ Expected 4 query element, got {}", client_addr, params_pack.len());
                        return Ok(());
                    }
                    let deserialize_start = Instant::now();

                    let query: Vec<Vec<LweCiphertextOwned<u64>>> = match bincode::deserialize(&query_vec[0]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize query: {}", client_addr, e);
                            return Ok(());
                        }
                    };
                    
                    let querysum: Vec<LweCiphertextOwned<u64>> = match bincode::deserialize(&query_vec[1]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize querysum: {}", client_addr, e);
                            return Ok(());
                        }
                    };
                    
                    let template: String = match bincode::deserialize(&query_vec[2]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize template: {}", client_addr, e);
                            return Ok(());
                        }
                    };

                    // let rank_vector: Vec<LweCiphertextOwned<u64>> = match bincode::deserialize(&query_vec[3]) {
                    //     Ok(param) => {
                    //         param
                    //     }
                    //     Err(e) => {
                    //         println!("[{}] ❌ Failed to deserialize querysum: {}", client_addr, e);
                    //         return Ok(());
                    //     }
                    // };
                    let rank_vector: Vec<SeededLweCiphertext<u64>> = match bincode::deserialize(&query_vec[3]) {
                        Ok(param) => {
                            param
                        }
                        Err(e) => {
                            println!("[{}] ❌ Failed to deserialize querysum: {}", client_addr, e);
                            return Ok(());
                        }
                    };

                    let deserialize_time = deserialize_start.elapsed();
                    println!("[{}]   ✅ All query deserialized in {:?}", 
                            client_addr, deserialize_time);

                    println!("[{}] 4. Processing query...", client_addr);
                    let process_start = Instant::now();
                    
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(THREAD_NUM)
                        .build_global()
                        .unwrap();
                    // let response = server.qiyana_response(query, querysum, template, rank_vector);
                    let response = server.qiyana_decompress_response(query, querysum, template, rank_vector);
                    
                    let process_time = process_start.elapsed();
                    
                    println!("[{}]   ✅ Query processed in {:?}", client_addr, process_time);
                    
                    println!("[{}] 5. Sending response...", client_addr);
                    let response_start = Instant::now();

                    let response_bytes = bincode::serialize(&response)?;
                    let response_size = response_bytes.len();
                    println!("[{}]   Response size: {} bytes", client_addr, response_size);

                    let size_bytes = (response_size as u64).to_be_bytes();
                    socket.write_all(&size_bytes).await?;
                    println!("[{}]   Sent response size header", client_addr);

                    let chunk_size = 5 * 1024 * 1024;
                    let mut sent = 0;
                    let mut chunk_count = 0;

                    while sent < response_size {
                        let remaining = response_size - sent;
                        let current_chunk_size = chunk_size.min(remaining);
                        let chunk = &response_bytes[sent..sent + current_chunk_size];
                        
                        match socket.write_all(chunk).await {
                            Ok(()) => {
                                sent += current_chunk_size;
                                chunk_count += 1;
                            }
                            Err(e) => {
                                println!("[{}] ❌ Failed to send response chunk at {} bytes: {}", 
                                        client_addr, sent, e);
                                return Err(e.into());
                            }
                        }
                    }

                    let send_time = response_start.elapsed();
                    println!("[{}]   ✅ Response sent in {:?} ({} chunks, {:.2} MB/s)", 
                            client_addr, send_time, chunk_count,
                            (response_size as f64 / 1024.0 / 1024.0) / send_time.as_secs_f64());

                    let total_time = read_start.elapsed();
                    println!("[{}] 📊 Session statistics:", client_addr);
                    println!("[{}]   - Receive public params: {:?}", client_addr, read_time);
                    println!("[{}]   - Server init: {:?}", client_addr, server_init_time);
                    println!("[{}]   - Preprocess wait: 10 seconds", client_addr);
                    println!("[{}]   - Receive query: {:?}", client_addr, query_read_time);
                    println!("[{}]   - Process query: {:?}", client_addr, process_time);
                    println!("[{}]   - Send response: {:?}", client_addr, send_time);
                    println!("[{}]   - TOTAL TIME: {:?}", client_addr, total_time);
                    
                    println!("[{}] 🎉 Session completed successfully!", client_addr);
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize query: {}", client_addr, e);
                    return Ok(());
                }
            }
        }
        Err(e) => {
            println!("[{}] ❌ Failed to deserialize public parameters: {}", client_addr, e);
        }
    }

    sleep(Duration::from_secs(1)).await;
    
    println!("[{}] Connection closed", client_addr);
    Ok(())
}
