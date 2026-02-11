#![allow(unused_imports)]

use std::time::{Instant};
use tfhe::core_crypto::prelude::*;
use tfhe::shortint::prelude::*;
use tfhe::shortint::parameters::v1_5::*;
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint8};
use tfhe::shortint::parameters::DynamicDistribution;

#[test]
#[ignore]
fn test_basic() {
    let mut param = V1_5_PARAM_MESSAGE_7_CARRY_0_KS_PBS_GAUSSIAN_2M128;
    param.lwe_noise_distribution = 
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.000000000465661287));
    let (client_key, server_key) = gen_keys(param);

    let msg1 = 96u64;
    let msg2 = 1u64;
    let msg3 = 0u64;

    let ct_1 = client_key.encrypt(msg1);
    let ct_2 = client_key.encrypt(msg2);
    let ct_3 = client_key.encrypt(msg3);
    let mut ct_4 = ct_3.clone();

    let add_start = Instant::now();
    for _i in 0..96{
        server_key.unchecked_add_assign(&mut ct_4, &ct_2);
    }
    for _i in 0..416{
        server_key.unchecked_add_assign(&mut ct_4, &ct_3);
    }
    let add_time = add_start.elapsed();
    println!("add time: {:?}", add_time);

    let equal_start = Instant::now();
    let ctbool = server_key.unchecked_equal(&ct_1, &ct_4);
    let equal_time = equal_start.elapsed();
    println!("equal time: {:?}", equal_time);

    // let sel_start = Instant::now();
    // let encrypted_res = server_key.unchecked_bitand(&ctbool, &ctbool);
    // let sel_time = sel_start.elapsed();
    // println!("and time: {:?}", sel_time);

    let output = client_key.decrypt(&ct_4);
    println!("add expected {}, found {}", 96, output);
    let result = client_key.decrypt(&ctbool);
    println!("equal expected {}, found {}", true, result);
    // let selres = client_key.decrypt(&encrypted_res);
    // println!("and expected {}, found {}", 0, selres);
}

#[test]
#[ignore]
fn test_uint8() {
    let (client_key, server_key) = generate_keys(ConfigBuilder::default());
    set_server_key(server_key);

    let a = FheUint8::encrypt(1u16, &client_key);
    let b = FheUint8::encrypt(2u16, &client_key);
    let c = FheUint8::encrypt(3u16, &client_key);

    let add_start = Instant::now();
    let result = FheUint8::sum([&a, &b, &c]);
    let decrypted: u16 = result.decrypt(&client_key);
    assert_eq!(decrypted, 1u16 + 2 + 3);
    let add_time = add_start.elapsed();
    println!("add time: {:?}", add_time);

    // Or
    let add_start = Instant::now();
    let result = [&a, &b, &c].into_iter().sum::<FheUint8>();
    let decrypted: u16 = result.decrypt(&client_key);
    assert_eq!(decrypted, 1u16 + 2 + 3);
    let add_time = add_start.elapsed();
    println!("add time: {:?}", add_time);
}