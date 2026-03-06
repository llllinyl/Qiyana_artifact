include!("share.rs");


#[tokio::main]
async fn main() {
    println!("========================================");
    println!("🔍 Distributed Client");
    println!("========================================\n");
    
    let args: Vec<String> = std::env::args().collect();
    let worker_num = match args[1].parse::<usize>() {
        Ok(num) => num,
        Err(_) => {
            println!("Usage: <command> -- <worker_id> <worker_num> <MODE>");
            println!("Example: <command> -- 0 2 1");
            return;
        }
    };

    let mode = match args[2].parse::<usize>() {
        Ok(num) => num,
        Err(_) => {
            println!("Usage: <command> -- <worker_id> <worker_num> <MODE>");
            println!("Example: <command> -- 0 2 1");
            return;
        }
    };

    println!("1. Starting result listener on port {}...", CLIENT_RESULT_PORT);
    let result_listener = match TcpListener::bind(format!("0.0.0.0:{}", CLIENT_RESULT_PORT)).await {
        Ok(listener) => listener,
        Err(e) => {
            println!("Failed to bind result listener: {}", e);
            return;
        }
    };
    println!("   ✅ Result listener ready");
    
    println!("2. Connecting to Master (127.0.0.1:{})...", MASTER_PORT);
    let server_addr = format!("127.0.0.1:{}", MASTER_PORT);
    let connect_result = timeout(
        Duration::from_secs(10),
        TcpStream::connect(server_addr.clone())
    ).await;
    
    let mut master_stream = match connect_result {
        Ok(Ok(stream)) => {
            println!("   ✅ Connect successfully!");
            stream
        }
        Ok(Err(e)) => {
            println!("   ❌ Failed to connect: {}", e);
            return;
        }
        Err(_) => {
            println!("   ⏰ Connect timeout (10 seconds)");
            return;
        }
    };

    println!("3. Initialize TFHE Client and send public parameters to Master...");
    let client = Client::new();
    let params_vec = vec![
        bincode::serialize(&client.seeded_bsk).unwrap(),
        bincode::serialize(&client.seeded_ksk).unwrap(),
        bincode::serialize(&client.seeded_pk).unwrap(),
        bincode::serialize(&client.equal_lut).unwrap(),
        bincode::serialize(&client.and_lut).unwrap(),
        bincode::serialize(&client.or_lut).unwrap(),
        bincode::serialize(&client.mul_lut).unwrap(),
        bincode::serialize(&client.one).unwrap(),
        bincode::serialize(&client.zero).unwrap(),
    ];

    let serialized_pp = bincode::serialize(&params_vec).unwrap();
    let pp_size = serialized_pp.len();
    println!("   Public parameters size: {} bytes", pp_size);

    let size_bytes = (pp_size as u64).to_be_bytes();
    if let Err(e) = master_stream.write_all(&size_bytes).await {
        println!("   ❌ Failed to send size header: {}", e);
        return;
    }

    let chunk_size = 5 * 1024 * 1024;
    let mut sent = 0;

    while sent < pp_size {
        let remaining = pp_size - sent;
        let current_chunk_size = chunk_size.min(remaining);
        let chunk = &serialized_pp[sent..sent + current_chunk_size];
        
        match master_stream.write_all(chunk).await {
            Ok(()) => {
                sent += current_chunk_size;
            }
            Err(e) => {
                println!("   ❌ Failed to send chunk at {} bytes: {}", sent, e);
                return;
            }
        }
    }

    println!("   ✅ Public parameters sent successfully!");

    println!("4. Wait 20 seconds for worker preprocessing...");
    sleep(Duration::from_secs(20)).await;
    println!("   ✅ Preprocessing wait completed");

    println!("5. Generate query...");
    //let test_string = "cladoniaceae AND cladonia";
    let test_string = "cladoniaceae OR cladonia";
    //let test_string = "NOT cladoniaceae";
    //let test_string = "(cladoniaceae OR cladonia OR stereocaulaceae) AND podetia AND NOT (swabians OR danube) AND pycnothelia AND stellaris";
    //let test_string = "(cladoniaceae AND cladonia AND stereocaulaceae) AND NOT (swabians OR danube) AND podetia AND NOT banat OR pycnothelia";
    //let test_string = "(cladoniaceae AND NOT swabians) OR (cladonia AND stereocaulaceae AND NOT danube) OR (podetia AND pycnothelia AND stellaris)";
    let start_time = Instant::now();
    if mode == 0 {
        let (query, querysum, template, rank_vector) = client.qiyana_query(&test_string);
        let query_time = start_time.elapsed();
        println!("   Client submit the Boolean query: {}", test_string);
        println!("   Query generation time: {:?}", query_time);

        let query_vec = vec![
            bincode::serialize(&query).unwrap(),
            bincode::serialize(&querysum).unwrap(),
            bincode::serialize(&template).unwrap(),
            bincode::serialize(&rank_vector).unwrap(),
        ];
        
        let serialized_query = bincode::serialize(&query_vec).unwrap();
        let query_size = serialized_query.len();

        println!("6. Send query to Master...");
        let query_size_bytes = (query_size as u64).to_be_bytes();
        match master_stream.write_all(&query_size_bytes).await {
            Ok(()) => {}
            Err(e) => {
                println!("   ❌ Failed to send query size: {}", e);
                return;
            }
        }

        let query_chunk_size = 5 * 1024 * 1024;
        let mut query_sent = 0;
        let mut query_chunk_count = 0;
        let query_send_start = Instant::now();

        while query_sent < query_size {
            let remaining = query_size - query_sent;
            let current_chunk_size = query_chunk_size.min(remaining);
            let chunk = &serialized_query[query_sent..query_sent + current_chunk_size];
            
            match master_stream.write_all(chunk).await {
                Ok(()) => {
                    query_sent += current_chunk_size;
                    query_chunk_count += 1;
                }
                Err(e) => {
                    println!("   ❌ Failed to send query chunk at {} bytes: {}", query_sent, e);
                    return;
                }
            }
        }

        let query_send_time = query_send_start.elapsed();
        println!("   ✅ Query sent successfully!");
        println!("   📊 Query summary: {} bytes in {} chunks, took {:?}, {:.2} MB/s", 
            query_sent, query_chunk_count, query_send_time,
            (query_sent as f64 / 1024.0 / 1024.0) / query_send_time.as_secs_f64());

        drop(master_stream);
        println!("   ✅ Disconnected from Master");
        
        println!("7. Waiting for {} workers...", worker_num);
        receive_and_process_results(client, result_listener, worker_num, mode).await;
    } else {
        let (query, querysum, template, rank_vector) = client.qiyana_compress_query(&test_string);
        let query_time = start_time.elapsed();
        println!("   Client submit the Boolean query: {}", test_string);
        println!("   Query generation time: {:?}", query_time);

        let query_vec = vec![
            bincode::serialize(&query).unwrap(),
            bincode::serialize(&querysum).unwrap(),
            bincode::serialize(&template).unwrap(),
            bincode::serialize(&rank_vector).unwrap(),
        ];
        
        let serialized_query = bincode::serialize(&query_vec).unwrap();
        let query_size = serialized_query.len();

        println!("6. Send query to Master...");
        let query_size_bytes = (query_size as u64).to_be_bytes();
        match master_stream.write_all(&query_size_bytes).await {
            Ok(()) => {}
            Err(e) => {
                println!("   ❌ Failed to send query size: {}", e);
                return;
            }
        }

        let query_chunk_size = 5 * 1024 * 1024;
        let mut query_sent = 0;
        let mut query_chunk_count = 0;
        let query_send_start = Instant::now();

        while query_sent < query_size {
            let remaining = query_size - query_sent;
            let current_chunk_size = query_chunk_size.min(remaining);
            let chunk = &serialized_query[query_sent..query_sent + current_chunk_size];
            
            match master_stream.write_all(chunk).await {
                Ok(()) => {
                    query_sent += current_chunk_size;
                    query_chunk_count += 1;
                }
                Err(e) => {
                    println!("   ❌ Failed to send query chunk at {} bytes: {}", query_sent, e);
                    return;
                }
            }
        }

        let query_send_time = query_send_start.elapsed();
        println!("   ✅ Query sent successfully!");
        println!("   📊 Query summary: {} bytes in {} chunks, took {:?}, {:.2} MB/s", 
            query_sent, query_chunk_count, query_send_time,
            (query_sent as f64 / 1024.0 / 1024.0) / query_send_time.as_secs_f64());

        drop(master_stream);
        println!("   ✅ Disconnected from Master");
        
        println!("7. Waiting for {} workers...", worker_num);
        receive_and_process_results(client, result_listener, worker_num, mode).await;
    }
}

async fn receive_and_process_results(client: Client, listener: TcpListener, worker_num: usize, mode: usize) {
    println!("   📍 Listening on: {:?}", listener.local_addr());
    
    let start_time = Instant::now();
    enum ResponseValue {
        Lwe(Vec<Vec<LweCiphertextOwned<u64>>>),
        Glwe(Vec<GlweCiphertext<Vec<u64>>>),
    }

    let mut results: HashMap<usize, ResponseValue> = HashMap::new();

    let mut dots = 0;
    let status_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;
            print!(".");
            let _ = std::io::stdout().flush();
            dots += 1;
            
            if dots % 60 == 0 {
                let elapsed = start_time.elapsed();
                println!("\n   Still waiting... ({:?} elapsed)", elapsed);
            }
        }
    });
    
    println!("   ⏳ Waiting for {} workers to report results...", worker_num);
    loop {
        match timeout(Duration::from_secs(5), listener.accept()).await {
            Ok(Ok((mut stream, addr))) => {
                println!("\n   🔗 Got connection from {}", addr);
                let mut size_buf = [0u8; 8];
                if let Err(e) = stream.read_exact(&mut size_buf).await {
                    println!("   ❌ Failed to read response size: {}", e);
                    println!("   Wait time: {:?}", start_time.elapsed());
                    status_handle.abort();
                    return;
                }
                let expected_response_size = u64::from_be_bytes(size_buf) as usize;
                println!("   Expecting response: {} bytes", expected_response_size);

                if let Err(e) = stream.read_exact(&mut size_buf).await {
                    println!("   ❌ Failed to read response size: {}", e);
                    println!("   Wait time: {:?}", start_time.elapsed());
                    status_handle.abort();
                    return;
                }
                let worker_id = u64::from_be_bytes(size_buf) as usize;
                println!("   The response part from : Worker {}", worker_id);

                let mut all_response_data = Vec::with_capacity(expected_response_size);
                let mut received = 0;
                let chunk_size = 5 * 1024 * 1024;

                while received < expected_response_size {
                    let remaining = expected_response_size - received;
                    let current_chunk_size = chunk_size.min(remaining);
                    let mut chunk_buf = vec![0; current_chunk_size];
                    
                    if let Err(e) = stream.read_exact(&mut chunk_buf).await {
                        println!("   ❌ Failed to read response chunk at {} bytes: {}", received, e);
                        status_handle.abort();
                        return;
                    }
                    
                    all_response_data.extend_from_slice(&chunk_buf);
                    received += current_chunk_size;
                }

                if mode == 0 {
                    match bincode::deserialize::<Vec<Vec<LweCiphertextOwned<u64>>>>(&all_response_data) {
                        Ok(response) => {
                            results.insert(worker_id, ResponseValue::Lwe(response));
                        }
                        Err(e) => {
                            println!("❌ Failed to deserialize response from Worker {}: {}", worker_id, e);
                        }
                    }
                } else {
                    match bincode::deserialize::<Vec<GlweCiphertext<Vec<u64>>>>(&all_response_data) {
                        Ok(response) => {
                            results.insert(worker_id, ResponseValue::Glwe(response));
                        }
                        Err(e) => {
                            println!("❌ Failed to deserialize response from Worker {}: {}", worker_id, e);
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                println!("   ❌ Accept error: {}", e);
            }
            Err(_) => {
                continue;
            }
        }

        println!("   📋 Current results: {}/{}", results.len(), worker_num);
        if results.len() == worker_num {
            println!("   🎉 All workers reported!");
            status_handle.abort();
            break;
        }
    }
    println!("   The total latency : {:?}", start_time.elapsed());

    println!("8. Recover results...");
    let recover_start = Instant::now();
    let recovered = if mode == 0 {
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by_key(|(id, _)| *id);
        let lwe_data = sorted.into_iter()
            .flat_map(|(_, v)| match v {
                ResponseValue::Lwe(data) => data,
                _ => panic!("Unexpected response type"),
            })
            .collect();
        client.qiyana_recovery(lwe_data)
    } else {
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by_key(|(id, _)| *id);
        let glwe_data = sorted.into_iter()
            .flat_map(|(_, v)| match v {
                ResponseValue::Glwe(data) => data,
                _ => panic!("Unexpected response type"),
            })
            .collect();
        client.qiyana_packing_recovery(glwe_data, worker_num)
    };
    let recover_time = recover_start.elapsed();

    println!("   Recovery time: {:?}", recover_time);
    println!("   Number of results: {}", recovered.len());
            
    let mut valid = true;
    for i in 0..DOCUMENT_NUM {
        if i % 16384 == 0 {
            if recovered[i] != 222 {
                valid = false;
                println!("error {} line 222: {}", i, recovered[i]);
                break;
            }
        }
        else{
            if recovered[i] != 0 {
                valid = false;
                break;
            }
        }
        //NOT query
        //if i % 16384 == 0 { 
        //    if recovered[i] != 0 {
        //        valid = false;
        //        println!("error {} line 222: {}", i, recovered[i]);
        //        break;
        //    }
        //}
        //else{
            //if let Ok(tfidf_content) = std::fs::read_to_string("/root/Qiyana-experiment/tf-idf.txt") {
            //  let lines: Vec<&str> = tfidf_content.lines().collect();
            //  if let Some(comma_index) = lines[i % 16384 + 1].find(',') {
            //      let first_part = &lines[i % 16384 + 1][..comma_index];
            //      let sum: u16 = first_part
            //          .split(',')
            //          .filter_map(|s| s.parse::<u16>().ok())
            //          .take(32)
            //          .sum();
            //      if recovered[i] != sum {
            //          valid = false;
            //          break;
            //      }
            //  } else {
            //      valid = false;
            //      break;
            //  }
            //} else {
            //  valid = false;
            //}
        //}
    }
            
    if valid {
        println!("   ✅ Verification passed!");
    } else {
        println!("   ❌ Verification failed!");
    }
}
