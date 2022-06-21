FROM  ekidd/rust-musl-builder:1.57.0  as builder
COPY --chown=rust:rust . ./
RUN cargo build --release


FROM ubuntu:jammy
ARG TZ=Asia/Tokyo
COPY --from=builder \
/home/rust/src/target/x86_64-unknown-linux-musl/release/rs-net-radio /usr/local/bin/
RUN apt-get update -qqy && apt-get install --no-install-recommends tzdata=2022a-0ubuntu1 ca-certificates ffmpeg=7:4.4.2* libssl-dev=3.0.2-0ubuntu1.7 -y && apt-get autoremove && rm -rf /var/lib/apt/lists/*

CMD ["/usr/local/bin/rs-net-radio"]
