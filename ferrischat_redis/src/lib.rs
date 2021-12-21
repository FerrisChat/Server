#![feature(once_cell)]

pub use deadpool;
use deadpool::managed::{PoolConfig, Timeouts};
pub use deadpool_redis;
use deadpool_redis::{Config, Pool, Runtime};
use ferrischat_config::GLOBAL_CONFIG;
pub use redis;
pub use redis_subscribe;
use redis_subscribe::RedisSub;
use std::lazy::SyncOnceCell as OnceCell;
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

pub static REDIS_MANAGER: OnceCell<Pool> = OnceCell::new();
pub static REDIS_LOCATION: OnceCell<String> = OnceCell::new();
pub static NODE_ID: OnceCell<u16> = OnceCell::new();
static NODE_SECRET: OnceCell<String> = OnceCell::new();

/// Load the Redis pool, set it into the global database pool, and return it.
///
/// # Panics
/// If the global pool was already set.
/// This will only happen if this function is called more than once.
pub async fn load_redis() {
    let cfg = GLOBAL_CONFIG
        .get()
        .expect("config not loaded: this is a bug");
    REDIS_LOCATION
        .set(format!("{}", &cfg.redis))
        .unwrap_or_else(|_| {
            panic!("failed to set Redis database location: did you call load_redis() twice?");
        });

    let mut cfg = Config::from_url(
        REDIS_LOCATION
            .get()
            .expect("just set REDIS_LOCATION but it's already unset?"),
    );
    cfg.pool = {
        use core::time::Duration;
        Some(PoolConfig {
            max_size: 1024,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(15)),
                create: Some(Duration::from_secs(10)),
                recycle: Some(Duration::from_secs(3)),
            },
        })
    };
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("failed to create pool");
    let mut conn = pool
        .get()
        .await
        .expect("failed to open database connection");
    let mut m1 = pool
        .get()
        .await
        .expect("failed to open database connection");
    let mut m2 = pool
        .get()
        .await
        .expect("failed to open database connection");
    REDIS_MANAGER.set(pool).unwrap_or_else(|_| {
        panic!("failed to set Redis pool: did you call load_redis() twice?");
    });

    let mut res = redis::Cmd::hkeys("node_ids")
        .query_async::<_, Vec<String>>(&mut conn)
        .await
        .expect("failed to get all existing node IDs")
        .into_iter()
        .filter_map(|x| x.parse::<u16>().ok())
        .collect::<Vec<_>>();
    res.sort_unstable();

    // compute the next available node ID
    let node_id = {
        // start at 0
        let mut should_be = 0;
        for x in res {
            // if there's a gap in the sequence fill it
            if x != should_be {
                break;
            }
            // no gap? bump it
            should_be = x + 1;
        }
        // if there's a gap this gets hit earlier, and if there's no gap this is just one beyond the end of the list
        should_be
    };

    let node_secret = {
        let mut ns1 = vec![];
        for _ in 0..96 {
            ns1.push(rand::random());
        }
        base64::encode(ns1)
    };

    if redis::Cmd::hset_nx("node_ids", node_id, &node_secret)
        .query_async::<_, u32>(&mut conn)
        .await
        .expect("failed to set new node ID")
        == 0
    {
        panic!("node ID was set while calculating new node ID: perhaps you're spinning up new nodes too fast?");
    }

    NODE_ID
        .set(node_id)
        .unwrap_or_else(|_| panic!("failed to set node ID: did you call `load_redis()` twice?"));
    NODE_SECRET.set(node_secret.clone()).unwrap_or_else(|_| {
        panic!("failed to set node secret: did you call `load_redis()` twice?");
    });

    let ns = node_secret.clone();
    tokio::spawn(async move {
        async fn exit_process() -> bool {
            let current_pid = match sysinfo::get_current_pid() {
                Ok(p) => p,
                Err(_) => return false,
            };
            let sys = System::new_with_specifics(RefreshKind::new().with_processes());
            let current_process = match sys.process(current_pid) {
                Some(p) => p,
                None => return false,
            };

            if !current_process.kill(Signal::Interrupt) {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            if !current_process.kill(Signal::Term) {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            if !current_process.kill(Signal::Kill) {
                return false;
            }

            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            // blocked on IO: forget about destructors at this point, this might be bad WS UX
            // but it can *probably* be handled by a load balancer (TODO: check that out)
            std::process::abort()
        }

        let mut error_counter = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(45)).await;
            match redis::Cmd::hget("node_ids", node_id)
                .query_async::<_, String>(&mut m1)
                .await
            {
                Ok(s) if s == ns => {
                    // nothing's gone wrong here
                    error_counter = 0;
                }
                Ok(_) => {
                    // some other process overwrote our key: this could result in duplicate snowflakes
                    // exit the entire process immediately, since we have no idea wtf just happened

                    while !exit_process().await {}
                    return;
                }
                Err(_) => {
                    // hmm something went wrong
                    // increment error counter and if it hits 3 take fatal action
                    error_counter += 1;
                    if error_counter == 3 {
                        while !exit_process().await {}
                        return;
                    }
                }
            }
        }
    });

    let ns = node_secret.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl+c");
        if redis::Cmd::hget("node_ids", node_id)
            .query_async::<_, String>(&mut m2)
            .await
            .expect("failed to get node info on shutdown")
            == ns
        {
            redis::Cmd::hdel("node_ids", node_id)
                .query_async::<_, ()>(&mut m2)
                .await
                .expect("failed to delete node key from Redis on shutdown");
        };
        REDIS_MANAGER
            .get()
            .expect("failed to get pool to shut it down")
            .close();
    });
}

/// Load the Redis pool as a `RedisSub` object.
#[must_use]
pub fn get_pubsub() -> RedisSub {
    RedisSub::new("127.0.0.1:6379")
}
