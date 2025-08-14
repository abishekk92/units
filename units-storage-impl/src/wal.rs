//! Write-Ahead Log Implementation
//! 
//! Provides concrete implementations of the WriteAheadLog trait for durability.

use units_storage::WriteAheadLog;
use bincode;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use units_core::error::StorageError;
use units_core::objects::UnitsObject;
use units_core::{StateProof, UnitsObjectProof, SlotNumber};

/// WAL entry for object updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALEntry {
    pub object: UnitsObject,
    pub slot: SlotNumber,
    pub proof: UnitsObjectProof,
    pub timestamp: u64,
    pub transaction_hash: Option<[u8; 32]>,
}

/// Entry type in the WAL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WALEntryType {
    /// Update to an object's state
    ObjectUpdate(WALEntry),
    /// State proof for a slot
    StateProof(StateProof),
}

/// A basic file-based write-ahead log implementation
pub struct FileWriteAheadLog {
    /// Path to the WAL file
    path: Arc<Mutex<PathBuf>>,
    /// File handle for writing
    file: Arc<Mutex<Option<BufWriter<File>>>>,
}

impl FileWriteAheadLog {
    /// Create a new file-based WAL
    pub fn new() -> Self {
        Self {
            path: Arc::new(Mutex::new(PathBuf::new())),
            file: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Initialize the WAL with a file path
    pub fn init(&self, path: &Path) -> Result<(), StorageError> {
        let mut file_guard = self
            .file
            .lock()
            .map_err(|e| StorageError::WAL(format!("Failed to acquire lock: {}", e)))?;

        // Create or open the WAL file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .map_err(|e| StorageError::WAL(format!("Failed to open WAL file: {}", e)))?;

        let writer = BufWriter::new(file);

        // Store the file writer
        *file_guard = Some(writer);

        // Store the path
        let mut path_guard = self
            .path
            .lock()
            .map_err(|e| StorageError::WAL(format!("Failed to acquire path lock: {}", e)))?;
        *path_guard = path.to_path_buf();

        Ok(())
    }
    
    /// Write a WAL entry to storage
    fn write_wal_entry(&self, entry: &WALEntryType) -> Result<(), StorageError> {
        let mut file_guard = self
            .file
            .lock()
            .map_err(|e| StorageError::WAL(format!("Failed to acquire lock: {}", e)))?;

        let file = file_guard
            .as_mut()
            .ok_or_else(|| StorageError::WAL("WAL has not been initialized".to_string()))?;

        // Serialize the entry
        let serialized = bincode::serialize(entry)?;

        // Write the entry length and data
        let entry_len = serialized.len() as u64;
        file.write_all(&entry_len.to_le_bytes())?;
        file.write_all(&serialized)?;
        file.flush()?;

        Ok(())
    }
    
    /// Get the current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for FileWriteAheadLog {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteAheadLog for FileWriteAheadLog {
    fn record_update(
        &self,
        object: &UnitsObject,
        proof: &UnitsObjectProof,
        transaction_hash: Option<[u8; 32]>,
    ) -> Result<(), StorageError> {
        let entry = WALEntry {
            object: object.clone(),
            slot: proof.slot,
            proof: proof.clone(),
            timestamp: Self::current_timestamp(),
            transaction_hash,
        };

        self.write_wal_entry(&WALEntryType::ObjectUpdate(entry))
    }

    fn record_state_proof(&self, state_proof: &StateProof) -> Result<(), StorageError> {
        self.write_wal_entry(&WALEntryType::StateProof(state_proof.clone()))
    }

    fn replay<F>(&self, mut callback: F) -> Result<(), StorageError>
    where
        F: FnMut(&UnitsObject, &UnitsObjectProof) -> Result<(), StorageError>,
    {
        let path_guard = self.path.lock()
            .map_err(|e| StorageError::WAL(format!("Failed to acquire path lock: {}", e)))?;
        let path = path_guard.clone();
        drop(path_guard);

        let file = File::open(&path)
            .map_err(|e| StorageError::WAL(format!("Failed to open WAL file: {}", e)))?;
        let mut reader = BufReader::new(file);

        loop {
            // Read the entry length
            let mut len_buf = [0u8; 8];
            match reader.read_exact(&mut len_buf) {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(StorageError::from(e)),
            }

            let entry_len = u64::from_le_bytes(len_buf);

            // Read the entry data
            let mut entry_data = vec![0u8; entry_len as usize];
            reader.read_exact(&mut entry_data)?;

            // Deserialize the entry
            let entry_type: WALEntryType = bincode::deserialize(&entry_data)?;

            // Only replay object updates
            if let WALEntryType::ObjectUpdate(entry) = entry_type {
                callback(&entry.object, &entry.proof)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use units_core::id::UnitsObjectId;
    use units_core::objects::UnitsObject;

    fn create_test_object() -> UnitsObject {
        let id = UnitsObjectId::random();
        let controller_id = UnitsObjectId::random();

        UnitsObject::new_data(
            id,
            controller_id,
            vec![1, 2, 3, 4],
        )
    }

    fn create_test_proof() -> UnitsObjectProof {
        let object_id = UnitsObjectId::random();
        let current_slot = 1234u64;

        UnitsObjectProof {
            object_id: object_id.into(),
            slot: current_slot,
            object_hash: [0u8; 32],
            prev_proof_hash: None,
            transaction_hash: None,
            proof_data: vec![5, 6, 7, 8],
        }
    }

    #[test]
    fn test_wal_object_updates() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = FileWriteAheadLog::new();
        wal.init(&wal_path).unwrap();

        let obj1 = create_test_object();
        let proof1 = create_test_proof();

        let obj2 = create_test_object();
        let proof2 = create_test_proof();

        wal.record_update(&obj1, &proof1, None).unwrap();
        wal.record_update(&obj2, &proof2, None).unwrap();

        let mut entries = Vec::new();
        wal.replay(|obj, proof| {
            entries.push((obj.clone(), proof.clone()));
            Ok(())
        }).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0.id(), obj1.id());
        assert_eq!(entries[1].0.id(), obj2.id());
    }
}