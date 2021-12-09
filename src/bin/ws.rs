fn main() {
    let rt = ferrischat_core::init();
    rt.block_on(ferrischat_core::async_init());
    rt.block_on(ferrischat_core::start_ws());
}
