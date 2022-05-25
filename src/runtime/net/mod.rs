async fn run() -> ! {
    // TODO: this is essentially #[tokio::main] but we manually build the runtime and submit this as the "main" function to it.
    // this function should set up all the networking stuff and then diverge into just serving terrain data over TCP.
    todo!()
}
