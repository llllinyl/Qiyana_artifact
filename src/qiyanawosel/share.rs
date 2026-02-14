use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::io::Write;

use tfhe::core_crypto::prelude::*;
use tfhe::shortint::parameters::DynamicDistribution;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};
use rayon::prelude::*;

#[path = "../utils/decomposition.rs"]
mod decomposition_mod;
#[path = "../utils/bloomfilter.rs"] 
mod bloomfilter_mod;
use bloomfilter_mod::BloomFilter;
use decomposition_mod::*;

pub const MASTER_PORT: u16 = 9999;
pub const MASTER_WORKER_PORT: u16 = 10000;  
pub const CLIENT_RESULT_PORT: u16 = 10001; 
pub const WORKER_BEGIN_PORT: u16 = 8000;

pub const THREAD_NUM: usize = 16;
pub const DOCUMENT_NUM: usize = 16; //2^k
pub const KEYWORD_SET_NUM: usize = 8;

#[derive(Debug, Clone, PartialEq)]
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
    small_lwe_sk: LweSecretKey<Vec<u64>>,
    lwe_noise_distribution: DynamicDistribution<u64>,
    pub seeded_bsk: SeededLweBootstrapKey<Vec<u64>>,
    pub seeded_ksk: SeededLweKeyswitchKey<Vec<u64>>,
    pub equal_lut: GlweCiphertextOwned<u64>,
    pub and_lut: GlweCiphertextOwned<u64>,
    pub or_lut: GlweCiphertextOwned<u64>,
    pub one: LweCiphertextOwned<u64>,
    pub zero: LweCiphertextOwned<u64>,
}

impl Client {
    pub fn new() -> Self {
        let small_lwe_dimension = LweDimension(1055);
        let glwe_dimension = GlweDimension(1);
        let polynomial_size = PolynomialSize(16384);
        let lwe_noise_distribution =
            DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));//512 ciphertext sum; real 2^{-40}
        let glwe_noise_distribution =
            DynamicDistribution::new_gaussian_from_std_dev(StandardDev(2.168404344971009e-19));
        let pbs_base_log = DecompositionBaseLog(15);
        let pbs_level = DecompositionLevelCount(2);
        let ks_decomp_base_log = DecompositionBaseLog(2);
        let ks_decomp_level_count = DecompositionLevelCount(11);
        let ciphertext_modulus = CiphertextModulus::new_native();
        let message_modulus = 1u64 << 7;
        let delta = (1_u64 << 63) / message_modulus;
        let mut boxed_seeder = new_seeder();
        let seeder = boxed_seeder.as_mut();

        let mut secret_generator =
            SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());
        let mut encryption_generator =
            EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);

        let small_lwe_sk =
            LweSecretKey::generate_new_binary(small_lwe_dimension, &mut secret_generator);
        let glwe_sk =
            GlweSecretKey::generate_new_binary(glwe_dimension, polynomial_size, &mut secret_generator);
        let big_lwe_sk = glwe_sk.clone().into_lwe_secret_key();

        // let std_bootstrapping_key = par_allocate_and_generate_new_lwe_bootstrap_key(
        //     &small_lwe_sk,
        //     &glwe_sk,
        //     pbs_base_log,
        //     pbs_level,
        //     glwe_noise_distribution,
        //     ciphertext_modulus,
        //     &mut encryption_generator,
        // );
        // let mut fourier_bsk = FourierLweBootstrapKey::new(
        //     std_bootstrapping_key.input_lwe_dimension(),
        //     std_bootstrapping_key.glwe_size(),
        //     std_bootstrapping_key.polynomial_size(),
        //     std_bootstrapping_key.decomposition_base_log(),
        //     std_bootstrapping_key.decomposition_level_count(),
        // );
        // convert_standard_lwe_bootstrap_key_to_fourier(&std_bootstrapping_key, &mut fourier_bsk);
        // drop(std_bootstrapping_key);
        let seeded_bsk = par_allocate_and_generate_new_seeded_lwe_bootstrap_key(
            &small_lwe_sk,
            &glwe_sk,
            pbs_base_log,
            pbs_level,
            glwe_noise_distribution,
            ciphertext_modulus,
            seeder,
        );

        // let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
        //     &big_lwe_sk,
        //     &small_lwe_sk,
        //     ks_decomp_base_log,
        //     ks_decomp_level_count,
        //     lwe_noise_distribution,
        //     ciphertext_modulus,
        //     &mut encryption_generator,
        // );
        let seeded_ksk = allocate_and_generate_new_seeded_lwe_keyswitch_key(
            &big_lwe_sk,
            &small_lwe_sk,
            ks_decomp_base_log,
            ks_decomp_level_count,
            lwe_noise_distribution,
            ciphertext_modulus,
            seeder,
        );

        let equal_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
            polynomial_size,
            glwe_dimension.to_glwe_size(),
            message_modulus as usize,
            ciphertext_modulus,
            delta,
            |x: u64| {if x == 0 { 1 } else { 0 }},
        );
        let and_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
            polynomial_size,
            glwe_dimension.to_glwe_size(),
            message_modulus as usize,
            ciphertext_modulus,
            delta,
            |x: u64| {if x == 2 { 1 } else { 0 }},
        );
        let or_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
            polynomial_size,
            glwe_dimension.to_glwe_size(),
            message_modulus as usize,
            ciphertext_modulus,
            delta,
            |x: u64| {if x == 0 { 0 } else { 1 }},
        );
        let one: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
            &small_lwe_sk,
            Plaintext(1u64 * delta),
            lwe_noise_distribution,
            ciphertext_modulus,
            &mut encryption_generator,
        );
        let zero: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
            &big_lwe_sk,
            Plaintext(0u64),
            lwe_noise_distribution,
            ciphertext_modulus,
            &mut encryption_generator,
        );

        Self {
            small_lwe_sk,
            lwe_noise_distribution,
            seeded_bsk,
            seeded_ksk,
            equal_lut,
            and_lut,
            or_lut,
            one,
            zero,
        }
    }
    
    pub fn qiyanawosel_query(&self, strings: &str)
         -> (Vec<Vec<LweCiphertextOwned<u64>>>, Vec<LweCiphertextOwned<u64>>, String) {
        let message_modulus = 1u64 << 7;
        let delta = (1_u64 << 63) / message_modulus;
        let lwe_noise_distribution = self.lwe_noise_distribution;
        let ciphertext_modulus = CiphertextModulus::new_native();
        let mut boxed_seeder = new_seeder();
        let seeder = boxed_seeder.as_mut();
        let mut encryption_generator =
            EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);
        let mut query = Vec::new();
        let mut querysum = Vec::new();
        let mut template: String = "".to_string();

        match decompose_query(strings) {
            Ok(decomp) => {
                for (_block_idx, block) in decomp.blocks.iter().enumerate() {
                    let filter = BloomFilter::new();
                    let length = filter.size.get();
                    let mut queryblock = Vec::with_capacity(length as usize);
                    for string in block.iter() {
                        filter.insert(string);
                    }
                    for len in 0..length {
                        let ind = filter.get_bit(len).unwrap() as u64;
                        let plaintext = Plaintext(ind * delta);
                        let lwe_ciphertext_in: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
                            &self.small_lwe_sk,
                            plaintext,
                            lwe_noise_distribution,
                            ciphertext_modulus,
                            &mut encryption_generator,
                        );
                        queryblock.push(lwe_ciphertext_in);
                    }
                    let onecount = filter.get_all_set_bits_indices().len() as u64;
                    let queryblocksum: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
                        &self.small_lwe_sk,
                        Plaintext(onecount * delta),
                        lwe_noise_distribution,
                        ciphertext_modulus,
                        &mut encryption_generator,
                    );

                    query.push(queryblock);
                    querysum.push(queryblocksum);
                }
                template = decomp.template.clone();
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
        (query, querysum, template)
    }

    pub fn qiyanawosel_recovery(&self, result: Vec<LweCiphertextOwned<u64>>) -> Vec<bool> {
        let number = result.len();
        let message_modulus = 1u64 << 7;
        let delta = (1_u64 << 63) / message_modulus;
        let small_sk = self.small_lwe_sk.clone();
        let mut bool_results = Vec::with_capacity(number);
        for (_row_idx, rowres) in result.into_iter().enumerate() {
            let plain: Plaintext<u64> =
                decrypt_lwe_ciphertext(&small_sk, &rowres);
            let signed_decomposer =
                SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));
            let plain_result: u64 =
                signed_decomposer.closest_representable(plain.0) / delta;
            let boolres: bool = if plain_result == 1 {
                true
            } else if plain_result == 0 {
                false
            } else {
                panic!("plain_result should be 0 or 1, but got {}", plain_result);
            };
            bool_results.push(boolres);
        }
        bool_results
    }
}

pub struct SubServer {
    pub keywords: Vec<Vec<u16>>,
    pub fourier_bsk: FourierLweBootstrapKeyOwned,
    pub ksk_big_to_small: LweKeyswitchKeyOwned<u64>,
    pub equal_lut: GlweCiphertextOwned<u64>,
    pub and_lut: GlweCiphertextOwned<u64>,
    pub or_lut: GlweCiphertextOwned<u64>,
    pub one: LweCiphertextOwned<u64>,
    pub zero: LweCiphertextOwned<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum TemplateNode {
    Operand(usize, usize),
    And(Box<TemplateNode>, Box<TemplateNode>),
    Or(Box<TemplateNode>, Box<TemplateNode>),
    Not(Box<TemplateNode>),
}

impl SubServer {
    pub fn new<P: AsRef<Path>>(worker_num: usize, worker_id: u32, seeded_bsk: SeededLweBootstrapKey<Vec<u64>>, 
        seeded_ksk: SeededLweKeyswitchKey<Vec<u64>>, equal_lut: GlweCiphertextOwned<u64>,
        and_lut: GlweCiphertextOwned<u64>, or_lut: GlweCiphertextOwned<u64>, 
        one: LweCiphertextOwned<u64>, zero: LweCiphertextOwned<u64>, file_path: P) -> Self {
        let content = fs::read_to_string(file_path).unwrap(); 
        let documents_per_worker = DOCUMENT_NUM / worker_num;
        let start_doc = worker_id as usize * documents_per_worker;
        let mut keywords = Vec::with_capacity(documents_per_worker);
        
        for (_i, line) in content.lines().enumerate().skip(start_doc).take(documents_per_worker) {
            let filter = BloomFilter::new();
            let mut index = 0;

            for part in line.split(',') {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    continue;
                }
                filter.insert(&trimmed.to_string());
                index += 1;
                if index >= KEYWORD_SET_NUM {
                    break;
                }
            }
            let indices = filter.get_all_set_bits_indices();
            keywords.push(indices);
        }
        println!("[Worker {}]: Successfully loaded {} keyword sets", worker_id, keywords.len());

        let std_bootstrapping_key = seeded_bsk.decompress_into_lwe_bootstrap_key();
        let mut fourier_bsk = FourierLweBootstrapKey::new(
            std_bootstrapping_key.input_lwe_dimension(),
            std_bootstrapping_key.glwe_size(),
            std_bootstrapping_key.polynomial_size(),
            std_bootstrapping_key.decomposition_base_log(),
            std_bootstrapping_key.decomposition_level_count(),
        );
        convert_standard_lwe_bootstrap_key_to_fourier(&std_bootstrapping_key, &mut fourier_bsk);
        drop(std_bootstrapping_key);
        let ksk_big_to_small = seeded_ksk.decompress_into_lwe_keyswitch_key();

        Self {
            keywords,
            fourier_bsk,
            ksk_big_to_small,
            equal_lut,
            and_lut,
            or_lut,
            one,
            zero,
        }
    }

    pub fn qiyanawosel_response(&self, query: Vec<Vec<LweCiphertextOwned<u64>>>,
        querysum: Vec<LweCiphertextOwned<u64>>, 
        template: String) -> Vec<LweCiphertextOwned<u64>> {
        
        let keywords = self.keywords.clone();
        let (_strings, queryformat) = parse_query_to_rpn(&template);
        
        let final_results: Vec<LweCiphertextOwned<u64>> = keywords
            .par_iter()
            .map(|keyrow| {
                let mut row_results = Vec::with_capacity(query.len());
                
                for (quid, qu) in query.iter().enumerate() {
                    let mut sum = qu[keyrow[0] as usize].clone();
                    let length = keyrow.len();
                    
                    for ind in 1..length {
                        lwe_ciphertext_add_assign(&mut sum, &qu[keyrow[ind] as usize]);
                    }
                    
                    let equal_res = self.equal(querysum[quid].clone(), sum);
                    row_results.push(equal_res);
                }
                
                self.evaluate_rpn_for_doc(&queryformat, &row_results)
            })
            .collect();
        
        final_results
    }

    pub fn evaluate_rpn_for_doc(&self, structure: &[QueryElement], term_results: &[LweCiphertextOwned<u64>]) -> LweCiphertextOwned<u64> {
        let mut stack: Vec<LweCiphertextOwned<u64>> = Vec::new();
        let one = self.one.clone();
        let mut zero = one.clone();
        lwe_ciphertext_sub(&mut zero, &one, &one);

        for element in structure {
            match element {
                QueryElement::Term(idx) => {
                    let result = if *idx < term_results.len() {
                        term_results[*idx].clone()
                    } else {
                        zero.clone()
                    };
                    stack.push(result);
                }
                QueryElement::Not => {
                    if let Some(operand) = stack.pop() {
                        stack.push(self.not(operand));
                    } else {
                        stack.push(zero.clone());
                    }
                }
                QueryElement::And => {
                    if stack.len() >= 2 {
                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                        stack.push(self.and(left, right));
                    } else {
                        stack.push(zero.clone());
                    }
                }
                QueryElement::Or => {
                    if stack.len() >= 2 {
                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                        stack.push(self.or(left, right));
                    } else {
                        stack.push(zero.clone());
                    }
                }
                _ => {
                }
            }
        }
        
        stack.pop().unwrap_or(zero.clone())
    }

    pub fn and(&self, ct0: LweCiphertextOwned<u64>, ct1: LweCiphertextOwned<u64>) -> LweCiphertextOwned<u64> {
        let mut result = self.zero.clone();
        let mut ct_diff = ct0.clone();

        lwe_ciphertext_add(
            &mut ct_diff,
            &ct0,
            &ct1,
        );
        programmable_bootstrap_lwe_ciphertext(
            &ct_diff,
            &mut result,
            &self.and_lut,
            &self.fourier_bsk,
        );

        let mut output_ct = self.one.clone();
        keyswitch_lwe_ciphertext(&self.ksk_big_to_small, &result, &mut output_ct);
        output_ct
    }

    pub fn or(&self, ct0: LweCiphertextOwned<u64>, ct1: LweCiphertextOwned<u64>) -> LweCiphertextOwned<u64> {
        let mut result = self.zero.clone();
        let mut ct_diff = ct0.clone();

        lwe_ciphertext_add(
            &mut ct_diff,
            &ct0,
            &ct1,
        );
        programmable_bootstrap_lwe_ciphertext(
            &ct_diff,
            &mut result,
            &self.or_lut,
            &self.fourier_bsk,
        );

        let mut output_ct = self.one.clone();
        keyswitch_lwe_ciphertext(&self.ksk_big_to_small, &result, &mut output_ct);
        output_ct
    }

    pub fn equal(&self, ct0: LweCiphertextOwned<u64>, ct1: LweCiphertextOwned<u64>) -> LweCiphertextOwned<u64> {
        let mut result = self.zero.clone();
        let mut ct_diff = ct0.clone();

        lwe_ciphertext_sub(
            &mut ct_diff,
            &ct0,
            &ct1,
        );
        programmable_bootstrap_lwe_ciphertext(
            &ct_diff,
            &mut result,
            &self.equal_lut,
            &self.fourier_bsk,
        );

        let mut output_ct = self.one.clone();
        keyswitch_lwe_ciphertext(&self.ksk_big_to_small, &result, &mut output_ct);
        output_ct
    }

    pub fn not(&self, ct: LweCiphertextOwned<u64>) -> LweCiphertextOwned<u64> {
        let one = self.one.clone();
        let mut ct_diff = ct.clone();

        lwe_ciphertext_sub(
            &mut ct_diff,
            &one,
            &ct,
        );
        ct_diff
    }
}
