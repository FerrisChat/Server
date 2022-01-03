fn main() {
    let rt = ferrischat_core::init();
    rt.block_on(ferrischat_core::async_init(false));
    rt.block_on(ferrischat_core::start_http());
}
