#![allow(unused_imports)]

use std::time::{Instant};
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint10, FheAsciiString, ClearString};
use tfhe::shortint::prelude::*;
use tfhe::shortint::parameters::v1_5::*;

#[test]
// #[ignore]
fn test_string() {
    let config = ConfigBuilder::default()
        .use_custom_parameters(V1_5_PARAM_MESSAGE_2_CARRY_2_KS_PBS_GAUSSIAN_2M128);
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key.clone());
    
    let string1 = FheAsciiString::try_encrypt_with_fixed_sized("history", 30usize, &client_key).unwrap();
    let string2 = ClearString::new("history".to_string());
    let conversion_start = Instant::now();
    let is_eq = string1.eq(&string2);
    let boo = &is_eq | &is_eq;
    let conversion_time = conversion_start.elapsed();
    println!("compare time: {:?}", conversion_time);

    let data = FheUint10::encrypt(30u16, &client_key);
    let conversion_start = Instant::now();
    let _ct = boo.select(&data, &data);
    let _sum = &data + &data;
    let conversion_time = conversion_start.elapsed();
    println!("select time: {:?}", conversion_time);

    let serialized_string = bincode::serialize(&string1).unwrap();
    let serialized_size_string = serialized_string.len();
    println!("FheAsciiString serialized size: {} bytes", serialized_size_string);
    let serialized_bool = bincode::serialize(&is_eq).unwrap();
    let serialized_size_bool = serialized_bool.len();
    println!("Compare result serialized size: {} bytes", serialized_size_bool);

    println!("Compare result is {}", is_eq.decrypt(&client_key));
}