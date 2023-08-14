#!/bin/bash -xe
# Run with `buildah unshare`
export BUILDAH_HISTORY=1

autogen_builder=$(buildah from rust:1.71)
buildah copy "$autogen_builder" . /usr/src/rss_autogen_giscus
buildah run --workingdir /usr/src/rss_autogen_giscus "$autogen_builder" -- cargo install --path .

autogen_image=$(buildah from debian:bullseye-slim)
buildah run -e DEBIAN_FRONTEND=noninteractive "$autogen_image" -- bash -c 'apt-get -y update && apt-get -y install ca-certificates'
autogen_mount=$(buildah mount "$autogen_builder")
buildah copy "$autogen_image" "$autogen_mount/usr/local/cargo/bin/rss_autogen_giscus" /usr/local/bin/
buildah unmount "$autogen_builder"
buildah rm "$autogen_builder"

buildah config -a 'org.opencontainers.image.description=Crate to generating Giscus discussion posts from RSS feeds' \
  -a 'org.opencontainers.image.authors=Cameron Rodriguez <dev@camrod.me>' \
  -a 'org.opencontainers.image.source=https://github.com/cam-rod/rss_autogen_giscus' \
  -a 'org.opencontainers.image.license=Apache-2.0' \
  --cmd /usr/local/bin/rss_autogen_giscus "$autogen_image"
buildah commit "$autogen_image" ghcr.io/cam-rod/rss_autogen_giscus:latest
