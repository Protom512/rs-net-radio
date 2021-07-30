FROM  ekidd/rust-musl-builder  as builder
ADD --chown=rust:rust . ./

RUN cargo build --release


FROM alpine:latest

COPY --from=builder \
/home/rust/src/target/x86_64-unknown-linux-musl/release/rs-net-radio /usr/local/bin/
RUN apk --no-cache add tzdata ffmpeg

CMD /usr/local/bin/rs-net-radio
