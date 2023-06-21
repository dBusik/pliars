use openssl::sha::sha256;

// Function determining the number of hashes which a machine can compute in a second.
// This will be used to determine the difficulty of the proof of work.
pub fn find_my_hashrate() -> usize {
    let mut nonce: i32 = 0;
    let mut count = 0;
    let mut total_time = std::time::Duration::new(0, 0);
    loop {
        let start = std::time::Instant::now();
        _ = sha256(&nonce.to_be_bytes());
        let elapsed = start.elapsed();
        
        total_time += elapsed;
        
        nonce += 1;
        count += 1;
        if total_time.as_secs() >= 1 {
            break;
        }
    }
    count
}

pub fn difficulty_from_secs(difficulty_in_secs: f64, hashrate: f64) -> Vec<u8> {
    let difficulty = (2.0f64.powi(256) - 1.0) / (difficulty_in_secs * hashrate);
    let difficulty = rug::Float::with_val(256, difficulty);
    let difficulty = difficulty.trunc().to_integer().unwrap();
    println!("Difficulty: {:?}", difficulty);
    
    let mut difficulty = difficulty.to_digits::<u8>(rug::integer::Order::MsfBe);
    while difficulty.len() < 32 {
        // Pad the difficulty with zeros if it is shorter that the length of the ouput
        // of the hash function (which in this case is 256 bits since we use sha256)
        difficulty.insert(0, 0);
    }

    difficulty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_my_hashrate() {
        let hashrate = find_my_hashrate();
        println!("My hashrate: {} hashes/s", hashrate);
        assert!(hashrate > 0);
    }

    #[test]
    fn test_u8_cmp() {
        use rand::Rng;
        // Generate two random long integers as byte arrays and compare them.
        let mut rng = rand::thread_rng();
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        rng.fill(&mut a[..]);
        rng.fill(&mut b[..]);
        let a = a.as_ref();
        let b = b.as_ref();
        let cmp = a.cmp(b);
        println!("a: {:?}\nb: {:?}\ncmp: {:?}", a, b, cmp);
        // Call the exorcist if this assert fails
        assert!(cmp == std::cmp::Ordering::Less || cmp == std::cmp::Ordering::Equal || cmp == std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_block_comparison() {
        use crate::blockchain::block::Block;
        
        let block1 = Block::genesis();
        let block2 = Block::genesis();

        assert_eq!(block1, block2);
    }

    #[test]
    fn test_hash_value() {
        use crate::blockchain::pow;

        let block = crate::blockchain::block::Block::new(
            1,
            "mDgKLzrjHxk/fpBrKby9puNvbVMVunf44ns3uj3d9UY=".to_string(),
            0,
            Vec::new(),
            "6339200808718768504".to_string(),
            Vec::new(),
            vec![0, 0, 0, 48, 80, 236, 231, 14, 175, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        let hash = pow::get_token_from_block(&block);

        // Assert that hash of the block above is smaller than difficulty in it and print values of the hash and of difficulty
        println!("hash: {:?}\ndifficulty: {:?}", hash, block.difficulty);
        assert!(hash.as_slice() < block.difficulty.as_slice());
    }
}