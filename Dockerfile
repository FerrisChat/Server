FROM rust:1-slim-bullseye as build

COPY . .
RUN cargo build --release

# our final base
FROM debian:bullseye-slim

# copy the build artifact from the build stage
COPY --from=build /ferrischat/target/release/both ./ferrischat

# set the startup command to run your binary
EXPOSE 8080/tcp
CMD ["./ferrischat", "./config.toml"]
