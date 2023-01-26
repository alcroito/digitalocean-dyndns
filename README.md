# Dynamic DNS using DigitalOcean's DNS API

![Logo](./docs/logo.png)

[![GitHub Source](https://img.shields.io/badge/github-source-ffb64c?style=flat-square&logo=github&logoColor=white&labelColor=757575)](https://github.com/alcroito/digitalocean-dyndns)
[![GitHub Registry](https://img.shields.io/badge/github-registry-ffb64c?style=flat-square&logo=github&logoColor=white&labelColor=757575)](https://github.com/users/alcroito/packages/container/package/digitalocean-dyndns)
[![Docker Pulls](https://img.shields.io/docker/pulls/alcroito/digitalocean-dyndns?color=ffb64c&label=pulls&logo=docker&logoColor=white&labelColor=757575)](https://hub.docker.com/r/alcroito/digitalocean-dyndns)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/alcroito/digitalocean-dyndns/ci.yaml?style=flat-square&logo=github&logoColor=white&labelColor=757575)](https://github.com/alcroito/digitalocean-dyndns/actions)
[![License](https://img.shields.io/github/license/alcroito/digitalocean-dyndns)](https://github.com/alcroito/digitalocean-dyndns/blob/master/LICENSE)

A Unix daemon that periodically updates DigitalOcean domain records with the current machine's public IP address.

## How it works

The daemon periodically runs the following steps:

* finds the current machine's public IPv4 by sending a DNS request to an OpenDNS resolver
* queries the configured domain records using DO's API. If the queried IPs
  are different from the current public API, the domain records are updated to point to the new IP

## Setup

* A Unix (Linux / macOS) server to run the daemon
* A DigitalOcean account with your domain associated to it
* An existing `A` record for each domain to be updated

## Usage

There are 2 configuration modes: **simple** and **advanced**.

**Simple** mode allows updating only one *single* domain record.
Simple mode can be configured via command line arguments, environment variables or a config file.

**Advanced** mode allows updating *multiple* domains and records.
Advanced mode can only be configured via the config file.

See [do_ddns.sample.toml](./config/do_ddns.sample.toml) for a sample configuration file.

Run `do_ddns -h` to see the available command line options and environment variables.

## Build requirements

To build the application you need a recent enough version of the Rust compiler (1.45+).
Build using `cargo build`. The executable will be placed into `$PWD/target[/target-arch]/do_ddns`.

## Docker images and docker-compose

Docker images based on Alpine Linux (~7MB) are available for your server-y needs.

They are regularly built using Github Actions for the following platforms:

* `linux/amd64`
* `linux/arm64`
* `linux/arm/v7`

They can be downloaded from [DockerHub](https://hub.docker.com/r/alcroito/digitalocean-dyndns) and the [Github Container registry](https://github.com/users/alcroito/packages/container/package/digitalocean-dyndns).

An easy way to use them is via the sample [docker-compose.yaml](./docker/docker-compose.yaml) file.

## LICENSE

MIT

## Alternative implementations

* [tunix/digitalocean-dyndns](https://github.com/tunix/digitalocean-dyndns)
  * written in `Bash`
  * provides `amd64` alpine-based docker image `~5MB`
  * uses `curl`/`HTTP` for IP resolving (3 possible services)
  * appears to be maintained 👏
* [skibish/ddns](https://github.com/skibish/ddns)
  * written in `Golang`
  * provides `amd64` alpine-based docker image `~8MB`
  * uses `HTTP` for IP resolving (3 possible services)
  * supports update notification via SMTP and Telegram
  * supports resolving `IPv6` addresses
  * supports updating multiple domain records of different types (`A`, `CNAME`, `TXT`)
  * provides standalone binaries for `darwin/amd64`, `linux/armv7`, `linux/amd64`, `windows/amd64`
  * appears to be maintained 👏
* [KyleLilly/do-dyndns](https://github.com/KyleLilly/do-dyndns)
  * written in `Javascript/NodeJS`
  * runs as a server expecting `PUT` requests
  * provides `Dockerfile`, but no image
  * doesn't resolve the public ip
  * appears unmaintained 👎
* [creltek/digitalocean-dyndns](https://github.com/creltek/digitalocean-dyndns)
  * written in `Python`
  * uses one `HTTP` based service for IP resolving
  * provides `Dockerfile`, but no image
  * appears unmaintained 👎
* [FMCorz/digitalocean-dyndns](https://github.com/FMCorz/digitalocean-dyndns)
  * written in `Python`
  * uses one `HTTP` based service for IP resolving
  * appears unmaintained 👎
* Many others that appear unmaintained or don't provide docker images
