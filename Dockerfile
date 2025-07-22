FROM rust:latest AS builder

# Allow overriding the target architecture
ARG TARGETARCH

# Compute the RUSTARCH based on the TARGETARCH argument; falling back to `uname`.
RUN case "$TARGETARCH" in \
    "amd64")  RUSTARCH="x86_64-unknown-linux-musl" ;; \
    "arm64"*) RUSTARCH="aarch64-unknown-linux-musl" ;; \
    *) echo "Unsupported TARGETARCH: $TARGETARCH"; exit 1 ;; \
  esac && \
  rustup target add $RUSTARCH && \
  echo "RUSTARCH=${RUSTARCH}" >> /etc/environment

# Install necessary packages for building the application
RUN apt update -y
RUN apt install -y musl-tools musl-dev ca-certificates curl gnupg

# Instal Node.js from NodeSource
RUN mkdir -p /etc/apt/keyrings
RUN curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg
RUN echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_21.x nodistro main" | tee /etc/apt/sources.list.d/nodesource.list
RUN apt-get update -y
RUN apt-get install -y nodejs

# Build the Rust application
RUN mkdir -p /usr/src/parcel
WORKDIR /usr/src/parcel
ADD . .
RUN npm install
RUN . /etc/environment && cargo build --target "$RUSTARCH" --release
RUN . /etc/environment && cp "/usr/src/parcel/target/${RUSTARCH}/release/parcel-server" /usr/src/parcel/target/parcel-server

# ------------------------------------------------------------------------------------------------
# Build the final image

FROM alpine:latest

# Install dependencies, such as file identification and preview generation tools.
RUN apk add --no-cache file imagemagick poppler-utils ffmpeg

# Prepare the working directory
WORKDIR /app
COPY --from=builder /usr/src/parcel/target/parcel-server .
COPY --from=builder /usr/src/parcel/static ./static
COPY --from=builder /usr/src/parcel/etc/previewers.json ./etc/previewers.json

# Set the user and run the application
EXPOSE 3000
ENTRYPOINT ["./parcel-server"]
