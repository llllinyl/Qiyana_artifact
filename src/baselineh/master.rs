include!("share.rs");
use tokio::sync::RwLock;


struct MasterNode {
    worker_num: usize,
    workers: Arc<RwLock<HashMap<u32, TcpStream>>>,
}

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("🚀 Distributed Master");
    println!("========================================\n");
    
    let args: Vec<String> = std::env::args().collect();
    let worker_num = if args.len() > 1 {
        args[1].parse::<usize>().unwrap_or(0)
    } else {
        println!("Usage: <command> -- <WORKER_NUM>");
        println!("Example: <command> -- 1");
        return;
    };
    let mut master = MasterNode::new(worker_num);
    master.run().await;
}

impl MasterNode {
    fn new(worker_num: usize) -> Self {
        Self {
            worker_num,
            workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn run(&mut self) {
        println!("1. Waiting for {} workers to connect...", self.worker_num);
        self.wait_for_workers().await;
        
        println!("2. Waiting for client connection...");
        self.wait_for_client().await;
    }
    
    async fn wait_for_workers(&mut self) {
        let worker_listener = TcpListener::bind(format!("0.0.0.0:{}", MASTER_WORKER_PORT))
            .await
            .expect("Failed to bind worker listener");
        
        println!("   Worker listener on port {}", MASTER_WORKER_PORT);
        
        let workers = self.workers.clone();
        let worker_num = self.worker_num.clone();
        tokio::spawn(async move {
            let mut counter = 0;
            while counter < worker_num as u32 {
                match worker_listener.accept().await {
                    Ok((stream, addr)) => {
                        let worker_id = (addr.port() - WORKER_BEGIN_PORT) as u32;
                        println!("   ✅ Worker {} connected from {}", worker_id, addr);
                        
                        let mut workers_lock = workers.write().await;
                        workers_lock.insert(worker_id, stream);
                        
                        counter += 1;
                    }
                    Err(e) => {
                        println!("   ❌ Error accepting worker: {}", e);
                    }
                }
            }
            
            println!("   ✅ All {} workers connected", worker_num);
        });
        
        while self.workers.read().await.len() < worker_num {
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    async fn wait_for_client(&mut self) {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", MASTER_PORT))
            .await
            .expect("Failed to bind master listener");
        
        println!("   Master listening on port {}", MASTER_PORT);
        
        if let Ok((stream, addr)) = listener.accept().await {
            println!("   ✅ Client connected from {}", addr);
            self.handle_client(stream, addr.to_string()).await;
        }
    }
    
    async fn handle_client(&mut self, mut stream: TcpStream, client_addr: String) {
        println!("3. Waiting for public parameters from Client...");
        
        let mut size_buf = [0u8; 8];
        if let Err(e) = stream.read_exact(&mut size_buf).await {
            println!("[{}] ❌ Failed to read size header: {}", client_addr, e);
            return;
        }

        let expected_size = u64::from_be_bytes(size_buf) as usize;
        println!("[{}]   Expecting {} bytes total", client_addr, expected_size);

        let mut all_data = Vec::with_capacity(expected_size);
        let mut received = 0;
        let read_start = Instant::now();

        while received < expected_size {
            let mut buffer = vec![0u8; 5 * 1024 * 1024];
            let bytes_to_read = std::cmp::min(buffer.len(), expected_size - received);
            buffer.truncate(bytes_to_read);
            
            match stream.read_exact(&mut buffer).await {
                Ok(_) => {
                    received += buffer.len();
                    all_data.extend_from_slice(&buffer);
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to read data chunk: {}", 
                            client_addr, e);
                    return;
                }
            }
        }

        let read_time = read_start.elapsed();
        println!("[{}]   ✅ Received all bytes of public parameters, took {:?}", 
                client_addr, read_time);

        match bincode::deserialize::<Vec<Vec<u8>>>(&all_data) {
            Ok(params_pack) => {
                println!("[{}]   ✅ Public parameters package deserialized, {} params", 
                        client_addr, params_pack.len());
                
                if params_pack.len() != 2 {
                    println!("[{}] ❌ Expected 2 parameters, got {}", client_addr, params_pack.len());
                    return;
                }

                println!("4. Distribute public parameters to Workers...");

                self.distribute_public_parameters_to_workers(params_pack.clone()).await;

                println!("[{}] 5. Waiting for query from Client...", client_addr);
                let mut query_size_buf = [0u8; 8];
                if let Err(e) = stream.read_exact(&mut query_size_buf).await {
                    println!("[{}] ❌ Failed to read query size header: {}", client_addr, e);
                    return;
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
                    
                    if let Err(e) = stream.read_exact(&mut chunk_buf).await {
                        println!("[{}] ❌ Failed to read query chunk at {} bytes: {}", 
                                client_addr, query_received, e);
                        return;
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
                        
                        if query_vec.len() != 2 {
                            println!("[{}] ❌ Expected 2 query element, got {}", client_addr, params_pack.len());
                            return;
                        }
                        println!("6. Distribute query to Workers...");

                        self.distribute_query_to_workers(query_vec.clone()).await;
                    }
                    Err(e) => {
                        println!("[{}] ❌ Failed to deserialize query: {}", client_addr, e);
                    }
                }
            }
            Err(e) => {
                println!("[{}] ❌ Failed to deserialize public parameters: {}", client_addr, e);
            }
        }
    }

    async fn distribute_public_parameters_to_workers(&self, params_pack: Vec<Vec<u8>>) {
        let mut workers = self.workers.write().await;
        
        if workers.is_empty() {
            println!("   ⚠️ No workers available");
            return;
        }
        
        println!("   📤 Distributing public parameters to {} workers...", workers.len());

        for (&worker_id, stream) in workers.iter_mut() {
            println!("   Sending public parameters to Worker {}...", worker_id);
            
            let serialized_pp = bincode::serialize(&params_pack).unwrap();
            let pp_size = serialized_pp.len();
            println!("   Public parameters size: {} bytes", pp_size);

            let size_bytes = (pp_size as u64).to_be_bytes();
            if let Err(e) = stream.write_all(&size_bytes).await {
                println!("   ❌ Failed to send size header: {}", e);
                return;
            }

            let chunk_size = 5 * 1024 * 1024;
            let mut sent = 0;

            while sent < pp_size {
                let remaining = pp_size - sent;
                let current_chunk_size = chunk_size.min(remaining);
                let chunk = &serialized_pp[sent..sent + current_chunk_size];
                
                match stream.write_all(chunk).await {
                    Ok(()) => {
                        sent += current_chunk_size;
                    }
                    Err(e) => {
                        println!("   ❌ Failed to send chunk at {} bytes: {}", sent, e);
                        return;
                    }
                }
            }
        }
        
        println!("   📊 Sent public parameters to workers successfully");
    }
    
    async fn distribute_query_to_workers(&self, query_pack: Vec<Vec<u8>>) {
        let mut workers = self.workers.write().await;
        
        if workers.is_empty() {
            println!("   ⚠️ No workers available");
            return;
        }
        
        println!("   📤 Distributing query to {} workers...", workers.len());
        
        for (&worker_id, stream) in workers.iter_mut() {
            println!("   Sending query to Worker {}...", worker_id);

            match stream.peer_addr() {
                Ok(addr) => println!("   Worker {} connection to {} is active", worker_id, addr),
                Err(e) => {
                    println!("   Worker {} connection is invalid: {}", worker_id, e);
                    return;
                }
            }
            
            let serialized_query = bincode::serialize(&query_pack).unwrap();
            let query_size = serialized_query.len();
            let query_size_bytes = (query_size as u64).to_be_bytes();
            match stream.write_all(&query_size_bytes).await {
                Ok(()) => {}
                Err(e) => {
                    println!("   ❌ Failed to send query size: {}", e);
                    return;
                }
            }

            let query_chunk_size = 5 * 1024 * 1024;
            let mut query_sent = 0;

            while query_sent < query_size {
                let remaining = query_size - query_sent;
                let current_chunk_size = query_chunk_size.min(remaining);
                let chunk = &serialized_query[query_sent..query_sent + current_chunk_size];
                
                match stream.write_all(chunk).await {
                    Ok(()) => {
                        query_sent += current_chunk_size;
                    }
                    Err(e) => {
                        println!("   ❌ Failed to send query chunk at {} bytes: {}", query_sent, e);
                        return;
                    }
                }
            }
        }
        
        println!("   📊 Sent query to workers successfully");
    }
}
