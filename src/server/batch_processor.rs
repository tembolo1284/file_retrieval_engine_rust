// src/server/batch_processor.rs

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::server::index_store::IndexStore;

const NUM_SHARDS: usize = 256;
const MIN_WORD_LENGTH: usize = 3;

pub struct BatchIndexProcessor {
    store: Arc<IndexStore>,
    batch_size: usize,
    current_batches: Vec<Vec<(String, i64)>>,
    document_buffer: Vec<(i64, String)>,
    word_buffer: String,
    freq_map: HashMap<String, i64>,
}

impl BatchIndexProcessor {
    pub fn new(store: Arc<IndexStore>) -> Self {
        Self {
            store,
            batch_size: 10000,
            current_batches: vec![Vec::with_capacity(10000); NUM_SHARDS],
            document_buffer: Vec::with_capacity(1000),
            word_buffer: String::with_capacity(64),
            freq_map: HashMap::with_capacity(1000),
        }
    }

    pub fn process_document(&mut self, content: &str, path: &str) -> i64 {
        let doc_num = self.store.put_document(path.to_string());
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

        // Update batches with word frequencies
        for (word, freq) in self.freq_map.drain() {
            let shard = self.get_shard_index(&word);
            self.current_batches[shard].push((word, freq));
            
            if self.current_batches[shard].len() >= self.batch_size {
                self.flush_shard(shard, doc_num);
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

    fn flush_shard(&mut self, shard: usize, doc_num: i64) {
        if self.current_batches[shard].is_empty() {
            return;
        }

        let mut word_freqs = HashMap::new();
        for (word, freq) in self.current_batches[shard].drain(..) {
            word_freqs.insert(word, freq);
        }

        self.store.batch_update_index(vec![(doc_num, word_freqs)]);
    }

    pub fn flush_all(&mut self) {
        let mut updates = Vec::new();
        
        for doc_info in self.document_buffer.drain(..) {
            updates.push(doc_info);
        }

        if !updates.is_empty() {
            self.store.batch_update_index(updates);
        }

        for shard in 0..NUM_SHARDS {
            if !self.current_batches[shard].is_empty() {
                self.flush_shard(shard, 0);
            }
        }
    }
}
