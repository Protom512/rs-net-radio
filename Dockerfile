FROM  ekidd/rust-musl-builder:1.57  as builder
ADD --chown=rust:rust . ./

RUN cargo build --release


FROM alpine:3:15

COPY --from=builder \
/home/rust/src/target/x86_64-unknown-linux-musl/release/rs-net-radio /usr/local/bin/
RUN apk --no-cache add tzdata ffmpeg python3 py3-pip musl-dev build-base alpine-sdk libxml2-dev libxslt-dev python3-dev \
&& pip3 install streamlink

CMD ["/usr/local/bin/rs-net-radio"]
