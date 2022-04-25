extern crate nalgebra as na;

mod block;
mod chunk;
mod util;
mod volume;

#[tokio::main]
async fn main() {
    println!("Hello World!")
}

#[cfg(test)]
mod tests {}
