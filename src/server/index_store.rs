// src/server/index_store.rs

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicI64, Ordering};

const NUM_SHARDS: usize = 256;

#[derive(Debug, Clone)]
pub struct DocFreqPair {
    pub document_number: i64,
    pub word_frequency: i64,
}

pub struct IndexStore {
    // Document mapping shards
    doc_map_shards: Box<[RwLock<DocumentShard>; NUM_SHARDS]>,
    // Term index shards
    term_index_shards: Box<[RwLock<TermShard>; NUM_SHARDS]>,
    next_document_number: AtomicI64,
}

struct DocumentShard {
    path_to_number: HashMap<String, i64>,
    number_to_path: HashMap<i64, String>,
}

struct TermShard {
    term_index: HashMap<String, Vec<DocFreqPair>>,
}

impl IndexStore {
    pub fn new() -> Self {
        // Initialize shards
        let doc_map_shards = Box::new(array_init::array_init(|_| {
            RwLock::new(DocumentShard {
                path_to_number: HashMap::new(),
                number_to_path: HashMap::new(),
            })
        }));

        let term_index_shards = Box::new(array_init::array_init(|_| {
            RwLock::new(TermShard {
                term_index: HashMap::new(),
            })
        }));

        IndexStore {
            doc_map_shards,
            term_index_shards,
            next_document_number: AtomicI64::new(1),
        }
    }

    fn get_doc_map_shard_index(&self, path: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        (hasher.finish() as usize) % NUM_SHARDS
    }

    fn get_term_shard_index(&self, term: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        term.hash(&mut hasher);
        let hash = hasher.finish();
        // println!("Getting shard for term '{}', hash: {}, shard: {}", 
                // term, hash, (hash as usize) % NUM_SHARDS);
        (hash as usize) % NUM_SHARDS
    }

    pub fn put_document(&self, document_path: String) -> i64 {
        let shard_idx = self.get_doc_map_shard_index(&document_path);
        let mut shard = self.doc_map_shards[shard_idx].write();

        if let Some(&doc_num) = shard.path_to_number.get(&document_path) {
            return doc_num;
        }

        let doc_num = self.next_document_number.fetch_add(1, Ordering::SeqCst);
        shard.path_to_number.insert(document_path.clone(), doc_num);
        shard.number_to_path.insert(doc_num, document_path);

        doc_num
    }

    pub fn get_document(&self, document_number: i64) -> Option<String> {
        // Search all shards since we don't know which one contains the document
        for shard in self.doc_map_shards.iter() {
            let shard = shard.read();
            if let Some(path) = shard.number_to_path.get(&document_number) {
                return Some(path.clone());
            }
        }
        None
    }

    pub fn update_index(&self, document_number: i64, word_frequencies: HashMap<String, i64>) {
        // println!("Updating index for doc {}, words: {}", document_number, word_frequencies.len());

        let mut sharded_updates: Vec<HashMap<String, i64>> = vec![HashMap::new(); NUM_SHARDS];

        for (word, freq) in word_frequencies {
            // println!("Processing word: {}", word);
            let shard = self.get_term_shard_index(&word);
            // println!("Assigned to shard: {}", shard);
            sharded_updates[shard].insert(word, freq);
        }

        // Update each shard independently
        for (shard_idx, updates) in sharded_updates.into_iter().enumerate() {
            if !updates.is_empty() {
                let mut shard = self.term_index_shards[shard_idx].write();
                for (word, freq) in updates {
                    // println!("Updating shard {} with word '{}', freq {}", shard_idx, word, freq);
                    let postings = shard.term_index.entry(word.clone()).or_insert_with(Vec::new);
                    if let Some(existing) = postings.iter_mut()
                        .find(|p| p.document_number == document_number) {
                        existing.word_frequency = freq;
                    } else {
                        postings.push(DocFreqPair {
                            document_number,
                            word_frequency: freq,
                        });
                    }
                }
            }
        }
    }

    pub fn batch_update_index(&self, updates: Vec<(i64, HashMap<String, i64>)>) {
        let mut sharded_updates: Vec<HashMap<String, Vec<(i64, i64)>>> = 
            vec![HashMap::new(); NUM_SHARDS];

        for (doc_num, word_freqs) in updates {
            for (word, freq) in word_freqs {
                let shard = self.get_term_shard_index(&word);
                sharded_updates[shard]
                    .entry(word)
                    .or_insert_with(Vec::new)
                    .push((doc_num, freq));
            }
        }

        for (shard_idx, updates) in sharded_updates.into_iter().enumerate() {
            if !updates.is_empty() {
                let mut shard = self.term_index_shards[shard_idx].write();
                for (word, doc_freqs) in updates {
                    let postings = shard.term_index.entry(word).or_insert_with(Vec::new);
                    for (doc_num, freq) in doc_freqs {
                        if let Some(existing) = postings.iter_mut()
                            .find(|p| p.document_number == doc_num) {
                            existing.word_frequency = freq;
                        } else {
                            postings.push(DocFreqPair {
                                document_number: doc_num,
                                word_frequency: freq,
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn lookup_index(&self, term: &str) -> Vec<DocFreqPair> {
        // Look in all shards, not just the hashed shard
        let mut all_results = Vec::new();
        
        // Search through all shards
        for (shard_idx, shard) in self.term_index_shards.iter().enumerate() {
            let shard = shard.read();
            println!("Looking up term '{}' in shard {}", term, shard_idx);
            
            if let Some(postings) = shard.term_index.get(term) {
                println!("Found {} matches in shard {}", postings.len(), shard_idx);
                all_results.extend(postings.clone());
            }
        }
    
        println!("Total matches for term '{}': {}", term, all_results.len());
        all_results
    }
    
    pub fn search(&self, terms: &[String]) -> Vec<(String, i64)> {
        if terms.is_empty() {
            return Vec::new();
        }

        // Get results for each term
        let mut all_results: Vec<Vec<DocFreqPair>> = Vec::new();
        for term in terms {
            let results = self.lookup_index(term);
            println!("Search term '{}' found {} matches", term, results.len());
            all_results.push(results);
        }

        // Combine results (AND operation)
        let mut combined_freqs: HashMap<i64, i64> = HashMap::new();
        if !all_results.is_empty() {
            // Initialize with first term's results
            for pair in &all_results[0] {
                combined_freqs.insert(pair.document_number, pair.word_frequency);
            }

            // AND with remaining terms
            for results in all_results.iter().skip(1) {
                let mut new_freqs: HashMap<i64, i64> = HashMap::new();
                for pair in results {
                    if let Some(prev_freq) = combined_freqs.get(&pair.document_number) {
                        new_freqs.insert(
                            pair.document_number,
                            prev_freq + pair.word_frequency
                        );
                    }
                }
                combined_freqs = new_freqs;
            }
        }

        // Sort by frequency and convert to paths
        let mut sorted_results: Vec<(String, i64)> = combined_freqs
            .into_iter()
            .filter_map(|(doc_num, freq)| {
                self.get_document(doc_num)
                    .map(|path| (path, freq))
            })
            .collect();

        sorted_results.sort_by(|a, b| b.1.cmp(&a.1));

        // Get top 10
        sorted_results.truncate(10);
        sorted_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_operations() {
        let store = IndexStore::new();
        let doc_path = "test/path/doc1.txt".to_string();

        let doc_num = store.put_document(doc_path.clone());
        assert!(doc_num > 0);

        let retrieved_path = store.get_document(doc_num).unwrap();
        assert_eq!(retrieved_path, doc_path);
    }

    #[test]
    fn test_index_operations() {
        let store = IndexStore::new();
        let doc_num = store.put_document("test/doc1.txt".to_string());

        let mut word_freqs = HashMap::new();
        word_freqs.insert("test".to_string(), 5);
        word_freqs.insert("word".to_string(), 3);

        store.update_index(doc_num, word_freqs);

        let results = store.lookup_index("test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_number, doc_num);
        assert_eq!(results[0].word_frequency, 5);
    }
}
