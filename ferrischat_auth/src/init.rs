use tokio::sync::mpsc::channel;

pub fn init_auth() {
    {
        let (tx, mut rx) = channel::<(
            String,
            tokio::sync::oneshot::Sender<Result<String, argonautica::Error>>,
        )>(250);
        let mut hasher = argonautica::Hasher::new();
        hasher
            .opt_out_of_secret_key(true) // we don't need secret keys atm
            .configure_password_clearing(true) // clear passwords from memory after hashing
            .configure_memory_size(8_192); // use 8MiB memory to hash

        std::thread::spawn(move || {
            while let Some(d) = rx.blocking_recv() {
                let (password, sender) = d;

                let r = hasher.with_password(password).hash();
                let _res = sender.send(r);
            }
        });

        crate::GLOBAL_HASHER
            .set(tx)
            .expect("couldn't set global hasher for some reason");
    }
    {
        let (tx, mut rx) = channel::<(
            (String, String),
            tokio::sync::oneshot::Sender<Result<bool, argonautica::Error>>,
        )>(250);
        let mut verifier = argonautica::Verifier::new();
        verifier
            .configure_password_clearing(true)
            .configure_secret_key_clearing(true);

        std::thread::spawn(move || {
            while let Some(d) = rx.blocking_recv() {
                let (password, sender) = d;

                let r = verifier
                    .with_password(password.0)
                    .with_hash(password.1)
                    .verify();
                let _res = sender.send(r);
            }
        });

        crate::GLOBAL_VERIFIER
            .set(tx)
            .expect("failed to set password verifier");
    }
}
