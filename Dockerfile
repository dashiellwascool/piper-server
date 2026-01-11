FROM rust:latest as builder
WORKDIR /usr/src/piper-server
COPY . .
RUN cargo run --release

