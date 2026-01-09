FROM rust:latest as builder
WORKDIR /usr/src/piper-server
COPY . .
RUN cargo install --path .

FROM debian:stable
COPY --from=builder /usr/local/cargo/bin/piper-server /usr/local/bin/piper-server
CMD [ "piper-server" ]
