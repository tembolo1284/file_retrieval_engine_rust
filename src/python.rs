use pyo3::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use crate::server::index_store::IndexStore;
use crate::client::engine::ClientProcessingEngine;

#[pyclass]
struct PyIndexStore {
    store: Arc<IndexStore>
}

#[pymethods]
impl PyIndexStore {
    #[new]
    fn new() -> Self {
        PyIndexStore {
            store: Arc::new(IndexStore::new())
        }
    }
    
    fn put_document(&self, document_path: String) -> i64 {
        self.store.put_document(document_path)
    }
    
    fn search(&self, terms: Vec<String>) -> Vec<(String, i64)> {
        self.store.search(&terms)
    }
    
    fn update_index(&self, doc_num: i64, word_freqs: HashMap<String, i64>) {
        self.store.update_index(doc_num, word_freqs);
    }
}

#[pyclass]
struct PyClient {
    engine: ClientProcessingEngine
}

#[pymethods]
impl PyClient {
    #[new]
    fn new() -> Self {
        PyClient {
            engine: ClientProcessingEngine::new()
        }
    }
    
    fn connect(&mut self, server_ip: &str, server_port: &str) -> PyResult<()> {
        self.engine.connect(server_ip, server_port)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyConnectionError, _>(e.to_string()))
    }
    
    fn index_folder(&self, folder_path: &str) -> PyResult<(f64, i64)> {
        self.engine.index_folder(folder_path)
            .map(|result| (result.execution_time, result.total_bytes_read))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    fn search(&self, terms: Vec<String>) -> PyResult<(f64, Vec<(String, i64)>)> {
        self.engine.search(terms)
            .map(|result| {
                let docs = result.document_frequencies.into_iter()
                    .map(|pair| (pair.document_path, pair.word_frequency))
                    .collect();
                (result.execution_time, docs)
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

// IMPORTANT: This function is exported and used by PyO3 to initialize the module
// The name must match the library name exactly
#[pymodule]
#[pyo3(name = "file_retrieval_engine")]
pub fn init_module(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyIndexStore>()?;
    m.add_class::<PyClient>()?;
    Ok(())
}
