pub mod chain;
pub mod block;
pub mod pow;

#[cfg(test)]
mod test {

    #[test]
    fn test_compare_token_getters() {
        use super::pow::{get_new_token, get_token_from_block};

        let nonce = 6339200808718768504;
        let block = crate::blockchain::block::Block::new(
            1,
            "mDgKLzrjHxk/fpBrKby9puNvbVMVunf44ns3uj3d9UY=".to_string(),
            Vec::new(),
            nonce.to_string(),
            Vec::new(),
            vec![0, 0, 0, 48, 80, 236, 231, 14, 175, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        let token = get_new_token(&block, nonce);
        let token2 = get_token_from_block(&block);

        println!("token: {:?}\ntoken2: {:?}", token, token2);
        assert_eq!(token, token2);
    }
}