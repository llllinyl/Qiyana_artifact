include!("qiyanawosel_sim.rs");
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::time::{timeout, sleep};
use std::io::*;

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("🔍 Qiyana-wosel Client");
    println!("========================================\n");
    
    let server_addr = "127.0.0.1:9999";
    //let test_string = "cladoniaceae AND cladonia";
    //let test_string = "cladoniaceae OR cladonia";
    let test_string = "NOT cladoniaceae";

    println!("1. Initialize TFHE Client...");
    let client = Client::new();
    
    println!("2. Connect to server: {}", server_addr);
    let connect_result = timeout(
        Duration::from_secs(10),
        TcpStream::connect(server_addr)
    ).await;
    
    let mut stream = match connect_result {
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

    println!("3. Send public parameters to server...");
    let params_vec = vec![
        bincode::serialize(&client.seeded_bsk).unwrap(),
        bincode::serialize(&client.seeded_ksk).unwrap(),
        bincode::serialize(&client.equal_lut).unwrap(),
        bincode::serialize(&client.and_lut).unwrap(),
        bincode::serialize(&client.or_lut).unwrap(),
        bincode::serialize(&client.one).unwrap(),
        bincode::serialize(&client.zero).unwrap(),
    ];

    let serialized_pp = bincode::serialize(&params_vec).unwrap();
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

    println!("   ✅ Public parameters sent successfully!");

    println!("4. Wait 10 seconds for server preprocessing...");
    sleep(Duration::from_secs(10)).await;
    println!("   ✅ Preprocessing wait completed");

    println!("5. Generate query...");
    let start_time = Instant::now();
    let (query, querysum, template) = client.qiyanawosel_query(&test_string);
    let query_time = start_time.elapsed();
    println!("   Query generation time: {:?}", query_time);

    let query_vec = vec![
        bincode::serialize(&query).unwrap(),
        bincode::serialize(&querysum).unwrap(),
        bincode::serialize(&template).unwrap(),
    ];
    
    let serialized_query = bincode::serialize(&query_vec).unwrap();
    let query_size = serialized_query.len();

    println!("6. Send query to server...");
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
    let mut query_chunk_count = 0;
    let query_send_start = Instant::now();

    while query_sent < query_size {
        let remaining = query_size - query_sent;
        let current_chunk_size = query_chunk_size.min(remaining);
        let chunk = &serialized_query[query_sent..query_sent + current_chunk_size];
        
        match stream.write_all(chunk).await {
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
    
    println!("7. Waiting for server response...");
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
    
    let wait_start = Instant::now();

    let mut size_buf = [0u8; 8];
    if let Err(e) = stream.read_exact(&mut size_buf).await {
        println!("   ❌ Failed to read response size: {}", e);
        println!("   Wait time: {:?}", wait_start.elapsed());
        status_handle.abort();
        return;
    }

    let expected_response_size = u64::from_be_bytes(size_buf) as usize;
    println!("   Expecting response: {} bytes", expected_response_size);

    let mut all_response_data = Vec::with_capacity(expected_response_size);
    let mut received = 0;
    let mut chunk_count = 0;
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
        chunk_count += 1;
    }

    status_handle.abort();
    println!();

    let total_time = start_time.elapsed();
    let wait_time = wait_start.elapsed();

    println!("   ✅ Response received!");
    println!("   Response size: {} bytes ({} chunks)", received, chunk_count);
    println!("   Wait time: {:?}", wait_time);
    println!("   Total time (query+wait): {:?}", total_time);

    println!("   Deserializing response...");
    match bincode::deserialize::<Vec<LweCiphertextOwned<u64>>>(&all_response_data) {
        Ok(response) => {
            println!("8. Recover results...");
            let recover_start = Instant::now();
            let recovered = client.qiyanawosel_recovery(response);
            let recover_time = recover_start.elapsed();
            
            println!("   Recovery time: {:?}", recover_time);
            println!("   Number of results: {}", recovered.len());
            
            let mut valid = true;
            if recovered[0] != false {
                valid = false;
            }
            for i in 1..DOCUMENT_NUM {
                if i < recovered.len() && !recovered[i] {
                    valid = false;
                    break;
                }
            }
            
            if valid {
                println!("   ✅ Verification passed!");
            } else {
                println!("   ❌ Verification failed!");
            }
            
            println!("\n📊 FINAL STATISTICS:");
            println!("   Query generation: {:?}", query_time);
            println!("   Server wait time: {:?}", wait_time);
            println!("   Result recovery: {:?}", recover_time);
            println!("   Total elapsed: {:?}", start_time.elapsed());
        }
        Err(e) => {
            println!("❌ Failed to deserialize response: {}", e);
        }
    }
    
    println!("\n🔶 Test completed!");
}
