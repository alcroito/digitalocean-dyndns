# Dynamic DNS using DigitalOcean's DNS API

![GitHub](https://img.shields.io/github/license/alcroito/digitalocean-dyndns)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/alcroito/digitalocean-dyndns/ci)
![Docker Pulls](https://img.shields.io/docker/pulls/alcroito/digitalocean-dyndns)


A Unix daemon that periodically updates a DigitalOcean domain record with the current machine's public IP address.

## How it works
The daemon periodically runs the following steps:

*  finds the current machine's public IPv4 by sending a DNS request to an OpenDNS resolver
*  queries the domain records using DO's API to find the configured subdomain. If the subdomain IP
   is different from the current public API, it updates the subdomain record to point to the new IP

## Setup
* A Unix server to run the daemon
* A DigitalOcean account with your domain associated to it
* An existing `A` record for the subdomain to be updated

## Usage

The daemon can be configured via command line arguments, environment variables or a config file.

Run `do_ddns -h` to see the available options.

## Build requirements

To build the application you need a recent enough version of the Rust compiler (1.45+).
Build it using `cargo build`.

# Docker images and docker-compose

Docker images for linux `x64`, `armv7` and `aarch64` are published to DockerHub.

A sample [docker-compose.yaml](./doker/docker-compose.yaml) file to run the daemon as a docker container is provided.

## LICENSE

MIT
