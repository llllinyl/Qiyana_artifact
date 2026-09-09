#[path = "../utils/bloomfilter.rs"]
mod bloomfilter_mod;
use bloomfilter_mod::{BloomFilter, DEFAULT_SIZE};

use std::time::Instant;
use std::fs;
use std::path::{Path, PathBuf};
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheBool, FheUint2};
use tfhe::shortint::parameters::v1_5::*;
use tfhe::{ClientKey, ServerKey};
use serde::{Serialize, Deserialize};
use rayon::prelude::*;
use std::sync::Arc;

pub const DOCUMENT_NUM: usize = 16384;
pub const THREAD_NUM: usize = 64;
pub const KEYWORD_SET_NUM: usize = 16;

pub type EncryptedKeywordBloom = Vec<FheBool>;

fn build_bloom_filter<'a, I>(keywords: I) -> BloomFilter
where
    I: IntoIterator<Item = &'a str>,
{
    let filter = BloomFilter::new();

    for keyword in keywords {
        filter.insert(keyword);
    }

    filter
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

pub struct Client {
    client_key: ClientKey,
    pub server_key: ServerKey,
    pub false_ciphertext: FheBool,
}

impl Client {
    pub fn new() -> Self {
        let config = ConfigBuilder::default()
            .use_custom_parameters(V1_5_PARAM_MESSAGE_1_CARRY_2_KS_PBS_GAUSSIAN_2M128);
        let (client_key, server_key) = generate_keys(config);
        set_server_key(server_key.clone());
        let false_ciphertext = FheBool::encrypt(false, &client_key);

        Self {
            client_key,
            server_key,
            false_ciphertext,
        }
    }
    
    pub fn baselineb_query(&self, query_string: &str, bloom_size: usize) -> (Vec<EncryptedKeywordBloom>, Vec<QueryElement>) {
        assert_eq!(bloom_size, DEFAULT_SIZE as usize);
        let (strings, queryformat) = parse_query_to_rpn(query_string);

        let mut encrypted_results = Vec::with_capacity(strings.len());
        for string in strings {
            let bloom = build_bloom_filter(std::iter::once(string.as_str()));
            let encrypted_bloom = (0..DEFAULT_SIZE)
                .map(|index| {
                    let bit = bloom.get_bit(index).unwrap_or(false);
                    FheBool::encrypt(bit, &self.client_key)
                })
                .collect();
            encrypted_results.push(encrypted_bloom);
        }
        (encrypted_results, queryformat)
    }

    pub fn baselineb_recovery(
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

pub struct Server {
    pub keyword_sets: Vec<BloomFilter>,
    pub server_key: ServerKey,
    pub false_ciphertext: FheBool,
}

impl Server {
    pub fn new<P: AsRef<Path>>(server_key: ServerKey, false_ciphertext: FheBool, file_path: P) -> Self {
        let file_path = file_path.as_ref();
        let content = fs::read_to_string(file_path).unwrap_or_else(|error| {
            panic!("failed to read keyword file {}: {error}", file_path.display())
        });
        let mut keyword_sets = Vec::with_capacity(DOCUMENT_NUM);
        
        for (i, line) in content.lines().enumerate() {
            if i >= DOCUMENT_NUM {
                break;
            }

            let document_keywords = line
                .split(',')
                .map(str::trim)
                .filter(|keyword| !keyword.is_empty())
                .take(KEYWORD_SET_NUM);
            keyword_sets.push(build_bloom_filter(document_keywords));
        }
        
        while keyword_sets.len() < DOCUMENT_NUM {
            keyword_sets.push(build_bloom_filter(std::iter::empty::<&str>()));
        }
        Self {
            keyword_sets,
            server_key,
            false_ciphertext,
        }
    }

    pub fn baselineb_response(&self, query: Vec<EncryptedKeywordBloom>,
        query_structure: &[QueryElement]) -> Vec<FheBool> {
        
        let server_key = self.server_key.clone();
        let keyword_sets = self.keyword_sets.clone();
        let false_ciphertext = self.false_ciphertext.clone();
        
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(THREAD_NUM)
            .start_handler(move |_| {
                tfhe::set_server_key(server_key.clone());
            })
            .build()
            .expect("Failed to build thread pool");
        
        pool.install(|| {
            let subset_testing_start = Instant::now();
            let row_results_all: Vec<Vec<FheBool>> = keyword_sets
                .into_par_iter()
                .map(|document_bloom| {
                    let row_results: Vec<FheBool> = query
                        .iter()
                        .map(|query_bloom| {
                            assert_eq!(query_bloom.len(), DEFAULT_SIZE as usize);
                            let mut forbidden_bit_or: Option<FheBool> = None;

                            for index in 0..DEFAULT_SIZE {
                                if !document_bloom.get_bit(index).unwrap_or(false) {
                                    let query_bit = &query_bloom[index as usize];
                                    forbidden_bit_or = match forbidden_bit_or {
                                        Some(existing) => Some(&existing | query_bit),
                                        None => Some(query_bit.clone()),
                                    };
                                }
                            }

                            match forbidden_bit_or {
                                Some(forbidden_bit_or) => !forbidden_bit_or,
                                None => !false_ciphertext.clone(),
                            }
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
                    Self::evaluate_rpn_for_doc(
                        query_structure,
                        &row_results,
                        &false_ciphertext,
                    )
                })
                .collect();
            
            let boolean_match_time = boolean_match_start.elapsed();
            println!("Boolean Match time: {:?}", boolean_match_time);
            
            results
        })

    }
    pub fn evaluate_rpn_for_doc(
        structure: &[QueryElement],
        term_results: &[FheBool],
        false_ciphertext: &FheBool,
    ) -> FheBool {
        let mut stack: Vec<FheBool> = Vec::new();
        
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
        
        stack.pop().unwrap_or_else(|| false_ciphertext.clone())
    }
}
