pub mod base_modules;
pub mod engine;

mod alloc;
mod cables;
mod config;

use std::fmt::Debug;

use cables::{Input, Output};

#[global_allocator]
static A: alloc::ZealousAllocator = alloc::ZealousAllocator;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortNum(u8);

pub trait Module: Debug + Send + 'static {
    fn num_inputs(&self) -> u8;
    fn num_outputs(&self) -> u8;

    fn run(&mut self, inputs: &[Input], outputs: &[Output]);
}
