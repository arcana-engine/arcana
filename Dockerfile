FROM rust:alpine3.14 as builder

# RUN apk add curl
# RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN rustup toolchain add nightly
RUN rustup +nightly target add x86_64-unknown-linux-musl

RUN apk add musl-dev

WORKDIR /arcana-server

COPY Cargo.* .
COPY examples examples
COPY images images
COPY import import
COPY mesh-file mesh-file
COPY proc proc
COPY src src
COPY time time
RUN cargo +nightly update
RUN cargo +nightly build --release --no-default-features --target=x86_64-unknown-linux-musl -p tanks-server

FROM scratch

EXPOSE 12345

WORKDIR /arcana-server
COPY --from=builder /arcana-server/target/x86_64-unknown-linux-musl/release/tanks-server /arcana-server/tanks-server
COPY --from=builder /arcana-server/examples/cfg.json /arcana-server/cfg.json
COPY --from=builder /arcana-server/examples/assets/.treasury /arcana-server/assets/.treasury

ENTRYPOINT [ "./tanks-server" ]
