use std::{
    cell::{Cell, RefCell},
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    sync::RwLock,
};

pub const DEFAULT_SIZE: u16 = 2048;
pub const DEFAULT_HASH_LOOP: u16 = 26;

pub struct BloomFilter {
    pub size: Cell<u16>,
    pub hash_loop: Cell<u16>,
    is_null: RwLock<Cell<bool>>,
    pub bitmap: RwLock<RefCell<Box<Vec<u8>>>>,
}

impl BloomFilter {
    fn bits_to_bytes(bits: u16) -> u16 {
        (bits + 7) / 8 
    }
    
    fn get_bit_value(bitmap: &[u8], index: u16) -> bool {
        let byte_index = index / 8;
        let bit_offset = index % 8;
        
        (bitmap[byte_index as usize] >> bit_offset) & 1 == 1
    }
    
    fn set_bit_value(bitmap: &mut [u8], index: u16) {
        let byte_index = index / 8;
        let bit_offset = index % 8;

        bitmap[byte_index as usize] |= 1 << bit_offset;
    }

    pub fn new() -> Self {
        let byte_count = Self::bits_to_bytes(DEFAULT_SIZE);
        let bitmap: Vec<u8> = vec![0; byte_count.into()];
        
        Self {
            size: Cell::new(DEFAULT_SIZE),
            hash_loop: Cell::new(DEFAULT_HASH_LOOP),
            is_null: RwLock::new(Cell::new(true)),
            bitmap: RwLock::new(RefCell::new(Box::new(bitmap))),
        }
    }

    pub fn set_size(self, size: u16) -> Self {
        if let Ok(is_null) = self.is_null.read() {
            if is_null.get() {
                self.size.set(size);
                let byte_count = Self::bits_to_bytes(size);
                
                if let Ok(mut bitmap) = self.bitmap.write() {
                    let bitmap_cell = bitmap.get_mut();
                    *bitmap_cell = Box::new(vec![0; byte_count.into()]);
                }
            } else {
                println!("The modification is invalid because the bitmap already has data");
            }
        }
        self
    }

    pub fn set_hash_loop(self, hash_loop: u16) -> Self {
        if let Ok(is_null) = self.is_null.read() {
            if is_null.get() {
                if hash_loop > 0 {
                    self.hash_loop.set(hash_loop);
                } else {
                    println!("Warning: hash_loop must be > 0, using default");
                    self.hash_loop.set(DEFAULT_HASH_LOOP);
                }
            } else {
                println!("The modification is invalid because the bitmap already has data");
            }
        }
        self
    }

    pub fn insert(&self, key: &str) {
        if let Ok(is_null_guard) = self.is_null.write() {
            is_null_guard.set(false);
        }
        
        let indices = self.hash(key);
        self.insert_bitmap(indices);
    }

    #[allow(dead_code)]
    pub fn contains(&self, key: &str) -> bool {
        let indices = self.hash(key);
        self.contains_bitmap(indices)
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        let byte_count = Self::bits_to_bytes(self.size.get());
        
        if let Ok(bitmap) = self.bitmap.write() {
            if let Ok(mut bitmap_cell) = bitmap.try_borrow_mut() {
                *bitmap_cell = Box::new(vec![0; byte_count.into()]);
                
                if let Ok(is_null_guard) = self.is_null.write() {
                    is_null_guard.set(true);
                }
            }
        }
    }

    pub fn get_bit(&self, index: u16) -> Option<bool> {
        if index >= self.size.get() {
            return None;
        }
        
        if let Ok(bitmap) = self.bitmap.read() {
            let bitmap_tmp = bitmap.borrow();
            Some(Self::get_bit_value(&bitmap_tmp, index))
        } else {
            None
        }
    }

    pub fn get_all_set_bits_indices(&self) -> Vec<u16> {
        let mut indices = Vec::new();
        let bitmap_guard = self.bitmap.read().unwrap();
        let bitmap_cell = bitmap_guard.borrow();
        let bitmap = &**bitmap_cell;
        let total_bits = self.size.get() as usize;
        
        for (byte_index, &byte) in bitmap.iter().enumerate() {
            if byte == 0 {
                continue;
            }
            
            for bit_offset in 0..8 {
                let bit_index = byte_index * 8 + bit_offset;
                if bit_index >= total_bits {
                    break;
                }
                
                if (byte >> bit_offset) & 1 == 1 {
                    indices.push(bit_index as u16);
                }
            }
        }
        
        indices
    }

    fn insert_bitmap(&self, indices: Vec<u16>) {
        let size = self.size.get();
        
        let valid_indices: Vec<u16> = indices
            .into_iter()
            .filter(|&index| index < size)
            .collect();
        
        if let Ok(bitmap) = self.bitmap.write() {
            if let Ok(mut bitmap_cell) = bitmap.try_borrow_mut() {
                for index in valid_indices {
                    Self::set_bit_value(&mut bitmap_cell, index);
                }
            }
        }
    }

    #[allow(dead_code)]
    fn contains_bitmap(&self, indices: Vec<u16>) -> bool {
        let size = self.size.get();
        
        for &index in &indices {
            if index >= size {
                return false;
            }
        }
        
        if let Ok(bitmap) = self.bitmap.read() {
            let bitmap_tmp = bitmap.borrow();
            
            for index in indices {
                if !Self::get_bit_value(&bitmap_tmp, index) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn hash(&self, key: &str) -> Vec<u16> {
        let hash_loop = self.hash_loop.get();
        let size = self.size.get();
        let mut result = Vec::with_capacity(hash_loop.into());
        
        let mut hasher1 = DefaultHasher::new();
        hasher1.write(key.as_bytes());
        let hash1 = hasher1.finish() as u16;
        
        let mut hasher2 = DefaultHasher::new();
        hasher2.write(&[1]);
        hasher2.write(key.as_bytes());
        let hash2 = hasher2.finish() as u16;
        
        for i in 0..hash_loop {
            let index = (hash1.wrapping_add(i.wrapping_mul(hash2))) % size;
            result.push(index);
        }
        
        result
    }
}

impl Clone for BloomFilter {
    fn clone(&self) -> Self {
        let new_filter = BloomFilter::new()
            .set_size(self.size.get())
            .set_hash_loop(self.hash_loop.get());
        
        if let Ok(is_null) = self.is_null.read() {
            if !is_null.get() {
                if let Ok(bitmap) = self.bitmap.read() {
                    let bitmap_cell = bitmap.borrow();
                    let bitmap_data = &**bitmap_cell;
                    
                    if let Ok(new_bitmap) = new_filter.bitmap.write() {
                        if let Ok(mut new_bitmap_cell) = new_bitmap.try_borrow_mut() {
                            **new_bitmap_cell = bitmap_data.clone();

                            if let Ok(new_is_null) = new_filter.is_null.write() {
                                new_is_null.set(false);
                            }
                        }
                    }
                }
            }
        }
        
        new_filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    // #[ignore]
    fn test_bit_storage() {
        let filter = BloomFilter::new().set_size(64);
        
        if let Ok(bitmap) = filter.bitmap.read() {
            let bitmap_tmp = bitmap.borrow();
            assert_eq!(bitmap_tmp.len(), 8); 
        }
        
        filter.insert_bitmap(vec![0, 1, 7, 8, 15, 63]);
        
        assert_eq!(filter.get_bit(0), Some(true));
        assert_eq!(filter.get_bit(1), Some(true));
        assert_eq!(filter.get_bit(7), Some(true));
        assert_eq!(filter.get_bit(8), Some(true));
        assert_eq!(filter.get_bit(15), Some(true));
        assert_eq!(filter.get_bit(63), Some(true));

        assert_eq!(filter.get_bit(2), Some(false));
        assert_eq!(filter.get_bit(62), Some(false));
        
        println!("Bit storage test passed!");
    }

    #[test]
    // #[ignore]
    fn test_insert_and_contains() {
        let filter = BloomFilter::new().set_size(1024).set_hash_loop(5);
        
        filter.insert("hello");
        filter.insert("world");
        
        assert!(filter.contains("hello"));
        assert!(filter.contains("world"));
        assert!(!filter.contains("not_exists"));
        
        println!("Insert and contains test passed!");
    }
    
    #[test]
    // #[ignore]
    fn test_clear() {
        let filter = BloomFilter::new().set_size(256);
        
        filter.insert("test1");
        filter.insert("test2");
        
        assert!(filter.contains("test1"));
        assert!(filter.contains("test2"));
        
        filter.clear();
        
        assert!(!filter.contains("test1"));
        assert!(!filter.contains("test2"));
        
        println!("Clear test passed!");
    }

    #[test]
    // #[ignore]
    fn test_get_all_set_bits() {
        let filter = BloomFilter::new().set_size(64);
        
        filter.insert_bitmap(vec![0, 1, 7, 8, 15, 63]);
        let set_bits = filter.get_all_set_bits_indices();
    
        println!("Set bits: {:?}", set_bits);
        assert_eq!(set_bits.len(), 6);
        assert_eq!(set_bits[0], 0);
        assert_eq!(set_bits[1], 1);
        assert_eq!(set_bits[2], 7);
        assert_eq!(set_bits[3], 8);
        assert_eq!(set_bits[4], 15);
        assert_eq!(set_bits[5], 63);
        println!("get_all_set_bits_indices is ok!");
    }
}
