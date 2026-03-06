use serde::{Serialize, Deserialize};
use std::error::Error;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use std::io::*;
use std::time::Instant;
use std::fs;
use std::fs::File;
use std::path::Path;
use rayon::prelude::*;
use std::sync::Arc;

pub const DOCUMENT_NUM: usize = 16384;
pub const KEYWORD_NUM: usize = 65536;
pub const THREAD_NUM: usize = 64;
pub const KEYWORD_SET_NUM: usize = 16;

pub fn read_tf_idf_file(
    file_path: &str,
    max_docs: usize,
) -> Vec<Vec<u16>> {
    let file = match File::open(Path::new(file_path)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open file {}: {}", file_path, e);
            return Vec::new();
        }
    };
    
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    match lines.next() {
        Some(Ok(_keywords_line)) => {}
        Some(Err(e)) => {
            eprintln!("Failed to read keywords line: {}", e);
            return Vec::new();
        }
        None => {
            eprintln!("File is empty");
            return Vec::new();
        }
    };
    
    let mut tf_idf_matrix = Vec::new();
    let mut doc_count = 0;
    
    for line in lines {
        if doc_count >= max_docs {
            break;
        }
        
        match line {
            Ok(line_content) => {
                let tf_idf_values: Vec<u16> = line_content
                    .split(',')
                    .filter_map(|s| s.parse::<u16>().ok())
                    .filter(|&x| x <= 1023)
                    .collect();
                
                tf_idf_matrix.push(tf_idf_values);
                doc_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to read document line: {}", e);
                break;
            }
        }
    }
    tf_idf_matrix
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryElement {
    Term(usize), 
    And,
    Or,
    Not, 
    LeftParen,
}

impl QueryElement {
    fn precedence(&self) -> u8 {
        match self {
            QueryElement::Not => 3,
            QueryElement::And => 2,
            QueryElement::Or => 1,
            _ => 0,
        }
    }
}


pub fn tokenize_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = query.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '(' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                }
                tokens.push("(".to_string());
            }
            ')' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                }
                tokens.push(")".to_string());
            }
            ' ' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                }
            }
            _ => {
                current_token.push(c);
                if matches!(c.to_ascii_uppercase(), 'A' | 'O' | 'N') {
                    if let Some(next_char) = chars.peek() {
                        if next_char.is_whitespace() || matches!(next_char, '(' | ')') {
                        }
                    }
                }
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token.trim().to_string());
    }

    tokens
}

pub fn parse_query_to_rpn(query_string: &str) -> (Vec<String>, Vec<QueryElement>) {
    let tokens = tokenize_query(query_string);
    let mut keywords = Vec::new();
    let mut output = Vec::new();
    let mut operator_stack = Vec::new();
    let mut term_counter = 0;

    for token in tokens {
        match token.as_str() {
            "(" => {
                operator_stack.push(QueryElement::LeftParen);
            }
            ")" => {
                while let Some(op) = operator_stack.pop() {
                    if let QueryElement::LeftParen = op {
                        break;
                    }
                    output.push(op);
                }
            }
            "AND" => {
                while let Some(top) = operator_stack.last() {
                    if let QueryElement::LeftParen = top {
                        break;
                    }
                    if QueryElement::And.precedence() <= top.precedence() {
                        output.push(operator_stack.pop().unwrap());
                    } else {
                        break;
                    }
                }
                operator_stack.push(QueryElement::And);
            }
            "OR" => {
                while let Some(top) = operator_stack.last() {
                    if let QueryElement::LeftParen = top {
                        break;
                    }
                    if QueryElement::Or.precedence() <= top.precedence() {
                        output.push(operator_stack.pop().unwrap());
                    } else {
                        break;
                    }
                }
                operator_stack.push(QueryElement::Or);
            }
            "NOT" => {
                operator_stack.push(QueryElement::Not);
            }
            term => {
                keywords.push(term.to_string());
                output.push(QueryElement::Term(term_counter));
                term_counter += 1;
            }
        }
    }

    while let Some(op) = operator_stack.pop() {
        output.push(op);
    }

    (keywords, output)
}

pub fn evaluate_rpn_for_doc_plain(structure: &[QueryElement], term_results: &[bool]) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    
    for element in structure {
        match element {
            QueryElement::Term(idx) => {
                let result = if *idx < term_results.len() {
                    term_results[*idx]
                } else {
                    false
                };
                stack.push(result);
            }
            QueryElement::Not => {
                if let Some(operand) = stack.pop() {
                    stack.push(!operand);
                } else {
                    stack.push(false);
                }
            }
            QueryElement::And => {
                if stack.len() >= 2 {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(left && right);
                } else {
                    stack.push(false);
                }
            }
            QueryElement::Or => {
                if stack.len() >= 2 {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(left || right);
                } else {
                    stack.push(false);
                }
            }
            _ => {}
        }
    }
    
    stack.pop().unwrap_or(false)
}

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
    println!("[Server] 1. Extracting data...");
    let content = fs::read_to_string(keyword_file).unwrap(); 
    let mut keywords = Vec::with_capacity(DOCUMENT_NUM);

    for (i, line) in content.lines().enumerate() {
        if i >= DOCUMENT_NUM {
            break;
        }
        
        let mut keys = [const{String::new()}; KEYWORD_SET_NUM];
        let mut index = 0;
        
        for part in line.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() && index < KEYWORD_SET_NUM {
                keys[index] = trimmed.to_string();
                index += 1;
            }
        }
            
        keywords.push(keys);
    }
    let matrix = read_tf_idf_file(tfidf_path, DOCUMENT_NUM);

    println!("[{}] 2. Receiving Query...", client_addr);

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
            println!("[{}]   ✅ Query package deserialized, {} params", 
                    client_addr, params_pack.len());
            
            if params_pack.len() != 2 {
                println!("[{}] ❌ Expected 2 parameters, got {}", client_addr, params_pack.len());
                return Ok(());
            }

            let deserialize_start = Instant::now();
            
            let test_string: String = match bincode::deserialize(&params_pack[0]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize test_string: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let rank_vector: Vec<u64> = match bincode::deserialize(&params_pack[1]) {
                Ok(param) => {
                    param
                }
                Err(e) => {
                    println!("[{}] ❌ Failed to deserialize rank_vector: {}", client_addr, e);
                    return Ok(());
                }
            };
            
            let deserialize_time = deserialize_start.elapsed();
            println!("[{}]   ✅ Query deserialized in {:?}", 
                    client_addr, deserialize_time);

            rayon::ThreadPoolBuilder::new()
                .num_threads(THREAD_NUM)
                .build_global()
                .unwrap();  
            let process_start = Instant::now();
            let (strings, queryformat) = parse_query_to_rpn(&test_string);

            let result: Vec<u64> = matrix.iter()
                .zip(keywords.iter())
                .par_bridge() 
                .map(|(doc_row, doc_keywords)| {
                    let sum: u64 = doc_row.iter()
                        .zip(rank_vector.iter())
                        .map(|(&matrix_val, &rank_val)| (matrix_val as u64) * rank_val)
                        .sum();
                    
                    let row_bool: Vec<bool> = strings.iter()
                        .map(|query_string| doc_keywords.iter().any(|k| k == query_string))
                        .collect();

                    let bool_result = evaluate_rpn_for_doc_plain(&queryformat, &row_bool);
                    
                    if bool_result {
                        sum
                    } else {
                        0u64
                    }
                })
                .collect();
            let process_time = process_start.elapsed();

            println!("[{}] 3. Sending response...", client_addr);
            let response_start = Instant::now();
            let response_bytes = bincode::serialize(&result).unwrap();
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
                        return Ok(());
                    }
                }
            }

            let send_time = response_start.elapsed();
            println!("[{}]   ✅ Response sent in {:?} ({} chunks, {:.2} MB/s)", 
                    client_addr, send_time, chunk_count,
                    (response_size as f64 / 1024.0 / 1024.0) / send_time.as_secs_f64());

            let total_time = read_start.elapsed();
            println!("[{}] 📊 Session statistics:", client_addr);
            println!("[{}]   - Receive query: {:?}", client_addr, read_time);
            println!("[{}]   - Process query: {:?}", client_addr, process_time);
            println!("[{}]   - Send response: {:?}", client_addr, send_time);
            println!("[{}]   - TOTAL TIME: {:?}", client_addr, total_time);
            println!("[{}] 🎉 Session completed successfully!", client_addr);
        }
        Err(e) => {
            println!("[{}] ❌ Failed to deserialize public parameters: {}", client_addr, e);
        }
    }

    sleep(Duration::from_secs(1)).await;
    
    println!("[{}] Connection closed", client_addr);
    Ok(())
}
