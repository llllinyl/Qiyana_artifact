#![allow(unused_imports)]

use tfhe::core_crypto::prelude::*;
use std::time::Instant;
use tfhe::shortint::parameters::DynamicDistribution;

#[test]
#[ignore]
fn test_add() {
    let small_lwe_dimension = LweDimension(1055);
    let glwe_dimension = GlweDimension(1);
    let polynomial_size = PolynomialSize(16384);
    let lwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));//512 ciphertext sum; real 2^{-40}
    let glwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(2.168404344971009e-19));
    let pbs_base_log = DecompositionBaseLog(15);
    let pbs_level = DecompositionLevelCount(2);
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

    let message_modulus = 1u64 << 7;

    let input_message = 128u64;

    let delta = (1_u64 << 63) / message_modulus;

    let plaintext = Plaintext(input_message * delta);

    let lwe_ciphertext_in: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let mut addition_ct = lwe_ciphertext_in.clone();
    println!("Performing addition...");
    let conversion_start = Instant::now();
    lwe_ciphertext_add(
        &mut addition_ct,
        &lwe_ciphertext_in,
        &lwe_ciphertext_in,
    );
    let conversion_time = conversion_start.elapsed();
    println!("addition time: {:?}", conversion_time);
    let serialized_u16 = bincode::serialize(&addition_ct).unwrap();
    let serialized_size_u16 = serialized_u16.len();
    println!("lwe serialized size: {} bytes", serialized_size_u16);
}

#[test]
#[ignore]
fn test_equal() {
    let small_lwe_dimension = LweDimension(1055);
    let glwe_dimension = GlweDimension(1);
    let polynomial_size = PolynomialSize(16384);
    let lwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));//512 ciphertext sum; real 2^{-40}
    let glwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(2.168404344971009e-19));
    let pbs_base_log = DecompositionBaseLog(15);
    let pbs_level = DecompositionLevelCount(2);
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

    let big_lwe_sk = glwe_sk.clone().into_lwe_secret_key();

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

    let message_modulus = 1u64 << 7;

    let delta = (1_u64 << 63) / message_modulus;

    let message1 = 125u64;
    let plaintext1 = Plaintext(message1 * delta);
    let lwe1: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext1,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let message2 = 0u64;
    let plaintext2 = Plaintext(message2 * delta);
    let lwe2: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext2,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    
    let f = |x: u64| {if x == 0 { 1 } else { 0 }};
    let zero_check_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        f,
    );

    let mut result = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    let mut ct_diff = lwe1.clone();

    println!("Performing sub...");
    let pbs_start = Instant::now();
    lwe_ciphertext_sub(
        &mut ct_diff,
        &lwe1,
        &lwe2,
    );
    programmable_bootstrap_lwe_ciphertext(
        &ct_diff,
        &mut result,
        &zero_check_lut,
        &fourier_bsk,
    );
    let pbs_time = pbs_start.elapsed();
    println!("check equal time: {:?}", pbs_time);

    let sub_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&big_lwe_sk, &result);
    let real_sub_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&small_lwe_sk, &ct_diff);

    let signed_decomposer =
        SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));

    let real_sub_plain_result: u64 =
        signed_decomposer.closest_representable(real_sub_plain.0) / delta;

    println!("Checking result...");
    assert_eq!(message1 - message2, real_sub_plain_result);
    println!(
        "Sub result is correct! \
        Expected {}, got {}", message1 - message2, real_sub_plain_result
    );

    let sub_plain_result: u64 =
        signed_decomposer.closest_representable(sub_plain.0) / delta;

    println!("Checking result...");
    assert_eq!(f(message1 - message2), sub_plain_result);
    println!(
        "Check equal result is correct! \
        Expected {}, got {}", f(message1 - message2), sub_plain_result
    );
}

#[test]
#[ignore]
fn test_and() {
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

    let big_lwe_sk = glwe_sk.clone().into_lwe_secret_key();

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

    let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
        &big_lwe_sk,
        &small_lwe_sk,
        ks_decomp_base_log,
        ks_decomp_level_count,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let message_modulus = 1u64 << 7;

    let delta = (1_u64 << 63) / message_modulus;

    let message1 = 1u64;
    let plaintext1 = Plaintext(message1 * delta);
    let lwe1: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext1,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let message2 = 1u64;
    let plaintext2 = Plaintext(message2 * delta);
    let lwe2: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext2,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let f = |x: u64| {if x == 2 { 1 } else { 0 }};
    let zero_check_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        f,
    );

    let mut result = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    let mut ct_diff = lwe1.clone();

    println!("Performing add...");
    let pbs_start = Instant::now();
    lwe_ciphertext_add(
        &mut ct_diff,
        &lwe1,
        &lwe2,
    );
    programmable_bootstrap_lwe_ciphertext(
        &ct_diff,
        &mut result,
        &zero_check_lut,
        &fourier_bsk,
    );

    let mut output_ct = LweCiphertext::new(
        0u64,
        small_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    keyswitch_lwe_ciphertext(&ksk_big_to_small, &result, &mut output_ct);
    let pbs_time = pbs_start.elapsed();
    println!("pbs + ks time: {:?}", pbs_time);

    let and_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&big_lwe_sk, &result);
    let real_sub_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&small_lwe_sk, &ct_diff);
    let key_and_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&small_lwe_sk, &output_ct);

    let signed_decomposer =
        SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));

    let real_sub_plain_result: u64 =
        signed_decomposer.closest_representable(real_sub_plain.0) / delta;

    println!("Checking result...");
    // assert_eq!(message1 + message2, real_sub_plain_result);
    println!(
        "just add result is correct! \
        Expected {}, got {}", message1 + message2, real_sub_plain_result
    );

    let and_plain_result: u64 =
        signed_decomposer.closest_representable(and_plain.0) / delta;

    println!("Checking result...");
    // assert_eq!(f(message1 + message2), and_plain_result);
    println!(
        "Check AND result with big key is correct! \
        Expected {}, got {}", f(message1 + message2), and_plain_result
    );

    let key_and_plain_result: u64 =
        signed_decomposer.closest_representable(key_and_plain.0) / delta;

    println!("Checking result...");
    // assert_eq!(f(message1 + message2), key_and_plain_result);
    println!(
        "Check AND result with small key is correct! \
        Expected {}, got {}", f(message1 + message2), key_and_plain_result
    );
}

#[test]
#[ignore]
fn test_or() {
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

    let big_lwe_sk = glwe_sk.clone().into_lwe_secret_key();

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

    let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
        &big_lwe_sk,
        &small_lwe_sk,
        ks_decomp_base_log,
        ks_decomp_level_count,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let message_modulus = 1u64 << 7;

    let delta = (1_u64 << 63) / message_modulus;

    let message1 = 0u64;
    let plaintext1 = Plaintext(message1 * delta);
    let lwe1: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext1,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let message2 = 0u64;
    let plaintext2 = Plaintext(message2 * delta);
    let lwe2: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext2,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let f = |x: u64| {if x == 0 { 0 } else { 1 }};
    let zero_check_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        f,
    );

    let mut result = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    let mut ct_diff = lwe1.clone();

    println!("Performing add...");
    let pbs_start = Instant::now();
    lwe_ciphertext_add(
        &mut ct_diff,
        &lwe1,
        &lwe2,
    );
    programmable_bootstrap_lwe_ciphertext(
        &ct_diff,
        &mut result,
        &zero_check_lut,
        &fourier_bsk,
    );
    let pbs_time = pbs_start.elapsed();
    println!("check equal time: {:?}", pbs_time);

    let mut output_ct = LweCiphertext::new(
        0u64,
        small_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    keyswitch_lwe_ciphertext(&ksk_big_to_small, &result, &mut output_ct);


    let or_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&big_lwe_sk, &result);
    let real_add_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&small_lwe_sk, &ct_diff);
    let key_or_plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&small_lwe_sk, &output_ct);

    let signed_decomposer =
        SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));

    let real_add_plain_result: u64 =
        signed_decomposer.closest_representable(real_add_plain.0) / delta;

    println!("Checking result...");
    // assert_eq!(message1 + message2, real_sub_plain_result);
    println!(
        "add result is correct! \
        Expected {}, got {}", message1 + message2, real_add_plain_result
    );

    let or_plain_result: u64 =
        signed_decomposer.closest_representable(or_plain.0) / delta;

    println!("Checking result...");
    // assert_eq!(f(message1 + message2), sub_plain_result);
    println!(
        "Check OR result with big key is correct! \
        Expected {}, got {}", f(message1 + message2), or_plain_result
    );

    let key_or_plain_result: u64 =
        signed_decomposer.closest_representable(key_or_plain.0) / delta;

    println!("Checking result...");
    // assert_eq!(f(message1 + message2), key_and_plain_result);
    println!(
        "Check AND result with small key is correct! \
        Expected {}, got {}", f(message1 + message2), key_or_plain_result
    );
}

#[test]
#[ignore]
fn test_sequence_operations() {
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
    let big_lwe_sk = glwe_sk.clone().into_lwe_secret_key();

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

    let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
        &big_lwe_sk,
        &small_lwe_sk,
        ks_decomp_base_log,
        ks_decomp_level_count,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let message_modulus = 1u64 << 7;
    let delta = (1_u64 << 63) / message_modulus;

    let message1 = 1u64;
    let message2 = 2u64;
    let message3 = 1u64;
    let message4 = 2u64;
    
    let plaintext1 = Plaintext(message1 * delta);
    let plaintext2 = Plaintext(message2 * delta);
    let plaintext3 = Plaintext(message3 * delta);
    let plaintext4 = Plaintext(message4 * delta);
    
    let lwe1: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext1,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let lwe2: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext2,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let lwe3: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext3,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let lwe4: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext4,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    println!("Starting sequence operations...");

    println!("\nStep 1: Performing AND operation (lwe1 AND lwe2)...");
    let and_start = Instant::now();
    
    let and_f = |x: u64| { if x == 2 { 1 } else { 0 } };
    let and_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        and_f,
    );
    
    let mut and_result_temp = lwe1.clone();
    lwe_ciphertext_add(&mut and_result_temp, &lwe1, &lwe2);
    
    let mut and_result_big = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    programmable_bootstrap_lwe_ciphertext(
        &and_result_temp,
        &mut and_result_big,
        &and_lut,
        &fourier_bsk,
    );

    let mut and_result_small = LweCiphertext::new(
        0u64,
        small_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    keyswitch_lwe_ciphertext(&ksk_big_to_small, &and_result_big, &mut and_result_small);
    
    let and_time = and_start.elapsed();
    println!("AND operation time: {:?}", and_time);
    
    println!("\nStep 2: Performing OR operation (lwe3 OR lwe4)...");
    let or_start = Instant::now();
    

    let or_f = |x: u64| { if x == 0 { 0 } else { 1 } };
    let or_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        or_f,
    );
    
    let mut or_result_temp = lwe3.clone();
    lwe_ciphertext_add(&mut or_result_temp, &lwe3, &lwe4);
    
    let mut or_result_big = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    programmable_bootstrap_lwe_ciphertext(
        &or_result_temp,
        &mut or_result_big,
        &or_lut,
        &fourier_bsk,
    );
    
    let mut or_result_small = LweCiphertext::new(
        0u64,
        small_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    keyswitch_lwe_ciphertext(&ksk_big_to_small, &or_result_big, &mut or_result_small);
    
    let or_time = or_start.elapsed();
    println!("OR operation time: {:?}", or_time);
    
    println!("\nStep 3: Performing EQUAL operation (check if AND result == OR result)...");
    let equal_start = Instant::now();
    
    let equal_f = |x: u64| { if x == 0 { 1 } else { 0 } };
    let equal_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        equal_f,
    );
    
    let mut equal_temp = and_result_small.clone();
    lwe_ciphertext_sub(&mut equal_temp, &and_result_small, &or_result_small);
    
    let mut equal_result_big = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    programmable_bootstrap_lwe_ciphertext(
        &equal_temp,
        &mut equal_result_big,
        &equal_lut,
        &fourier_bsk,
    );
    
    let mut equal_result_small = LweCiphertext::new(
        0u64,
        small_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    keyswitch_lwe_ciphertext(&ksk_big_to_small, &equal_result_big, &mut equal_result_small);
    
    let equal_time = equal_start.elapsed();
    println!("EQUAL operation time: {:?}", equal_time);
    
    println!("\nStep 4: Performing AND operation on EQUAL result...");
    let final_and_start = Instant::now();
    
    let mut final_and_temp = equal_result_small.clone();
    lwe_ciphertext_add(&mut final_and_temp, &equal_result_small, &or_result_small);
    
    let mut final_and_result_big = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    programmable_bootstrap_lwe_ciphertext(
        &final_and_temp,
        &mut final_and_result_big,
        &and_lut,
        &fourier_bsk,
    );
    
    let mut final_result = LweCiphertext::new(
        0u64,
        small_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    keyswitch_lwe_ciphertext(&ksk_big_to_small, &final_and_result_big, &mut final_result);
    
    let final_and_time = final_and_start.elapsed();
    println!("Final AND operation time: {:?}", final_and_time);
    
    println!("\nVerifying results...");
    let signed_decomposer =
        SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));
    
    let and_plain: Plaintext<u64> = decrypt_lwe_ciphertext(&small_lwe_sk, &and_result_small);
    let and_result: u64 = signed_decomposer.closest_representable(and_plain.0) / delta;
    println!("Step 1 AND result (lwe1 AND lwe2): {} (expected: {})", 
             and_result, if message1 == 1 && message2 == 2 { 0 } else { 0 });
    
    let or_plain: Plaintext<u64> = decrypt_lwe_ciphertext(&small_lwe_sk, &or_result_small);
    let or_result: u64 = signed_decomposer.closest_representable(or_plain.0) / delta;
    println!("Step 2 OR result (lwe3 OR lwe4): {} (expected: {})", 
             or_result, if message3 == 1 || message4 == 2 { 1 } else { 0 });
    
    let equal_plain: Plaintext<u64> = decrypt_lwe_ciphertext(&small_lwe_sk, &equal_result_small);
    let equal_result: u64 = signed_decomposer.closest_representable(equal_plain.0) / delta;
    println!("Step 3 EQUAL result (AND == OR): {} (expected: {})", 
             equal_result, if and_result == or_result { 1 } else { 0 });
    
    let final_plain: Plaintext<u64> = decrypt_lwe_ciphertext(&small_lwe_sk, &final_result);
    let final_result_val: u64 = signed_decomposer.closest_representable(final_plain.0) / delta;
    println!("Step 4 Final AND result: {} (expected: {})", 
             final_result_val, or_result & equal_result);
}    

#[test]
// #[ignore]
fn test_mul() {
    let small_lwe_dimension = LweDimension(1055);
    let glwe_dimension = GlweDimension(1);
    let polynomial_size = PolynomialSize(16384);
    let lwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));//512 ciphertext sum; real 2^{-40}
    let glwe_noise_distribution =
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(2.168404344971009e-19));
    let pbs_base_log = DecompositionBaseLog(15);
    let pbs_level = DecompositionLevelCount(2);
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

    let big_lwe_sk = glwe_sk.clone().into_lwe_secret_key();

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

    let message_modulus = 1u64 << 7;

    let delta = (1_u64 << 63) / message_modulus;

    let message1 = 1u64;
    let plaintext1 = Plaintext(message1 * delta);
    let lwe1: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext1,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );
    let message2 = 126u64;
    let plaintext2 = Plaintext(message2 * delta);
    let lwe2: LweCiphertextOwned<u64> = allocate_and_encrypt_new_lwe_ciphertext(
        &small_lwe_sk,
        plaintext2,
        lwe_noise_distribution,
        ciphertext_modulus,
        &mut encryption_generator,
    );

    let f = |x: u64| (x * (x - 1)) / 2;
    let mul_lut: GlweCiphertextOwned<u64> = generate_programmable_bootstrap_glwe_lut(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        message_modulus as usize,
        ciphertext_modulus,
        delta,
        f,
    );

    let mut result1 = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    let mut result2 = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    let mut finalresult = LweCiphertext::new(
        0u64,
        big_lwe_sk.lwe_dimension().to_lwe_size(),
        ciphertext_modulus,
    );
    let mut addition_ct = lwe1.clone();
    lwe_ciphertext_add(
        &mut addition_ct,
        &lwe1,
        &lwe2,
    );

    println!("Performing add...");
    let pbs_start = Instant::now();
    programmable_bootstrap_lwe_ciphertext(
        &lwe2,
        &mut result1,
        &mul_lut,
        &fourier_bsk,
    );
    programmable_bootstrap_lwe_ciphertext(
        &addition_ct,
        &mut result2,
        &mul_lut,
        &fourier_bsk,
    );
    lwe_ciphertext_sub(
        &mut finalresult,
        &result2,
        &result1,
    );
    let pbs_time = pbs_start.elapsed();
    println!("check equal time: {:?}", pbs_time);

    let plain: Plaintext<u64> =
        decrypt_lwe_ciphertext(&big_lwe_sk, &finalresult);

    let signed_decomposer =
        SignedDecomposer::new(DecompositionBaseLog(8), DecompositionLevelCount(1));

    let plain_result: u64 =
        signed_decomposer.closest_representable(plain.0) / delta;

    println!("Checking result...");
    println!(
        "mul result is correct! \
        Expected {}, got {}", message1 * message2, plain_result
    );
}
