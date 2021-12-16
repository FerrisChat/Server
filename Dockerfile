FROM rustlang/rust:nightly AS builder
COPY . /build
WORKDIR /build
ENV SQLX_OFFLINE true
ENV RUSTFLAGS -Ctarget-cpu=native --emit=asm
RUN cargo build --release
FROM alpine
COPY --from=builder /build/config.example.toml /home/config.toml
COPY --from=builder /build/target/release/both /usr/bin/ferrischat_server
ENTRYPOINT ["/usr/bin/ferrischat_server"]
CMD ["/home/config.toml"]
