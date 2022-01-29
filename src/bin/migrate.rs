fn main() {
    let rt = ferrischat_core::init();
    rt.block_on(ferrischat_core::async_init(true));
    rt.block_on(ferrischat_core::run_migrations());
}
