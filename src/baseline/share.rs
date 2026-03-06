use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::io::Write;

use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheAsciiString, FheBool, ClearString};
use tfhe::shortint::parameters::v1_5::*;
use tfhe::{ClientKey, ServerKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};
use rayon::prelude::*;
use serde::{Serialize, Deserialize};

pub const MASTER_PORT: u16 = 9999;
pub const MASTER_WORKER_PORT: u16 = 10000;  
pub const CLIENT_RESULT_PORT: u16 = 10001; 
pub const WORKER_BEGIN_PORT: u16 = 8000;

pub const THREAD_NUM: usize = 16;
pub const DOCUMENT_NUM: usize = 1024; //2^k
pub const KEYWORD_SET_NUM: usize = 16;

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

pub struct Client {
    client_key: ClientKey,
    pub server_key: ServerKey,
    pub false_ciphertext: FheBool,
}

impl Client {
    pub fn new() -> Self {
        let config = ConfigBuilder::default()
            .use_custom_parameters(V1_5_PARAM_MESSAGE_2_CARRY_2_KS_PBS_GAUSSIAN_2M128);
        let (client_key, server_key) = generate_keys(config);
        set_server_key(server_key.clone());
        let false_ciphertext = FheBool::encrypt(false, &client_key);

        Self {
            client_key,
            server_key,
            false_ciphertext,
        }
    }
    
    pub fn baseline_query(&self, query_string: &str, length: usize) -> (Vec<FheAsciiString>, Vec<QueryElement>) {
        let (strings, queryformat) = parse_query_to_rpn(query_string);

        let mut encrypted_results = Vec::with_capacity(strings.len());
        for string in strings {
            //avoid to reveal the length
            let encrypted_string = FheAsciiString::try_encrypt_with_fixed_sized(&string, length, &self.client_key).unwrap();
            encrypted_results.push(encrypted_string);
        }
        (encrypted_results, queryformat)
    }

    pub fn baseline_recovery(
        &self,
        encrypted_results: Vec<FheBool>,
    ) -> Vec<bool> {
        let mut final_results = Vec::with_capacity(DOCUMENT_NUM);
        
        for bool_cipher in encrypted_results {
            let decrypted = bool_cipher.decrypt(&self.client_key);
            final_results.push(decrypted);
        }

        final_results
    }
}

pub struct SubServer {
    pub keywords: Vec<[String; KEYWORD_SET_NUM]>,
    pub server_key: ServerKey,
    pub false_ciphertext: FheBool,
}

impl SubServer {
    pub fn new<P: AsRef<Path>>(worker_num: usize, worker_id: u32, server_key: ServerKey, false_ciphertext: FheBool, file_path: P) -> Self {
        let content = fs::read_to_string(file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let actual_file_lines = lines.len();

        let documents_per_worker = DOCUMENT_NUM / worker_num;
        let start_doc = worker_id as usize * documents_per_worker;
        let end_doc = start_doc + documents_per_worker;
        let mut keywords = Vec::with_capacity(documents_per_worker);

        for doc_idx in start_doc..end_doc {
            let line_idx = doc_idx % actual_file_lines;
            let line = lines[line_idx];
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
        println!("[Worker {}]: Successfully loaded {} keyword sets", worker_id, keywords.len());
        
        Self {
            keywords,
            server_key,
            false_ciphertext,
        }
    }

    pub fn baseline_response(&self, query: Vec<FheAsciiString>,
        query_structure: &[QueryElement]) -> Vec<FheBool> {
        
        let server_key = self.server_key.clone();
        let keywords = self.keywords.clone();
        
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(THREAD_NUM)
            .start_handler(move |_| {
                tfhe::set_server_key(server_key.clone());
            })
            .build()
            .expect("Failed to build thread pool");
        pool.install(|| {
            let subset_testing_start = Instant::now();
            let precomputed_keywords: Vec<Vec<ClearString>> = keywords
                .iter()
                .map(|keyword_row| {
                    keyword_row.iter()
                        .map(|kc| ClearString::new(kc.clone()))
                        .collect()
                })
                .collect();
            
            let row_results_all: Vec<Vec<FheBool>> = precomputed_keywords
                .par_iter()
                .zip(&keywords)
                .map(|(precomputed_row, _keyword_row)| {
                    let row_results: Vec<FheBool> = query
                        .iter()
                        .map(|query_cipher| {
                            let mut or_result: Option<FheBool> = None;
                            
                            for precomputed in precomputed_row {
                                let is_equal = query_cipher.eq(precomputed);
                                
                                or_result = match or_result {
                                    Some(existing) => Some(existing | &is_equal),
                                    None => Some(is_equal),
                                };
                            }
                            
                            or_result.expect("OR result should exist")
                        })
                        .collect();
                    
                    row_results
                })
                .collect();
            
            let subset_testing_time = subset_testing_start.elapsed();
            println!("Homomorphic Subset Testing time: {:?}", subset_testing_time);
            
            let boolean_match_start = Instant::now();
            
            let results: Vec<FheBool> = row_results_all
                .into_par_iter()
                .map(|row_results| {
                    self.evaluate_rpn_for_doc(query_structure, &row_results)
                })
                .collect();
            
            let boolean_match_time = boolean_match_start.elapsed();
            println!("Boolean Match time: {:?}", boolean_match_time);
            
            results
        })
    }

    pub fn evaluate_rpn_for_doc(&self, structure: &[QueryElement], term_results: &[FheBool]) -> FheBool {
        let mut stack: Vec<FheBool> = Vec::new();
        let false_ciphertext = self.false_ciphertext.clone();

        for element in structure {
            match element {
                QueryElement::Term(idx) => {
                    let result = if *idx < term_results.len() {
                        term_results[*idx].clone()
                    } else {
                        false_ciphertext.clone()
                    };
                    stack.push(result);
                }
                QueryElement::Not => {
                    if let Some(operand) = stack.pop() {
                        stack.push(!operand);
                    } else {
                        stack.push(false_ciphertext.clone());
                    }
                }
                QueryElement::And => {
                    if stack.len() >= 2 {
                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                        stack.push(&left & &right);
                    } else {
                        stack.push(false_ciphertext.clone());
                    }
                }
                QueryElement::Or => {
                    if stack.len() >= 2 {
                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                        stack.push(&left | &right);
                    } else {
                        stack.push(false_ciphertext.clone());
                    }
                }
                _ => {
                }
            }
        }

        stack.pop().unwrap_or(false_ciphertext.clone())
    }
}
