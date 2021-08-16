#![feature(once_cell)]

use redis::cluster::{ClusterClient, ClusterConnection};

pub fn load_redis() -> (ClusterClient, ClusterConnection) {
    let nodes = vec!["redis://127.0.0.1:6379/"];

    let client = ClusterClient::open(nodes).expect("initial redis connection failed");
    let conn = client.get_connection().expect("failed to open redis connection");

    (client, conn)
}