### Messing around with procedural generation in Rust.
This repository is going to serve as a place to keep a bunch of neat procedural generation related stuff.
I have vague plans of writing a little protocol + client/server that'll allow me to have a server (written in Rust hopefully btw btw)
which will generate and send terrain at the request of a client, which will be implemented as a Minecraft (Spigot) plugin.
Reasons being:
  1. World generation is very fun to mess around with but can be slow and inefficient. Because Minecraft is already laggy
  as hell in its current state, offloading large parts of this ordeal to an external server could reduce the load on the
  main server so it has more time to lag around with other (more important) things instead.
  2. Writing any mathematical code in Java is uh... undesirable, to put it mildly. Kotlin doesn't really help this issue either
  despite having operator overloading because there's not really any libraries that are ergonomic enough for me out there (im very picky).
  3. "you can use rust for this, btw, btw, btw". One thing I really like about Rust (maybe even more so than the AMAZING compile-time memory safety,
  fearless concurrency, and zero-cost abstractions) is the ergonomics of the language, especially when it comes to math! The type system really
  helps this with crates like [num-traits](https://crates.io/crates/num-traits), and there's also the lovely [nalgebra](https://crates.io/crates/nalgebra)
  crate for linear algebra which I find quite satisfying too.
  
