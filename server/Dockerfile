FROM rust:1.98.1

COPY ./ ./

RUN cargo build --release

CMD ["./target/release/kanalis"]
