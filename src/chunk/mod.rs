#[allow(dead_code)]
mod access;
#[allow(dead_code)]
mod basic;
mod section;
#[allow(dead_code)]
mod trait_impls;

mod serialization;

pub use basic::Chunk;

#[cfg(test)]
mod tests {
    // TODO: tests for Chunks
}
