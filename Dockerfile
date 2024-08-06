FROM rust:latest AS builder
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update -y
RUN apt install -y musl-tools musl-dev ca-certificates curl gnupg

RUN mkdir -p /etc/apt/keyrings
RUN curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg
RUN echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_20.x nodistro main" | tee /etc/apt/sources.list.d/nodesource.list
RUN apt-get update -y
RUN apt-get install -y nodejs

RUN mkdir -p /usr/src/parcel
WORKDIR /usr/src/parcel
ADD . .
RUN npm install
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch
EXPOSE 3000
COPY --from=builder /usr/src/parcel/target/x86_64-unknown-linux-musl/release/parcel .
COPY --from=builder /usr/src/parcel/static ./static
ENTRYPOINT ["./parcel"]


