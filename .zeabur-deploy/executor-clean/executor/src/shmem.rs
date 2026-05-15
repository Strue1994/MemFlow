use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared_memory::{Shmem, ShmemConf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShmemError {
    #[error("Shared memory creation failed: {0}")]
    Creation(String),
    #[error("Write failed: {0}")]
    Write(String),
    #[error("Read failed: {0}")]
    Read(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

const HEADER_SIZE: usize = 8;

#[derive(Clone, Serialize, Deserialize)]
pub struct ShmemRequest {
    pub workflow_id: String,
    pub params: Value,
    pub version: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShmemResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

pub struct SharedMemory {
    shmem: Shmem,
}

impl SharedMemory {
    pub fn create(name: &str, size: usize) -> Result<Self, ShmemError> {
        let shmem = ShmemConf::new()
            .size(size)
            .os_id(name)
            .create()
            .map_err(|e| ShmemError::Creation(e.to_string()))?;
        Ok(Self { shmem })
    }

    pub fn open(name: &str) -> Result<Self, ShmemError> {
        let shmem = ShmemConf::new()
            .os_id(name)
            .open()
            .map_err(|e| ShmemError::Creation(e.to_string()))?;
        Ok(Self { shmem })
    }

    pub fn write_request(&mut self, req: &ShmemRequest) -> Result<(), ShmemError> {
        let data = bincode::serialize(req).map_err(|e| ShmemError::Serialization(e.to_string()))?;
        if data.len() + HEADER_SIZE > self.shmem.len() {
            return Err(ShmemError::Write(
                "Data size exceeds shared memory capacity".to_string(),
            ));
        }
        unsafe {
            let ptr = self.shmem.as_ptr() as *mut u8;
            let len_bytes = data.len().to_le_bytes();
            ptr.copy_from(len_bytes.as_ptr(), HEADER_SIZE);
            ptr.add(HEADER_SIZE).copy_from(data.as_ptr(), data.len());
        }
        Ok(())
    }

    pub fn read_response(&self) -> Result<ShmemResponse, ShmemError> {
        unsafe {
            let ptr = self.shmem.as_ptr();
            let len = usize::from_le_bytes(*(ptr as *const [u8; HEADER_SIZE]));
            if len == 0 || len + HEADER_SIZE > self.shmem.len() {
                return Err(ShmemError::Read("Invalid data length".to_string()));
            }
            let data = std::slice::from_raw_parts(ptr.add(HEADER_SIZE), len);
            let response =
                bincode::deserialize(data).map_err(|e| ShmemError::Serialization(e.to_string()))?;
            Ok(response)
        }
    }

    pub fn write_response(&mut self, resp: &ShmemResponse) -> Result<(), ShmemError> {
        let data =
            bincode::serialize(resp).map_err(|e| ShmemError::Serialization(e.to_string()))?;
        if data.len() + HEADER_SIZE > self.shmem.len() {
            return Err(ShmemError::Write(
                "Data size exceeds shared memory capacity".to_string(),
            ));
        }
        unsafe {
            let ptr = self.shmem.as_ptr() as *mut u8;
            let len_bytes = data.len().to_le_bytes();
            ptr.copy_from(len_bytes.as_ptr(), HEADER_SIZE);
            ptr.add(HEADER_SIZE).copy_from(data.as_ptr(), data.len());
        }
        Ok(())
    }

    pub fn read_request(&self) -> Result<ShmemRequest, ShmemError> {
        unsafe {
            let ptr = self.shmem.as_ptr();
            let len = usize::from_le_bytes(*(ptr as *const [u8; HEADER_SIZE]));
            if len == 0 || len + HEADER_SIZE > self.shmem.len() {
                return Err(ShmemError::Read("Invalid data length".to_string()));
            }
            let data = std::slice::from_raw_parts(ptr.add(HEADER_SIZE), len);
            let request =
                bincode::deserialize(data).map_err(|e| ShmemError::Serialization(e.to_string()))?;
            Ok(request)
        }
    }

    pub fn len(&self) -> usize {
        self.shmem.len()
    }
}
