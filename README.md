# Dynamic DNS using DigitalOcean's DNS API

![](./docs/logo.png)

[![GitHub Source](https://img.shields.io/badge/github-source-ffb64c?style=flat-square&logo=github&logoColor=white&labelColor=757575)](https://github.com/alcroito/digitalocean-dyndns)
[![GitHub Registry](https://img.shields.io/badge/github-registry-ffb64c?style=flat-square&logo=github&logoColor=white&labelColor=757575)](https://github.com/users/alcroito/packages/container/package/digitalocean-dyndns)
[![Docker Pulls](https://img.shields.io/docker/pulls/alcroito/digitalocean-dyndns?color=ffb64c&label=pulls&logo=docker&logoColor=white&labelColor=757575)](https://hub.docker.com/r/alcroito/digitalocean-dyndns)
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/alcroito/digitalocean-dyndns/ci?style=flat-square&logo=github&logoColor=white&labelColor=757575)](https://github.com/alcroito/digitalocean-dyndns/actions)
[![License](https://img.shields.io/github/license/alcroito/digitalocean-dyndns)](https://github.com/alcroito/digitalocean-dyndns/blob/master/LICENSE)

A Unix daemon that periodically updates a DigitalOcean domain record with the current machine's public IP address.

## How it works
The daemon periodically runs the following steps:

*  finds the current machine's public IPv4 by sending a DNS request to an OpenDNS resolver
*  queries the domain records using DO's API to find the configured subdomain. If the subdomain IP
   is different from the current public API, it updates the subdomain record to point to the new IP

## Setup
* A Unix (Linux / macOS) server to run the daemon
* A DigitalOcean account with your domain associated to it
* An existing `A` record for the subdomain to be updated

## Usage

The daemon can be configured via command line arguments, environment variables or a config file.

See [do_ddns.sample.toml](./do_ddns.sample.toml) for a sample configuration file.

Run `do_ddns -h` to see the available command line options as well as available
environment variables.
## Build requirements

To build the application you need a recent enough version of the Rust compiler (1.45+).
Build using `cargo build`. The executable will be placed into `$PWD/target[/target-arch]/do_ddns`.

# Docker images and docker-compose

Docker images (built using Github Actions) for the following platforms `linux/amd64`, `linux/arm64`, `linux/arm/v7`
are published to [DockerHub](https://hub.docker.com/r/alcroito/digitalocean-dyndns)
and the [Github Container registry](https://github.com/users/alcroito/packages/container/package/digitalocean-dyndns).

A sample [docker-compose.yaml](./docker/docker-compose.yaml) file to run the daemon as a docker container is provided.

## LICENSE

MIT
