use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

#[cfg(feature = "gpu")]
use crate::cl;
use crate::error::Error;
use crate::poseidon::SimplePoseidonBatchHasher;
use crate::{Arity, BatchHasher, Strength, DEFAULT_STRENGTH};
use bellperson::bls::Fr;
use generic_array::GenericArray;
use rust_gpu_tools::opencl::GPUSelector;

#[cfg(feature = "gpu")]
use triton::FutharkContext;

#[derive(Clone)]
pub enum BatcherType {
    #[cfg(feature = "gpu")]
    CustomGPU(GPUSelector),
    #[cfg(feature = "gpu")]
    FromFutharkContext(Arc<Mutex<FutharkContext>>),
    GPU,
    CPU,
}

impl Debug for BatcherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("BatcherType::"))?;
        match self {
            BatcherType::FromFutharkContext(_) => f.write_fmt(format_args!("FromFutharkContext")),
            BatcherType::CustomGPU(x) => f.write_fmt(format_args!("CustomGPU({:?})", x)),
            BatcherType::CPU => f.write_fmt(format_args!("CPU")),
            BatcherType::GPU => f.write_fmt(format_args!("GPU")),
        }
    }
}

use crate::gpu::GPUBatchHasher;

pub enum Batcher<A>
where
    A: Arity<Fr>,
{
    GPU(GPUBatchHasher<A>),
    CPU(SimplePoseidonBatchHasher<A>),
}

impl<A> Batcher<A>
where
    A: Arity<Fr>,
{
    pub(crate) fn t(&self) -> BatcherType {
        match self {
            Batcher::GPU(_) => BatcherType::GPU,
            Batcher::CPU(_) => BatcherType::CPU,
        }
    }

    pub(crate) fn new(t: &BatcherType, max_batch_size: usize) -> Result<Self, Error> {
        Self::new_with_strength(DEFAULT_STRENGTH, t, max_batch_size)
    }

    pub(crate) fn new_with_strength(
        strength: Strength,
        t: &BatcherType,
        max_batch_size: usize,
    ) -> Result<Self, Error> {
        match t {
            #[cfg(feature = "gpu")]
            BatcherType::GPU => Ok(Batcher::GPU(GPUBatchHasher::<A>::new_with_strength(
                cl::default_futhark_context()?,
                strength,
                max_batch_size,
            )?)),
            #[cfg(feature = "gpu")]
            BatcherType::CustomGPU(selector) => {
                Ok(Batcher::GPU(GPUBatchHasher::<A>::new_with_strength(
                    cl::futhark_context(*selector)?,
                    strength,
                    max_batch_size,
                )?))
            }
            BatcherType::CPU => Ok(Batcher::CPU(
                SimplePoseidonBatchHasher::<A>::new_with_strength(strength, max_batch_size)?,
            )),
            #[cfg(feature = "gpu")]
            BatcherType::FromFutharkContext(futhark_context) => {
                Ok(Batcher::GPU(GPUBatchHasher::<A>::new_with_strength(
                    futhark_context.clone(),
                    strength,
                    max_batch_size,
                )?))
            }
        }
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn futhark_context(&self) -> Option<Arc<Mutex<FutharkContext>>> {
        match self {
            Batcher::GPU(b) => Some(b.futhark_context()),
            _ => None,
        }
    }
}

impl<A> BatchHasher<A> for Batcher<A>
where
    A: Arity<Fr>,
{
    fn hash(&mut self, preimages: &[GenericArray<Fr, A>]) -> Result<Vec<Fr>, Error> {
        match self {
            Batcher::GPU(batcher) => batcher.hash(preimages),
            Batcher::CPU(batcher) => batcher.hash(preimages),
        }
    }

    fn max_batch_size(&self) -> usize {
        match self {
            Batcher::GPU(batcher) => batcher.max_batch_size(),
            Batcher::CPU(batcher) => batcher.max_batch_size(),
        }
    }
}

// /// NoGPUBatchHasher is a dummy required so we can build with the gpu flag even on platforms on which we cannot currently
// /// run with GPU.
pub struct NoGPUBatchHasher<A>(PhantomData<A>);

impl<A> BatchHasher<A> for NoGPUBatchHasher<A>
where
    A: Arity<Fr>,
{
    fn hash(&mut self, _preimages: &[GenericArray<Fr, A>]) -> Result<Vec<Fr>, Error> {
        unimplemented!();
    }

    fn max_batch_size(&self) -> usize {
        unimplemented!();
    }
}

#[cfg(feature = "gpu")]
impl<A> NoGPUBatchHasher<A>
where
    A: Arity<Fr>,
{
    fn futhark_context(&self) -> Arc<Mutex<FutharkContext>> {
        unimplemented!()
    }
}
