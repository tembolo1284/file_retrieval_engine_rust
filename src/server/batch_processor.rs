// src/server/batch_processor.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use crate::server::index_store::IndexStore;

const NUM_SHARDS: usize = 256;
const MIN_WORD_LENGTH: usize = 3;

pub struct BatchIndexProcessor {
    store: Arc<IndexStore>,
    batch_size: usize,
    current_batches: Vec<HashMap<String, i64>>,
    document_number: i64,
    word_buffer: String,
    freq_map: HashMap<String, i64>,
}

impl BatchIndexProcessor {
    pub fn new(store: Arc<IndexStore>) -> Self {
        let mut current_batches = Vec::with_capacity(NUM_SHARDS);
        for _ in 0..NUM_SHARDS {
            current_batches.push(HashMap::with_capacity(1000));
        }

        Self {
            store,
            batch_size: 10000,
            current_batches,
            document_number: 0,
            word_buffer: String::with_capacity(64),
            freq_map: HashMap::with_capacity(1000),
        }
    }

    pub fn process_document(&mut self, content: &str, path: &str) -> i64 {
        let doc_num = self.store.put_document(path.to_string());
        self.document_number = doc_num;
        self.freq_map.clear();
        self.word_buffer.clear();

        let mut in_word = false;
        
        // Process content character by character
        for c in content.chars() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                self.word_buffer.push(c);
                in_word = true;
            } else if in_word {
                self.process_word();
                in_word = false;
            }
        }

        // Process last word if any
        if in_word {
            self.process_word();
        }

        // Create a temporary copy of the word frequencies
        let tmp_freq_map = std::mem::take(&mut self.freq_map);
        
        // Update batches with word frequencies
        for (word, freq) in tmp_freq_map {
            let shard = self.get_shard_index(&word);
            self.current_batches[shard].insert(word, freq);
            
            if self.current_batches[shard].len() >= self.batch_size {
                self.flush_shard(shard);
            }
        }

        doc_num
    }

    fn process_word(&mut self) {
        if self.word_buffer.len() >= MIN_WORD_LENGTH {
            *self.freq_map.entry(self.word_buffer.clone()).or_insert(0) += 1;
        }
        self.word_buffer.clear();
    }

    fn get_shard_index(&self, word: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        word.hash(&mut hasher);
        (hasher.finish() as usize) % NUM_SHARDS
    }

    fn flush_shard(&mut self, shard: usize) {
        if self.current_batches[shard].is_empty() {
            return;
        }

        let mut update = HashMap::new();
        std::mem::swap(&mut update, &mut self.current_batches[shard]);
        
        self.store.batch_update_index(vec![(self.document_number, update)]);
    }

    pub fn flush_all(&mut self) {
        for shard in 0..NUM_SHARDS {
            if !self.current_batches[shard].is_empty() {
                self.flush_shard(shard);
            }
        }
    }
}
