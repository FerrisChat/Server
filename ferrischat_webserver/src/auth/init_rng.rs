use ring::rand::{SecureRandom, SystemRandom};

pub fn init_rng() {
    crate::RNG_CORE
        .set(SystemRandom::new())
        .expect("failed to set RNG");

    let mut v = vec![0; 64];
    // we call fill here to be sure that the RNG will block if required here instead of
    // in the webserver loop
    crate::RNG_CORE
        .get()
        .expect("RNG was already set but unloaded?")
        .fill(&mut v)
        .expect("failed to generate RNG");
}
