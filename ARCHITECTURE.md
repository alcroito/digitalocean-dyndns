# Docker

Docker builds are done using the images provided by

* https://github.com/emk/rust-musl-builder (for `linux_amd64`)
* https://github.com/messense/rust-musl-cross (for `linux_armv7` and `linux_aarch64` images)

and the actual images are built in Github Actions using a combination of the
`docker/build-push-action@v2` action and some manual commands to create a multi-arch
manifest list.
