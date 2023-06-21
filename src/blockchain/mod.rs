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

    #[test]
    fn test_sidelink_deriviation() {
        use super::block::Block;

        let num_sidelinks = 0;
        let block = Block::new(
            10,
            "mDgKLzrjHxk/fpBrKby9puNvbVMVunf44ns3uj3d9UY=".to_string(),
            Vec::new(),
            "6339200808718768504".to_string(),
            Vec::new(),
            vec![0, 0, 0, 48, 80, 236, 231, 14, 175, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        let sidelinks = block.derive_sidelink_indices(num_sidelinks);
        let sidelinks_once_more = block.derive_sidelink_indices(num_sidelinks);
        println!("sidelinks: {:?}", sidelinks);

        assert_eq!(sidelinks, sidelinks_once_more);
    }

    mod file_operations {
        /*
         This is samepl file's contents:
            {"idx":1,"previous_block_hash":"00000000000000000000000000000000","validation_sidelinks":[],"pow":"","timestamp":0,"records":[],"difficulty":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":2,"previous_block_hash":"mDgKLzrjHxk/fpBrKby9puNvbVMVunf44ns3uj3d9UY=","validation_sidelinks":[],"pow":"18358677514904226553","timestamp":1687109269,"records":[{"idx":[2,1],"timestamp":1687109265,"data":"dupa123","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"}],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":3,"previous_block_hash":"fc+EskuYUs1uDVJI6aciGbZ2cXXIYE1n12Iljj/23HI=","validation_sidelinks":[],"pow":"7074847124089670442","timestamp":1687109279,"records":[],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":4,"previous_block_hash":"DCK3m1YJS/j7ogJvXvDE7AD08WwJbC+xlMuxmx49OR8=","validation_sidelinks":[],"pow":"12749254611793627068","timestamp":1687109324,"records":[],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":5,"previous_block_hash":"08Aj//EPRnp8/pQJrJoH2tUhOBCrxLrr8+YhPE/5In8=","validation_sidelinks":[],"pow":"6889260309631619081","timestamp":1687109338,"records":[],"difficulty":[0,0,0,88,158,236,227,121,51,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":6,"previous_block_hash":"XC4S0SPB5V6mYaPAPUHIJp3PO9DJnuFVEipyE97h9rQ=","validation_sidelinks":[],"pow":"6985210315240570000","timestamp":1687109339,"records":[],"difficulty":[0,0,0,88,158,236,227,121,51,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":7,"previous_block_hash":"wgqDmhPY4MJqtclsKBJ/jEwfbVCpXdubFMjSFHYWRrE=","validation_sidelinks":[],"pow":"15860608882751981174","timestamp":1687109341,"records":[],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":8,"previous_block_hash":"gjK1Ti8fbvxAjt79wQ5horEpLuRV0xwY6aEZOIOzEMM=","validation_sidelinks":[],"pow":"1443274565602880904","timestamp":1687109357,"records":[{"idx":[8,1],"timestamp":1687109370,"data":"asdsads","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"},{"idx":[8,2],"timestamp":1687109373,"data":"abba","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"},{"idx":[8,3],"timestamp":1687109376,"data":"pamamama","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"}],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
            {"idx":9,"previous_block_hash":"bY7SoGgUHF07oGgmZyvyX68fUIW0OWnSvaQEi7yLwTQ=","validation_sidelinks":[],"pow":"3530195229330195409","timestamp":1687109385,"records":[{"idx":[9,1],"timestamp":1687109381,"data":"papapa","author_peer_id":"12D3KooWFmb524g674gmQnuFu9CME4e6yXLgb3hpKLuagPNUhBQj"}],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}    
         */

        const SAMPLE_FILE: &str = r##"{"idx":1,"previous_block_hash":"00000000000000000000000000000000","validation_sidelinks":[],"pow":"","timestamp":0,"records":[],"difficulty":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":2,"previous_block_hash":"mDgKLzrjHxk/fpBrKby9puNvbVMVunf44ns3uj3d9UY=","validation_sidelinks":[],"pow":"18358677514904226553","timestamp":1687109269,"records":[{"idx":[2,1],"timestamp":1687109265,"data":"dupa123","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"}],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":3,"previous_block_hash":"fc+EskuYUs1uDVJI6aciGbZ2cXXIYE1n12Iljj/23HI=","validation_sidelinks":[],"pow":"7074847124089670442","timestamp":1687109279,"records":[],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":4,"previous_block_hash":"DCK3m1YJS/j7ogJvXvDE7AD08WwJbC+xlMuxmx49OR8=","validation_sidelinks":[],"pow":"12749254611793627068","timestamp":1687109324,"records":[],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":5,"previous_block_hash":"08Aj//EPRnp8/pQJrJoH2tUhOBCrxLrr8+YhPE/5In8=","validation_sidelinks":[],"pow":"6889260309631619081","timestamp":1687109338,"records":[],"difficulty":[0,0,0,88,158,236,227,121,51,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":6,"previous_block_hash":"XC4S0SPB5V6mYaPAPUHIJp3PO9DJnuFVEipyE97h9rQ=","validation_sidelinks":[],"pow":"6985210315240570000","timestamp":1687109339,"records":[],"difficulty":[0,0,0,88,158,236,227,121,51,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":7,"previous_block_hash":"wgqDmhPY4MJqtclsKBJ/jEwfbVCpXdubFMjSFHYWRrE=","validation_sidelinks":[],"pow":"15860608882751981174","timestamp":1687109341,"records":[],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":8,"previous_block_hash":"gjK1Ti8fbvxAjt79wQ5horEpLuRV0xwY6aEZOIOzEMM=","validation_sidelinks":[],"pow":"1443274565602880904","timestamp":1687109357,"records":[{"idx":[8,1],"timestamp":1687109370,"data":"asdsads","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"},{"idx":[8,2],"timestamp":1687109373,"data":"abba","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"},{"idx":[8,3],"timestamp":1687109376,"data":"pamamama","author_peer_id":"12D3KooWDYEAgpLJzm289WwxsB3hf9H9b7rUrepvbgXmaGa1zXsZ"}],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}
{"idx":9,"previous_block_hash":"bY7SoGgUHF07oGgmZyvyX68fUIW0OWnSvaQEi7yLwTQ=","validation_sidelinks":[],"pow":"3530195229330195409","timestamp":1687109385,"records":[{"idx":[9,1],"timestamp":1687109381,"data":"papapa","author_peer_id":"12D3KooWFmb524g674gmQnuFu9CME4e6yXLgb3hpKLuagPNUhBQj"}],"difficulty":[0,0,0,63,218,110,249,240,181,42,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"##;

        
        #[test]
        fn test_get_last_n_blocks_from_file() {
            use crate::blockchain::chain::Chain;
            use tempfile::NamedTempFile;
            use std::io::Write;

            let mut file = NamedTempFile::new().unwrap();
            file.write_all(SAMPLE_FILE.as_bytes()).unwrap();
            let filename_as_str =  file.path().to_str().unwrap().clone();

            let file_contents = std::fs::read_to_string(filename_as_str).unwrap();
            println!("file_contents: {:?}", file_contents);

            let blocks_to_read = 3;
            let blocks = Chain::get_last_n_blocks_from_file(
                blocks_to_read,
                filename_as_str);
            
            println!("blocks: {:?}", blocks);
        }

        #[test]
        fn test_get_chosen_set_of_blocks_from_file() {
            use crate::blockchain::chain::Chain;
            use tempfile::NamedTempFile;
            use std::io::Write;

            let mut file = NamedTempFile::new().unwrap();
            file.write_all(SAMPLE_FILE.as_bytes()).unwrap();
            let filename_as_str =  file.path().to_str().unwrap().clone();

            let file_contents = std::fs::read_to_string(filename_as_str).unwrap();
            println!("file_contents: {:?}", file_contents);

            let blocks_to_read = vec![0, 1, 3, 8, 9, 120];
            let blocks = Chain::get_blocks_by_indices_from_file(
                blocks_to_read,
                filename_as_str);
            
            // Won't print blocks with indicees 0 and 120 since they do not exist
            println!("blocks: {:?}", blocks);
        }
    }
}