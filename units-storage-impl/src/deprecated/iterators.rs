//! Simplified iterator implementations for UNITS storage
//! 
//! This module provides clean, simple iterators without complex async adapters

use std::marker::PhantomData;
use units_core::error::StorageError;
use units_core::objects::UnitsObject;
use units_core::proofs::{SlotNumber, StateProof, UnitsObjectProof};
use units_core::transaction::TransactionReceipt;

//==============================================================================
// SIMPLE ITERATOR WRAPPER
//==============================================================================

/// A simple wrapper for storage iterators
/// 
/// This replaces the complex UnitsIterator with async adapters
pub struct StorageIterator<T> {
    inner: Box<dyn Iterator<Item = Result<T, StorageError>> + Send>,
}

impl<T: Send + 'static> StorageIterator<T> {
    /// Create a new storage iterator
    pub fn new<I>(iter: I) -> Self
    where
        I: Iterator<Item = Result<T, StorageError>> + Send + 'static,
    {
        Self {
            inner: Box::new(iter),
        }
    }
    
    /// Create an empty iterator
    pub fn empty() -> Self {
        Self {
            inner: Box::new(std::iter::empty()),
        }
    }
    
    /// Create an iterator from a vector
    pub fn from_vec(items: Vec<T>) -> Self {
        Self {
            inner: Box::new(items.into_iter().map(Ok)),
        }
    }
    
    /// Create an iterator that yields a single error
    pub fn error(error: StorageError) -> Self {
        Self {
            inner: Box::new(std::iter::once(Err(error))),
        }
    }
}

impl<T> Iterator for StorageIterator<T> {
    type Item = Result<T, StorageError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

//==============================================================================
// TYPE ALIASES FOR COMMON ITERATORS
//==============================================================================

/// Iterator over storage objects
pub type ObjectIterator = StorageIterator<UnitsObject>;

/// Iterator over object proofs
pub type ProofIterator = StorageIterator<(SlotNumber, UnitsObjectProof)>;

/// Iterator over state proofs
pub type StateProofIterator = StorageIterator<StateProof>;

/// Iterator over transaction receipts
pub type ReceiptIterator = StorageIterator<TransactionReceipt>;

//==============================================================================
// FILTERED ITERATOR
//==============================================================================

/// A filtered iterator that applies a predicate
pub struct FilteredIterator<T, F> {
    inner: StorageIterator<T>,
    filter: F,
}

impl<T, F> FilteredIterator<T, F>
where
    F: Fn(&T) -> bool,
{
    /// Create a new filtered iterator
    pub fn new(inner: StorageIterator<T>, filter: F) -> Self {
        Self { inner, filter }
    }
}

impl<T, F> Iterator for FilteredIterator<T, F>
where
    F: Fn(&T) -> bool,
{
    type Item = Result<T, StorageError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next()? {
                Ok(item) => {
                    if (self.filter)(&item) {
                        return Some(Ok(item));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

//==============================================================================
// MAPPED ITERATOR
//==============================================================================

/// An iterator that maps items to a different type
pub struct MappedIterator<T, U, F> {
    inner: StorageIterator<T>,
    mapper: F,
    _phantom: PhantomData<U>,
}

impl<T, U, F> MappedIterator<T, U, F>
where
    F: Fn(T) -> U,
{
    /// Create a new mapped iterator
    pub fn new(inner: StorageIterator<T>, mapper: F) -> Self {
        Self {
            inner,
            mapper,
            _phantom: PhantomData,
        }
    }
}

impl<T, U, F> Iterator for MappedIterator<T, U, F>
where
    F: Fn(T) -> U,
{
    type Item = Result<U, StorageError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result.map(&self.mapper)
        })
    }
}

//==============================================================================
// BATCHED ITERATOR
//==============================================================================

/// An iterator that yields items in batches
pub struct BatchedIterator<T> {
    inner: StorageIterator<T>,
    batch_size: usize,
}

impl<T> BatchedIterator<T> {
    /// Create a new batched iterator
    pub fn new(inner: StorageIterator<T>, batch_size: usize) -> Self {
        Self {
            inner,
            batch_size: batch_size.max(1),
        }
    }
}

impl<T> Iterator for BatchedIterator<T> {
    type Item = Result<Vec<T>, StorageError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = Vec::with_capacity(self.batch_size);
        
        for _ in 0..self.batch_size {
            match self.inner.next() {
                Some(Ok(item)) => batch.push(item),
                Some(Err(e)) => return Some(Err(e)),
                None => break,
            }
        }
        
        if batch.is_empty() {
            None
        } else {
            Some(Ok(batch))
        }
    }
}

//==============================================================================
// ITERATOR EXTENSIONS
//==============================================================================

/// Extension trait for storage iterators
pub trait StorageIteratorExt<T>: Iterator<Item = Result<T, StorageError>> + Sized + Send + 'static 
where 
    T: Send + 'static
{
    /// Filter items by a predicate
    fn filter_storage<F>(self, filter: F) -> FilteredIterator<T, F>
    where
        F: Fn(&T) -> bool,
    {
        FilteredIterator::new(StorageIterator::new(self), filter)
    }
    
    /// Map items to a different type
    fn map_storage<U, F>(self, mapper: F) -> MappedIterator<T, U, F>
    where
        F: Fn(T) -> U,
        U: Send + 'static,
    {
        MappedIterator::new(StorageIterator::new(self), mapper)
    }
    
    /// Yield items in batches
    fn batch(self, size: usize) -> BatchedIterator<T> {
        BatchedIterator::new(StorageIterator::new(self), size)
    }
    
    /// Collect all items, stopping at the first error
    fn collect_storage(self) -> Result<Vec<T>, StorageError> {
        self.collect()
    }
    
    /// Count items, stopping at the first error
    fn count_storage(self) -> Result<usize, StorageError> {
        let mut count = 0;
        for result in self {
            result?;
            count += 1;
        }
        Ok(count)
    }
}

impl<T, I> StorageIteratorExt<T> for I 
where 
    I: Iterator<Item = Result<T, StorageError>> + Sized + Send + 'static,
    T: Send + 'static,
{}