# SPDX-License-Identifier: MIT
#
# Docker file for server-side component of COMP4000 experience 1.
# Copyright (c) 2021  William Findlay
#
# September 16, 2021  William Findlay  Created this.

FROM ekidd/rust-musl-builder:latest AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/hello4000 /app/hello4000
EXPOSE 4000

cmd ["/app/hello4000"]

# vi:ft=dockerfile
