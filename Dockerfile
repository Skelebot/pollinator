FROM rust:latest as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/poll
COPY src ./src
COPY static ./static
COPY templates ./templates
COPY Cargo.toml .
COPY Cargo.lock .

RUN cargo install --path . --root /usr/

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y --no-install-recommends sqlite3

WORKDIR /usr/local/polls
COPY --from=build /usr/bin/polls .
COPY templates ./templates
COPY static ./static

VOLUME "/db"

CMD ["/usr/local/polls/polls"]
