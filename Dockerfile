# SPDX-License-Identifier: MIT
#
# Docker file for server-side component of COMP4000 experience 1.
# Copyright (c) 2021  William Findlay
#
# September 16, 2021  William Findlay  Created this.

FROM lukemathwalker/cargo-chef:latest-rust-1.55.0-alpine AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM busybox
WORKDIR /app
COPY --from=builder /app/target/release/hello4000 /app/hello4000

ENTRYPOINT ["/app/hello4000"]

# vi:ft=dockerfile
