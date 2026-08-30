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
use std::path::Path;
use rayon::prelude::*;

pub const THREAD_NUM: usize = 64;
pub const DOCUMENT_NUM: usize = 16384;
pub const KEYWORD_SET_NUM: usize = 16;

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
            DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));
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
    
    pub fn qiyana1_query(&self, strings: &str)
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

    pub fn qiyana1_recovery(&self, result: Vec<LweCiphertextOwned<u64>>) -> Vec<bool> {
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

pub struct Server {
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

impl Server {
    pub fn new<P: AsRef<Path>>(seeded_bsk: SeededLweBootstrapKey<Vec<u64>>, 
        seeded_ksk: SeededLweKeyswitchKey<Vec<u64>>, equal_lut: GlweCiphertextOwned<u64>,
        and_lut: GlweCiphertextOwned<u64>, or_lut: GlweCiphertextOwned<u64>, 
        one: LweCiphertextOwned<u64>, zero: LweCiphertextOwned<u64>, file_path: P) -> Self {
        let content = fs::read_to_string(file_path).unwrap(); 
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

    pub fn qiyana1_response(&self, query: Vec<Vec<LweCiphertextOwned<u64>>>,
        querysum: Vec<LweCiphertextOwned<u64>>, 
        template: String) -> Vec<LweCiphertextOwned<u64>> {
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
        
        let final_results: Vec<LweCiphertextOwned<u64>> = subset_results
            .par_iter()
            .map(|res| self.evaluate_rpn_for_doc(&queryformat, res))
            .collect();
        
        println!("Boolean match time: {:?}", boolexe.elapsed());

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
        let mut ct_diff = one.clone();

        lwe_ciphertext_sub(
            &mut ct_diff,
            &one,
            &ct,
        );
        ct_diff
    }
}

#[test]
fn test_qiyana1_simulate(){
    rayon::ThreadPoolBuilder::new()
        .num_threads(THREAD_NUM)
        .build_global()
        .unwrap();
    let file_path = "/root/Qiyana-experiment/keyword.txt";
    let client = Client::new();
    let pre = Instant::now();
    let server = Server::new(client.seeded_bsk.clone(), 
        client.seeded_ksk.clone(), 
        client.equal_lut.clone(),
        client.and_lut.clone(),
        client.or_lut.clone(),
        client.one.clone(),
        client.zero.clone(),
        file_path);
    println!("preprocess time: {:?}", pre.elapsed());
    let test_string = "cladoniaceae AND cladonia OR species".to_string();

    let query_gen = Instant::now();
    let (query, querysum, template) = client.qiyana1_query(&test_string);
    let query_time = query_gen.elapsed();
    println!("query generation time: {:?}", query_time);

    let serialized_query = bincode::serialize(&query).unwrap();
    let mut serialized_query_size = serialized_query.len();
    serialized_query_size += bincode::serialize(&querysum).unwrap().len();
    serialized_query_size += bincode::serialize(&template).unwrap().len();
    println!("Query serialized size: {} bytes", serialized_query_size);

    let response_gen = Instant::now();
    let response = server.qiyana1_response(query, querysum, template);
    let response_time = response_gen.elapsed();
    println!("response generation time: {:?}", response_time);

    let serialized_response = bincode::serialize(&response).unwrap();
    let serialized_response_size = serialized_response.len();
    println!("Response serialized size: {} bytes", serialized_response_size);

    let recover_gen = Instant::now();
    let recovered = client.qiyana1_recovery(response);
    let recover_time = recover_gen.elapsed();
    println!("recovery time: {:?}", recover_time);

    println!("The document number is {}.", server.keywords.len());
    println!("The first line is {}.", recovered[0]);
    println!("The others lines are {}.", recovered[1]);
    for i in 1..DOCUMENT_NUM{
        assert_eq!(recovered[i], false);
    }
}
