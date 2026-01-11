FROM rust:latest
RUN apt-get update && apt-get install -y cmake clang
WORKDIR /app
COPY . .
RUN cargo build --release
CMD [ "cargo", "run", "--release" ]
