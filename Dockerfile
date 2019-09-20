FROM rust:1.37-buster AS build
WORKDIR /build
COPY . .
RUN cargo build --release


FROM debian:buster
LABEL maintainer="KITAGAWA Yasutaka <kit494way@gmail.com>"
RUN apt-get update && apt-get install -y libssl-dev \
    && apt-get clean \
    && rm -fr /var/lib/apt/lists/*
COPY --from=build /build/target/release/csal ./

EXPOSE 3000
ENTRYPOINT ["./csal"]
CMD ["--help"]
