#[path = "../utils/decomposition.rs"]
mod decomposition_mod;
#[path = "../utils/bloomfilter.rs"] 
mod bloomfilter_mod;
use bloomfilter_mod::BloomFilter;
use decomposition_mod::*;
use tfhe::core_crypto::prelude::*;
use std::time::Instant;
use tfhe::shortint::parameters::DynamicDistribution;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use rayon::prelude::*;
use std::sync::Arc;

pub const DOCUMENT_NUM: usize = 16384;
pub const KEYWORD_NUM: usize = 65536;
pub const THREAD_NUM: usize = 64;
pub const PACKING_NUM: usize = 16380;
pub const LWESIZE: usize = 1056;
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

pub fn decompose_matrix_2bit(matrix: &[Vec<u16>]) -> [Vec<Vec<u8>>; 5] {
    let mut decomposed_matrices: [Vec<Vec<u8>>; 5] = [
        Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()
    ];
    
    for row in matrix {
        let mut decomposed_row0 = Vec::new();
        let mut decomposed_row1 = Vec::new();
        let mut decomposed_row2 = Vec::new();
        let mut decomposed_row3 = Vec::new();
        let mut decomposed_row4 = Vec::new();
        
        for &value in row {
            decomposed_row0.push(((value >> 0) & 0b11) as u8);
            decomposed_row1.push(((value >> 2) & 0b11) as u8);
            decomposed_row2.push(((value >> 4) & 0b11) as u8);
            decomposed_row3.push(((value >> 6) & 0b11) as u8);
            decomposed_row4.push(((value >> 8) & 0b11) as u8);
        }
        
        decomposed_matrices[0].push(decomposed_row0);
        decomposed_matrices[1].push(decomposed_row1);
        decomposed_matrices[2].push(decomposed_row2);
        decomposed_matrices[3].push(decomposed_row3);
        decomposed_matrices[4].push(decomposed_row4);
    }
    
    decomposed_matrices
}

pub fn compose_matrices(decomposed_data: &Vec<Vec<u8>>) -> Vec<u16> {
    let mut result = Vec::with_capacity(DOCUMENT_NUM);
    
    for row in decomposed_data {
        if row.len() != 5 {
            panic!("Each row must contain exactly 5 values");
        }
        
        let mut value: u16 = 0;
        value += (row[0] as u16) << 0;
        value += (row[1] as u16) << 2;
        value += (row[2] as u16) << 4;
        value += (row[3] as u16) << 6;
        value += (row[4] as u16) << 8;
        
        result.push(value);
    }
    
    result
}

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
    glwe_sk: GlweSecretKey<Vec<u64>>,
    lwe_noise_distribution: DynamicDistribution<u64>,
    pub seeded_bsk: SeededLweBootstrapKey<Vec<u64>>,
    pub seeded_ksk: SeededLweKeyswitchKey<Vec<u64>>,
    pub seeded_pk: SeededLwePackingKeyswitchKey<Vec<u64>>, 
    pub equal_lut: GlweCiphertextOwned<u64>,
    pub and_lut: GlweCiphertextOwned<u64>,
    pub or_lut: GlweCiphertextOwned<u64>,
    pub mul_lut: GlweCiphertextOwned<u64>,    
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
        let pk_decomp_base_log = DecompositionBaseLog(23);
        let pk_decomp_level_count = DecompositionLevelCount(1);
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
        // let packing_key = allocate_and_generate_new_lwe_packing_keyswitch_key(
        //     &small_lwe_sk,
        //     &glwe_sk,
        //     pk_decomp_base_log,
        //     pk_decomp_level_count,
        //     glwe_noise_distribution,
        //     ciphertext_modulus,
        //     &mut encryption_generator,
        // );
        let seeded_pk = allocate_and_generate_new_seeded_lwe_packing_keyswitch_key(
            &small_lwe_sk,
            &glwe_sk,
            pk_decomp_base_log,
            pk_decomp_level_count,
            glwe_noise_distribution,
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
        let mul_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
            polynomial_size,
            glwe_dimension.to_glwe_size(),
            message_modulus as usize,
            ciphertext_modulus,
            delta,
            |x: u64| (x * (x - 1)) / 2
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
            glwe_sk,
            lwe_noise_distribution,
            seeded_bsk,
            seeded_ksk,
            seeded_pk,
            equal_lut,
            and_lut,
            or_lut,
            mul_lut,
            one,
            zero,
        }
    }
    
    pub fn qiyana000_query(&self, strings: &str)
         -> (Vec<Vec<LweCiphertextOwned<u64>>>, 
            Vec<LweCiphertextOwned<u64>>, 
            String, Vec<LweCiphertextOwned<u64>>) {
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

        let mut rank_vector = Vec::new();
        for num in 0..KEYWORD_NUM {
            let input_message = match num {
                0 | 1 | 2 | 3 | 4 | 6 => 1u64,
                _ => 0u64,
            };
            let lwe_ciphertext_in: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
                &self.small_lwe_sk,
                Plaintext(input_message * delta),
                lwe_noise_distribution,
                ciphertext_modulus,
                &mut encryption_generator,
            );
            rank_vector.push(lwe_ciphertext_in);
        }

        (query, querysum, template, rank_vector)
    }

    pub fn qiyana000_compress_query(&self, strings: &str)
         -> (Vec<Vec<LweCiphertextOwned<u64>>>, 
            Vec<LweCiphertextOwned<u64>>, 
            String, Vec<SeededLweCiphertext<u64>>) {
        let message_modulus = 1u64 << 7;
        let delta = (1_u64 << 63) / message_modulus;
        let small_lwe_dimension = LweDimension(1055);
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

        let mut rank_vector = Vec::new();
        for num in 0..KEYWORD_NUM {
            let input_message = match num {
                0 | 1 | 2 | 3 | 4 | 6 => 1u64,
                _ => 0u64,
            };
            let mut lwe = SeededLweCiphertext::new(
                0u64,
                small_lwe_dimension.to_lwe_size(),
                seeder.seed().into(),
                ciphertext_modulus,
            );

            encrypt_seeded_lwe_ciphertext(
                &self.small_lwe_sk,
                &mut lwe,
                Plaintext(input_message * delta),
                lwe_noise_distribution,
                seeder,
            );
            rank_vector.push(lwe);
        }

        (query, querysum, template, rank_vector)
    }

    pub fn qiyana000_recovery(&self, results: Vec<Vec<LweCiphertextOwned<u64>>>) -> Vec<u16> {
        let message_modulus = 1u64 << 7;
        let delta = (1_u64 << 63) / message_modulus;
        let small_sk = self.small_lwe_sk.clone();
        let mut plainsubmatrix = Vec::new();
        for ind in 0..DOCUMENT_NUM {
            let row_cipher = results[ind].clone();
            let mut plainrow = Vec::new();
            for num in 0..5 {
                let plain: Plaintext<u64> =
                    decrypt_lwe_ciphertext(&small_sk, &row_cipher[num]);

                let signed_decomposer =
                    SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));

                let real_plain: u64 =
                    signed_decomposer.closest_representable(plain.0) / delta;

                if real_plain > 96 {
                    println!("error! {}", real_plain);
                } 
                plainrow.push(real_plain as u8);
            }
            plainsubmatrix.push(plainrow);
        }

        let final_result = compose_matrices(&plainsubmatrix);
        final_result
    }

    pub fn qiyana000_packing_recovery(&self, results: Vec<GlweCiphertext<Vec<u64>>>) -> Vec<u16> {
        let message_modulus = 1u64 << 7;
        let delta = (1_u64 << 63) / message_modulus;
        let glwe_sk = self.glwe_sk.clone();
        let mut plainsubmatrix = Vec::new();
        let length = results.len();
        for (idx, packed) in results.into_iter().enumerate() {
            let mut decrypted_plaintext_list =
                PlaintextList::new(0u64, PlaintextCount(glwe_sk.polynomial_size().0));

            decrypt_glwe_ciphertext(
                &glwe_sk,
                &packed,
                &mut decrypted_plaintext_list,
            );

            let decomposer = SignedDecomposer::new(
                DecompositionBaseLog(8), 
                DecompositionLevelCount(1)
            );
            
            decrypted_plaintext_list
                .iter_mut()
                .for_each(|x| *x.0 = decomposer.closest_representable(*x.0) / delta);
            
            let plaintext_slice = decrypted_plaintext_list.as_ref();

            let range = if idx < length - 1 {
                PACKING_NUM / 5
            } else {
                DOCUMENT_NUM % (PACKING_NUM / 5)
            };

            for num in 0..range {
                let mut plainrow = Vec::new();
                for offset in 0..5 {
                    let index = 5 * num + offset;
                    if index < plaintext_slice.len() {
                        if plaintext_slice[index] > 96 {
                            println!("error! {}", plaintext_slice[index]);
                        }
                        plainrow.push(plaintext_slice[index] as u8);
                    }
                }
                if plainrow.len() == 5 {
                    plainsubmatrix.push(plainrow);
                }
            }
        }
        let final_result = compose_matrices(&plainsubmatrix);
        final_result
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum TemplateNode {
    Operand(usize, usize),
    And(Box<TemplateNode>, Box<TemplateNode>),
    Or(Box<TemplateNode>, Box<TemplateNode>),
    Not(Box<TemplateNode>),
}

pub struct Server {
    pub keywords: Vec<Vec<u16>>,
    pub fourier_bsk: FourierLweBootstrapKeyOwned,
    pub ksk_big_to_small: LweKeyswitchKeyOwned<u64>,
    pub packing_key: LwePackingKeyswitchKey<Vec<u64>>, 
    pub equal_lut: GlweCiphertextOwned<u64>,
    pub and_lut: GlweCiphertextOwned<u64>,
    pub or_lut: GlweCiphertextOwned<u64>,
    pub mul_lut: GlweCiphertextOwned<u64>,
    pub one: LweCiphertextOwned<u64>,
    pub zero: LweCiphertextOwned<u64>,
    pub submatrix: [Vec<Vec<u8>>; 5],
}

impl Server {
    pub fn new<P: AsRef<Path>>(seeded_bsk: SeededLweBootstrapKey<Vec<u64>>, 
        seeded_ksk: SeededLweKeyswitchKey<Vec<u64>>, seeded_pk: SeededLwePackingKeyswitchKey<Vec<u64>>, 
        equal_lut: GlweCiphertextOwned<u64>, and_lut: GlweCiphertextOwned<u64>, 
        or_lut: GlweCiphertextOwned<u64>, mul_lut: GlweCiphertextOwned<u64>, 
        one: LweCiphertextOwned<u64>, zero: LweCiphertextOwned<u64>, 
        keyword_path: P, tfidf_path: &str) -> Self {
        let content = fs::read_to_string(keyword_path).unwrap(); 
        let mut keywords = Vec::with_capacity(DOCUMENT_NUM);

        for (i, line) in content.lines().enumerate() {
            let filter = BloomFilter::new();
            if i >= DOCUMENT_NUM {
                break;
            }
            let mut index = 0;
            for part in line.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() && index < KEYWORD_SET_NUM {
                    filter.insert(&trimmed.to_string());
                    index += 1;
                }
            }
            let indices = filter.get_all_set_bits_indices();
            
            keywords.push(indices);
            drop(filter);
        }

        let matrix = read_tf_idf_file(tfidf_path, DOCUMENT_NUM);
        let submatrix = decompose_matrix_2bit(&matrix);

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
        let packing_key = seeded_pk.decompress_into_lwe_packing_keyswitch_key();
        
        Self {
            keywords,
            fourier_bsk,
            ksk_big_to_small,
            packing_key,
            equal_lut,
            and_lut,
            or_lut,
            mul_lut,
            one,
            zero,
            submatrix,
        }
    }

    pub fn qiyana000_response(&self, query: Vec<Vec<LweCiphertextOwned<u64>>>,
        querysum: Vec<LweCiphertextOwned<u64>>, 
        template: String,
        rank_vector: Vec<LweCiphertextOwned<u64>>) -> Vec<Vec<LweCiphertextOwned<u64>>> {
        let keywords = self.keywords.clone();

        let equal = Instant::now();
        let subset_results: Vec<Vec<LweCiphertextOwned<u64>>> = keywords
            .par_iter()
            .map(|keyrow| {
                query
                    .par_iter()
                    .enumerate()
                    .map(|(quid, qu)| {
                        let mut sum = qu[keyrow[0] as usize].clone();
                        let length = keyrow.len();
                        
                        for ind in 1..length {
                            lwe_ciphertext_add_assign(&mut sum, &qu[keyrow[ind] as usize]);
                        }

                        self.equal(querysum[quid].clone(), sum)
                    })
                    .collect::<Vec<LweCiphertextOwned<u64>>>()
            })
            .collect();
        println!("homomorphic subset testing time: {:?}", equal.elapsed());

        let boolexe = Instant::now();
        let (_strings, queryformat) = parse_query_to_rpn(&template);

        let match_results: Vec<LweCiphertextOwned<u64>> = subset_results
            .par_iter()
            .map(|res| self.evaluate_rpn_for_doc(&queryformat, res))
            .collect();
        
        println!("Boolean match time: {:?}", boolexe.elapsed());
        
        let mul_start = Instant::now();
        let submatrix_arc = Arc::new(self.submatrix.clone());
        let one_clone_arc = Arc::new(self.one.clone());
        let rank_vector_arc = Arc::new(rank_vector);

        let results: Vec<Vec<LweCiphertextOwned<u64>>> = (0..DOCUMENT_NUM)
            .into_par_iter()
            .map(|doc_idx| {
                (0..5)
                    .into_par_iter()
                    .map(|bit_position| {
                        let mut partial_result = one_clone_arc.as_ref().clone();
                        let mut mul_curr = one_clone_arc.as_ref().clone();
                        
                        lwe_ciphertext_cleartext_mul(
                            &mut partial_result,
                            &rank_vector_arc[0],
                            Cleartext(submatrix_arc[bit_position][doc_idx][0] as u64),
                        );
                        
                        for keyword_idx in 1..KEYWORD_NUM {
                            lwe_ciphertext_cleartext_mul(
                                &mut mul_curr,
                                &rank_vector_arc[keyword_idx],
                                Cleartext(submatrix_arc[bit_position][doc_idx][keyword_idx] as u64),
                            );
                            lwe_ciphertext_add_assign(&mut partial_result, &mul_curr);
                        }
                        
                        partial_result
                    })
                    .collect()
            })
            .collect();
        println!("MVM time: {:?}", mul_start.elapsed());

        let select = Instant::now();
        let final_results: Vec<Vec<LweCiphertextOwned<u64>>> = (0..keywords.len())
            .into_par_iter()
            .map(|num| {
                let row = results[num].clone();
                let mask = match_results[num].clone();
                (0..5)
                    .into_par_iter()
                    .map(|ind| self.mul(mask.clone(), row[ind].clone()))
                    .collect()
            })
            .collect();
        println!("TFHE selection time: {:?}", select.elapsed());

        final_results
    }

    pub fn qiyana000_decompress_response(&self, query: Vec<Vec<LweCiphertextOwned<u64>>>,
        querysum: Vec<LweCiphertextOwned<u64>>, 
        template: String,
        rank_vector: Vec<SeededLweCiphertext<u64>>) -> Vec<GlweCiphertext<Vec<u64>>> {
        let keywords = self.keywords.clone();

        let equal = Instant::now();
        let subset_results: Vec<Vec<LweCiphertextOwned<u64>>> = keywords
            .par_iter()
            .map(|keyrow| {
                query
                    .par_iter()
                    .enumerate()
                    .map(|(quid, qu)| {
                        let mut sum = qu[keyrow[0] as usize].clone();
                        let length = keyrow.len();
                        
                        for ind in 1..length {
                            lwe_ciphertext_add_assign(&mut sum, &qu[keyrow[ind] as usize]);
                        }

                        self.equal(querysum[quid].clone(), sum)
                    })
                    .collect::<Vec<LweCiphertextOwned<u64>>>()
            })
            .collect();
    
        println!("homomorphic subset testing time: {:?}", equal.elapsed());

        let boolexe = Instant::now();
        let (_strings, queryformat) = parse_query_to_rpn(&template);
        
        let match_results: Vec<LweCiphertextOwned<u64>> = subset_results
            .par_iter()
            .map(|res| self.evaluate_rpn_for_doc(&queryformat, res))
            .collect();
        
        println!("Boolean match time: {:?}", boolexe.elapsed());
        
        let mul_start = Instant::now();
        let real_vector: Vec<LweCiphertextOwned<u64>> = rank_vector
            .par_iter()
            .map(|ct| ct.decompress_into_lwe_ciphertext())
            .collect();
        
        let real_vector_arc = Arc::new(real_vector);
        let submatrix_arc = Arc::new(self.submatrix.clone());
        let one_clone_arc = Arc::new(self.one.clone());

        let results: Vec<Vec<LweCiphertextOwned<u64>>> = (0..DOCUMENT_NUM)
            .into_par_iter()
            .map(|doc_idx| {
                (0..5)
                    .into_par_iter()
                    .map(|bit_position| {
                        let mut partial_result = one_clone_arc.as_ref().clone();
                        let mut mul_curr = one_clone_arc.as_ref().clone();
                        
                        lwe_ciphertext_cleartext_mul(
                            &mut partial_result,
                            &real_vector_arc[0],
                            Cleartext(submatrix_arc[bit_position][doc_idx][0] as u64),
                        );
                        
                        for keyword_idx in 1..KEYWORD_NUM {
                            lwe_ciphertext_cleartext_mul(
                                &mut mul_curr,
                                &real_vector_arc[keyword_idx],
                                Cleartext(submatrix_arc[bit_position][doc_idx][keyword_idx] as u64),
                            );
                            lwe_ciphertext_add_assign(&mut partial_result, &mul_curr);
                        }
                        
                        partial_result
                    })
                    .collect()
            })
            .collect();
        
        println!("MVM time: {:?}", mul_start.elapsed());

        let select = Instant::now();
        let all_containers: Vec<Vec<u64>> = {
            let flat_items: Vec<u64> = (0..keywords.len())
                .into_par_iter()
                .flat_map(|num| {
                    let row = results[num].clone();
                    let mask = match_results[num].clone();
                    let mut flat = Vec::with_capacity(5);
                    
                    for ind in 0..5 {
                        let mul_res = self.mul(mask.clone(), row[ind].clone());
                        flat.extend_from_slice(mul_res.as_ref());
                    }
                    flat
                })
                .collect();
            
            flat_items
                .chunks(PACKING_NUM * LWESIZE)
                .map(|chunk| chunk.to_vec())
                .collect()
        };

        println!("TFHE selection and container building time: {:?}", select.elapsed());

        let packing_start = Instant::now();
        let ciphertext_modulus = CiphertextModulus::new_native();
        let packing_key = self.packing_key.clone();
        let mut final_results = Vec::new();

        for packed in all_containers {
            let lwe_list = LweCiphertextList::from_container(
                packed,
                LweSize(1056),
                ciphertext_modulus,
            );
                
            let mut output_glwe = GlweCiphertext::new(
                0u64,
                GlweSize(2),
                PolynomialSize(16384),
                ciphertext_modulus,
            );
                
            par_keyswitch_lwe_ciphertext_list_and_pack_in_glwe_ciphertext(
                &packing_key,
                &lwe_list,
                &mut output_glwe,
            );

            final_results.push(output_glwe);
        }
        println!("TFHE packing time: {:?}", packing_start.elapsed());
        
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

    pub fn mul(&self, ct0: LweCiphertextOwned<u64>, ct1: LweCiphertextOwned<u64>) -> LweCiphertextOwned<u64> {
        let mut result1 = self.zero.clone();
        let mut result2 = self.zero.clone();
        let mut result3 = self.zero.clone();
        let mut add_ct = ct0.clone();
        lwe_ciphertext_add(
            &mut add_ct,
            &ct0,
            &ct1,
        );

        programmable_bootstrap_lwe_ciphertext(
            &ct1,
            &mut result1,
            &self.mul_lut,
            &self.fourier_bsk,
        );
                    
        programmable_bootstrap_lwe_ciphertext(
            &add_ct,
            &mut result2,
            &self.mul_lut,
            &self.fourier_bsk,
        );
                    
        lwe_ciphertext_sub(
            &mut result3,
            &result2,
            &result1,
        );

        let mut output_ct = self.one.clone();
        keyswitch_lwe_ciphertext(&self.ksk_big_to_small, &result3, &mut output_ct);
        output_ct
    }
}

#[test]
fn test_qiyana000_simulate(){
    rayon::ThreadPoolBuilder::new()
        .num_threads(THREAD_NUM)
        .build_global()
        .unwrap();
    let keyword_path = "/root/Qiyana000-experiment/keyword.txt";
    let tfidf_path = "/root/Qiyana000-experiment/tf-idf.txt";
    let client = Client::new();
    let pre = Instant::now();
    let server = Server::new(client.seeded_bsk.clone(), 
        client.seeded_ksk.clone(), 
        client.seeded_pk.clone(),
        client.equal_lut.clone(),
        client.and_lut.clone(),
        client.or_lut.clone(),
        client.mul_lut.clone(),
        client.one.clone(),
        client.zero.clone(),
        keyword_path,
        tfidf_path);
    println!("preprocess time: {:?}", pre.elapsed());
    let test_string = "cladoniaceae AND cladonia OR species".to_string();

    let query_gen = Instant::now();
    let (query, querysum, template, rank_vector) = client.qiyana000_query(&test_string);
    let query_time = query_gen.elapsed();
    println!("query generation time: {:?}", query_time);

    let serialized_query = bincode::serialize(&query).unwrap();
    let mut serialized_query_size = serialized_query.len();
    serialized_query_size += bincode::serialize(&querysum).unwrap().len();
    serialized_query_size += bincode::serialize(&template).unwrap().len();
    serialized_query_size += bincode::serialize(&rank_vector).unwrap().len();
    println!("Query serialized size: {} bytes", serialized_query_size);

    let response_gen = Instant::now();
    let response = server.qiyana000_response(query, querysum, template, rank_vector);
    let response_time = response_gen.elapsed();
    println!("response generation time: {:?}", response_time);

    let serialized_response = bincode::serialize(&response).unwrap();
    let serialized_response_size = serialized_response.len();
    println!("Response serialized size: {} bytes", serialized_response_size);

    let recover_gen = Instant::now();
    let recovered = client.qiyana000_recovery(response);
    let recover_time = recover_gen.elapsed();
    println!("recovery time: {:?}", recover_time);

    println!("The document number is {}.", DOCUMENT_NUM);
    for ind in 0..DOCUMENT_NUM  {
        println!("final score {}", recovered[ind]);
    }
}

#[test]
fn test_qiyana000_zip_simulate(){
    rayon::ThreadPoolBuilder::new()
        .num_threads(THREAD_NUM)
        .build_global()
        .unwrap();
    let keyword_path = "/home/lyl/Desktop/Qiyana000/keyword.txt";
    let tfidf_path = "/home/lyl/Desktop/Qiyana000/tf-idf.txt";
    let client = Client::new();
    let pre = Instant::now();
    let server = Server::new(client.seeded_bsk.clone(), 
        client.seeded_ksk.clone(), 
        client.seeded_pk.clone(),
        client.equal_lut.clone(),
        client.and_lut.clone(),
        client.or_lut.clone(),
        client.mul_lut.clone(),
        client.one.clone(),
        client.zero.clone(),
        keyword_path,
        tfidf_path);
    println!("preprocess time: {:?}", pre.elapsed());
    let test_string = "cladoniaceae AND cladonia OR species".to_string();
    // let test_string = "((cladoniaceae AND lichen) OR (genera AND NOT merri)) AND (thallus OR NOT fossil)".to_string();

    let query_gen = Instant::now();
    let (query, querysum, template, rank_vector) = client.qiyana000_compress_query(&test_string);
    let query_time = query_gen.elapsed();
    println!("query generation time: {:?}", query_time);

    let serialized_query = bincode::serialize(&query).unwrap();
    let mut serialized_query_size = serialized_query.len();
    serialized_query_size += bincode::serialize(&querysum).unwrap().len();
    serialized_query_size += bincode::serialize(&template).unwrap().len();
    serialized_query_size += bincode::serialize(&rank_vector).unwrap().len();
    println!("Query serialized size: {} bytes", serialized_query_size);

    let response_gen = Instant::now();
    let response = server.qiyana000_decompress_response(query, querysum, template, rank_vector);
    let response_time = response_gen.elapsed();
    println!("response generation time: {:?}", response_time);

    let serialized_response = bincode::serialize(&response).unwrap();
    let serialized_response_size = serialized_response.len();
    println!("Response serialized size: {} bytes", serialized_response_size);

    let recover_gen = Instant::now();
    let recovered = client.qiyana000_packing_recovery(response);
    let recover_time = recover_gen.elapsed();
    println!("recovery time: {:?}", recover_time);

    println!("The document number is {}.", DOCUMENT_NUM);
    for ind in 0..DOCUMENT_NUM  {
        println!("final score {}", recovered[ind]);
    }
}
