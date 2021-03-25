FROM rust:latest

WORKDIR /usr/src/static_server
COPY . .
RUN cargo build --release
RUN cargo install --path .
CMD ["/usr/src/static_server/target/release/main"]
