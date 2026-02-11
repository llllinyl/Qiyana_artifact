#![allow(unused_imports)]

use tfhe::core_crypto::prelude::*;
use std::time::Instant;
use tfhe::shortint::parameters::DynamicDistribution;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tfhe::core_crypto::algorithms::*;

pub const DOCUMENT_NUM: usize = 16;

pub const KEYWORD_NUM: usize = 65536;

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
        Some(Ok(keywords_line)) => {
            let keyword_count = keywords_line.split(',').count();
            println!("Number of keywords: {}", keyword_count);
        }
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
    
    println!("Successfully read {} documents", tf_idf_matrix.len());
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
    let mut result = Vec::with_capacity(decomposed_data.len());
    
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

#[test]
fn test_matrix_vector_mul(){
    let small_lwe_dimension = LweDimension(1055);
    let lwe_size = LweSize(1056);
    let glwe_dimension = GlweDimension(1);
    let polynomial_size = PolynomialSize(16384);
    let lwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));//512 ciphertext sum; real 2^{-40}
    let glwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(2.168404344971009e-19));
    let pbs_base_log = DecompositionBaseLog(15);
    let pbs_level = DecompositionLevelCount(2);
    let decomp_base_log = DecompositionBaseLog(23);
    let decomp_level_count = DecompositionLevelCount(1);
    let ciphertext_modulus = CiphertextModulus::new_native();
    let mut boxed_seeder = new_seeder();
    let seeder = boxed_seeder.as_mut();

    let mut secret_generator =
        SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());

    let mut encryption_generator =
        EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);

    println!("Generating keys...");

    let small_lwe_sk =
        LweSecretKey::generate_new_binary(small_lwe_dimension, &mut secret_generator);

    let glwe_sk =
        GlweSecretKey::generate_new_binary(glwe_dimension, polynomial_size, &mut secret_generator);

    let std_bootstrapping_key = par_allocate_and_generate_new_lwe_bootstrap_key(
        &small_lwe_sk,
        &glwe_sk,
        pbs_base_log,
        pbs_level,
        glwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let mut fourier_bsk = FourierLweBootstrapKey::new(
        std_bootstrapping_key.input_lwe_dimension(),
        std_bootstrapping_key.glwe_size(),
        std_bootstrapping_key.polynomial_size(),
        std_bootstrapping_key.decomposition_base_log(),
        std_bootstrapping_key.decomposition_level_count(),
    );

    convert_standard_lwe_bootstrap_key_to_fourier(&std_bootstrapping_key, &mut fourier_bsk);
    drop(std_bootstrapping_key);

    // let pksk = allocate_and_generate_new_lwe_packing_keyswitch_key(
    //     &small_lwe_sk,
    //     &glwe_sk,
    //     decomp_base_log,
    //     decomp_level_count,
    //     glwe_noise_distribution,
    //     ciphertext_modulus,
    //     &mut encryption_generator,
    // );
    let seeded_pksk = allocate_and_generate_new_seeded_lwe_packing_keyswitch_key(
        &small_lwe_sk,
        &glwe_sk,
        decomp_base_log,
        decomp_level_count,
        glwe_noise_distribution,
        ciphertext_modulus,
        seeder,
    );

    let message_modulus = 1u64 << 7;

    let delta = (1_u64 << 63) / message_modulus;

    let file_path = "/home/lyl/Desktop/Qiyana/tf-idf.txt";

    let matrix = read_tf_idf_file(file_path, DOCUMENT_NUM);

    let submatrix = decompose_matrix_2bit(&matrix);

    let mut vector = Vec::new();
    for num in 0..KEYWORD_NUM {
        let input_message = if num < 32 { 1u64 } else { 0u64 };
        let mut lwe = SeededLweCiphertext::new(
            0u64,
            small_lwe_dimension.to_lwe_size(),
            seeder.seed().into(),  // ← 只存储种子
            ciphertext_modulus,
        );

        // 加密为 seeded 密文
        encrypt_seeded_lwe_ciphertext(
            &small_lwe_sk,
            &mut lwe,           // ← 输出是压缩格式
            Plaintext(input_message * delta),
            lwe_noise_distribution,
            seeder,
        );
        vector.push(lwe);

        // let lwe_ciphertext_in: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        //     &small_lwe_sk,
        //     Plaintext(input_message * delta),
        //     lwe_noise_distribution,
        //     ciphertext_modulus,
        //     &mut encryption_generator,
        // );
        // vector.push(lwe_ciphertext_in);
    }

    let serialized_u16 = bincode::serialize(&vector).unwrap();
    let serialized_size_u16 = serialized_u16.len();
    println!("Vector size: {} bytes", serialized_size_u16);

    println!("Performing matrix vector mul...");
    let mul_start = Instant::now();
    // let mut results = Vec::new();
    let zero_ct: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        Plaintext(0u64 * delta),
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let mut mul_curr = zero_ct.clone();
    let mut real_vector = Vec::new();
    for num in 0..KEYWORD_NUM {
        let lwe = vector[num].decompress_into_lwe_ciphertext();
        real_vector.push(lwe);
    }

    let mut container = Vec::new();
    for doc_idx in 0..DOCUMENT_NUM {
        // let mut final_result = Vec::new();
        
        for bit_position in 0..5 {
            let mut partial_result = zero_ct.clone();
            
            for keyword_idx in 0..KEYWORD_NUM {
                lwe_ciphertext_cleartext_mul(
                    &mut mul_curr,
                    &real_vector[keyword_idx],
                    Cleartext(submatrix[bit_position][doc_idx][keyword_idx] as u64),
                );
                lwe_ciphertext_add_assign(&mut partial_result, &mul_curr);
            }
            
            container.extend_from_slice(partial_result.as_ref());
            // final_result.push(partial_result);
        }
        
        // results.push(final_result);
    }
    println!("MVM time: {:?}", mul_start.elapsed());

    let mul_start = Instant::now();
    let lwe_list = LweCiphertextList::from_container(
        container,
        lwe_size,
        ciphertext_modulus,
    );
    println!("lwe number is {}", lwe_list.lwe_ciphertext_count().0);
    let mut output_glwe = GlweCiphertext::new(
        0u64,
        GlweSize(2),
        PolynomialSize(16384),
        ciphertext_modulus,
    );

    let decompress = Instant::now();
    let pksk = seeded_pksk.decompress_into_lwe_packing_keyswitch_key();
    println!("recovery time: {:?}", decompress.elapsed());

    keyswitch_lwe_ciphertext_list_and_pack_in_glwe_ciphertext(
        &pksk,
        &lwe_list,
        &mut output_glwe,
    );
    println!("packing time: {:?}", mul_start.elapsed());

    let serialized_u16 = bincode::serialize(&output_glwe).unwrap();
    let serialized_size_u16 = serialized_u16.len();
    println!("After packing, response size: {} bytes", serialized_size_u16);

    let mut decrypted_plaintext_list =
        PlaintextList::new(0u64, PlaintextCount(output_glwe.polynomial_size().0));

    decrypt_glwe_ciphertext(
        &glwe_sk,
        &output_glwe,
        &mut decrypted_plaintext_list,
    );

    let decomposer = SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));
    decrypted_plaintext_list
        .iter_mut()
        .for_each(|x| *x.0 = decomposer.closest_representable(*x.0) / delta);
    let mut plainsubmatrix = Vec::new();
    let plaintext_slice = decrypted_plaintext_list.as_ref();
    for ind in 0..DOCUMENT_NUM {
        let mut plainrow = Vec::new();
        for num in 0..5 {
            let idx = ind * 5 + num;
            if plaintext_slice[idx] > 96 {
                println!("error! {}", plaintext_slice[idx]);
            }
            plainrow.push(plaintext_slice[idx] as u8);
        }
        plainsubmatrix.push(plainrow);
    }

    // let mut plainsubmatrix = Vec::new();
    // for ind in 0..DOCUMENT_NUM {
    //     let row_cipher = results[ind].clone();
    //     let mut plainrow = Vec::new();
    //     for num in 0..5 {
    //         let plain: Plaintext<u64> =
    //             decrypt_lwe_ciphertext(&small_lwe_sk, &row_cipher[num]);

    //         let signed_decomposer =
    //             SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));

    //         let real_plain: u64 =
    //             signed_decomposer.closest_representable(plain.0) / delta;

    //         if real_plain > 96 {
    //             println!("error! {}", real_plain);
    //         } 
    //         plainrow.push(real_plain as u8);
    //     }
    //     if ind == 0 {
    //         println!("The first line: {:?}", plainrow);
    //     }
    //     plainsubmatrix.push(plainrow);
    // }

    let final_result = compose_matrices(&plainsubmatrix);
    for ind in 0..DOCUMENT_NUM  {
        let mut value = 0u16;
        for num in 0..32 {
            value += matrix[ind][num];
        }
        println!("real score {}, final score {}", value, final_result[ind]);
    }
}